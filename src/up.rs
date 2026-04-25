use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::exit;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{
        disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
    },
};
use ginger_shared_rs::{read_db_config, DbType};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph},
    Terminal,
};
use serde_json::Value;
use tera::{Context, Tera};
use tokio::time::sleep;
use MetadataService::apis::default_api::{
    metadata_get_dbschema_by_id, MetadataGetDbschemaByIdParams,
};
use MetadataService::get_configuration;

use crate::types::{Schema, SchemaType};
use crate::ui::render_ui;

/* ================================================================
   SCHEMA / PYTHON GENERATION  (unchanged from original)
   ================================================================ */

pub fn generate_python_files_for_db(db_name: &str, schemas: &[Schema], tera: &Tera) {
    let mut sorted_schemas = schemas.to_vec();
    sorted_schemas.sort_by(|a, _b| {
        if a.schema_type == SchemaType::Enum {
            std::cmp::Ordering::Less
        } else {
            std::cmp::Ordering::Greater
        }
    });

    let mut context = Context::new();
    context.insert("schemas", &sorted_schemas);

    let models_path = format!("{}/models.py", db_name);
    match tera.render("models.py.tpl", &context) {
        Ok(rendered_template) => {
            if let Err(err) = fs::write(&models_path, rendered_template) {
                eprintln!("Error writing to models.py: {:?}", err);
            }
        }
        Err(e) => eprintln!("Error rendering models.py template: {:?}", e),
    }

    let admin_path = format!("{}/admin.py", db_name);
    if !Path::new(&admin_path).exists() {
        match tera.render("admin.py.tpl", &context) {
            Ok(rendered_template) => {
                if let Err(err) = fs::write(&admin_path, rendered_template) {
                    eprintln!("Error writing to admin.py: {:?}", err);
                }
            }
            Err(e) => eprintln!("Error rendering admin.py template: {:?}", e),
        }
    } else {
        println!("admin.py already exists, skipping creation.");
    }
}

/* ================================================================
   SERVICE STATUS (used by startup TUI)
   ================================================================ */

#[derive(Clone, Debug, PartialEq)]
enum BootStatus {
    Waiting,
    Starting,
    Running,
    Failed,
}

#[derive(Clone, Debug)]
struct BootService {
    name: String,
    status: BootStatus,
    /// raw docker status string e.g. "Up 3 seconds"
    raw_status: String,
}

/* ================================================================
   DOCKER HELPERS
   ================================================================ */

async fn get_compose_project_name() -> Result<String, Box<dyn std::error::Error>> {
    let output = tokio::process::Command::new("docker-compose")
        .arg("config")
        .output()
        .await?;

    let config = String::from_utf8(output.stdout)?;
    for line in config.lines() {
        if line.starts_with("name:") {
            return Ok(line.split(':').nth(1).unwrap_or("").trim().to_string());
        }
    }

    std::env::current_dir()?
        .file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.to_string())
        .ok_or_else(|| "Could not determine project name".into())
}

async fn poll_service_statuses(
    project_name: &str,
    expected: &[String],
) -> Vec<BootService> {
    let output = tokio::process::Command::new("docker")
        .args(&[
            "ps",
            "-a",
            "--filter",
            &format!("label=com.docker.compose.project={}", project_name),
            "--format",
            "{{.Label \"com.docker.compose.service\"}}|{{.Status}}",
        ])
        .output()
        .await;

    let mut map: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();

    if let Ok(out) = output {
        let text = String::from_utf8_lossy(&out.stdout);
        for line in text.lines().filter(|l| !l.is_empty()) {
            let parts: Vec<&str> = line.splitn(2, '|').collect();
            if parts.len() == 2 {
                map.insert(parts[0].to_string(), parts[1].to_string());
            }
        }
    }

    expected
        .iter()
        .map(|name| {
            let raw = map.get(name).cloned().unwrap_or_default();
            let lower = raw.to_lowercase();
            let status = if lower.contains("up") {
                BootStatus::Running
            } else if lower.contains("exit") || lower.contains("dead") {
                BootStatus::Failed
            } else if lower.is_empty() {
                BootStatus::Waiting
            } else {
                BootStatus::Starting
            };
            BootService {
                name: name.clone(),
                status,
                raw_status: raw,
            }
        })
        .collect()
}

/// Pull the list of service names declared in docker-compose.yml
async fn get_declared_services() -> Vec<String> {
    let output = tokio::process::Command::new("docker-compose")
        .args(&["config", "--services"])
        .output()
        .await;

    match output {
        Ok(out) => String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter(|l| !l.is_empty())
            .map(|l| l.to_string())
            .collect(),
        Err(_) => vec![],
    }
}

/* ================================================================
   SPINNER
   ================================================================ */

const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

fn spinner_frame(tick: usize) -> &'static str {
    SPINNER_FRAMES[tick % SPINNER_FRAMES.len()]
}

/* ================================================================
   STARTUP TUI
   ================================================================ */

/// Shows a boot-progress TUI until all services are running (or user quits).
/// Returns Ok(true) if all services came up, Ok(false) if user pressed q.
async fn run_startup_tui(
    project_name: String,
    services_snapshot: Arc<Mutex<Vec<BootService>>>,
) -> Result<bool, Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let started_at = Instant::now();
    let mut tick: usize = 0;

    let result = loop {
        let services = services_snapshot.lock().unwrap().clone();
        let total = services.len();
        let running = services.iter().filter(|s| s.status == BootStatus::Running).count();
        let failed  = services.iter().filter(|s| s.status == BootStatus::Failed).count();

        let all_up   = total > 0 && running == total;
        let any_fail = failed > 0;

        // Phase label
        let phase = if total == 0 {
            ("Preparing…", Color::Yellow)
        } else if all_up {
            ("All services running ✓", Color::Green)
        } else if any_fail {
            ("Some services failed ✗", Color::Red)
        } else {
            ("Starting services…", Color::Cyan)
        };

        // Progress ratio (0.0–1.0)
        let ratio = if total == 0 {
            0.0
        } else {
            running as f64 / total as f64
        };

        let elapsed = started_at.elapsed().as_secs();

        terminal.draw(|f| {
            let area = f.area();

            let root = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),  // header
                    Constraint::Length(3),  // progress bar
                    Constraint::Min(0),     // service list
                    Constraint::Length(2),  // footer
                ])
                .split(area);

            /* ── Header ── */
            let header = Paragraph::new(Line::from(vec![
                Span::styled(
                    " ginger-db ",
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw("  "),
                Span::styled(phase.0, Style::default().fg(phase.1).add_modifier(Modifier::BOLD)),
                Span::styled(
                    format!("  ({}s)", elapsed),
                    Style::default().fg(Color::DarkGray),
                ),
            ]))
            .block(Block::default().borders(Borders::ALL).border_style(
                Style::default().fg(Color::Yellow),
            ));
            f.render_widget(header, root[0]);

            /* ── Progress bar ── */
            let bar_color = if any_fail {
                Color::Red
            } else if all_up {
                Color::Green
            } else {
                Color::Cyan
            };

            let gauge = Gauge::default()
                .block(Block::default().borders(Borders::ALL).title(" Progress "))
                .gauge_style(Style::default().fg(bar_color).bg(Color::DarkGray))
                .ratio(ratio)
                .label(format!("{}/{} running", running, total));

            f.render_widget(gauge, root[1]);

            /* ── Service list ── */
            let items: Vec<ListItem> = services
                .iter()
                .map(|svc| {
                    let (icon, color) = match svc.status {
                        BootStatus::Running => ("✓", Color::Green),
                        BootStatus::Failed  => ("✗", Color::Red),
                        BootStatus::Starting => (spinner_frame(tick), Color::Cyan),
                        BootStatus::Waiting  => ("○", Color::DarkGray),
                    };

                    let raw_display = if svc.raw_status.is_empty() {
                        "waiting for docker…".to_string()
                    } else {
                        svc.raw_status.clone()
                    };

                    ListItem::new(Line::from(vec![
                        Span::styled(
                            format!("  {} ", icon),
                            Style::default().fg(color).add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            format!("{:<30}", svc.name),
                            Style::default().fg(Color::White),
                        ),
                        Span::styled(raw_display, Style::default().fg(Color::DarkGray)),
                    ]))
                })
                .collect();

            let list_title = format!(
                " Services ({}/{}) ",
                running, total
            );
            f.render_widget(
                List::new(items).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(list_title)
                        .border_style(Style::default().fg(Color::Blue)),
                ),
                root[2],
            );

            /* ── Footer ── */
            let footer = Paragraph::new(if all_up {
                Line::from(Span::styled(
                    "  All services up — switching to UI…",
                    Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
                ))
            } else {
                Line::from(Span::styled(
                    "  q  abort",
                    Style::default().fg(Color::DarkGray),
                ))
            })
            .block(Block::default().borders(Borders::TOP));

            f.render_widget(footer, root[3]);
        })?;

        // Auto-advance to UI once everything is up
        if all_up {
            sleep(Duration::from_millis(800)).await; // brief pause so user sees the green state
            break Ok(true);
        }

        // Input (non-blocking)
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.code == KeyCode::Char('q') {
                    break Ok(false);
                }
            }
        }

        tick = tick.wrapping_add(1);
        sleep(Duration::from_millis(150)).await;
    };

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    result
}

/* ================================================================
   UP  (entry point called from main)
   ================================================================ */

pub async fn up(tera: Tera, skip: bool) {
    // ── 1. Auth ────────────────────────────────────────────────────
    let home_dir = match dirs::home_dir() {
        Some(path) => path,
        None => {
            println!("Failed to locate home directory. Exiting.");
            exit(1);
        }
    };

    let auth_file_path: PathBuf = [home_dir.to_str().unwrap(), ".ginger-society", "auth.json"]
        .iter()
        .collect();

    let mut file = match File::open(&auth_file_path) {
        Ok(f) => f,
        Err(_) => {
            println!("Failed to open {}. Exiting.", auth_file_path.display());
            exit(1);
        }
    };
    let mut contents = String::new();
    if file.read_to_string(&mut contents).is_err() {
        println!("Failed to read the auth.json file. Exiting.");
        exit(1);
    }

    let json: Value = match serde_json::from_str(&contents) {
        Ok(v) => v,
        Err(_) => {
            println!("Failed to parse auth.json as JSON. Exiting.");
            exit(1);
        }
    };

    let token = match json.get("API_TOKEN").and_then(|v| v.as_str()) {
        Some(t) => t.to_string(),
        None => {
            println!("API_TOKEN not found in auth.json. Exiting.");
            exit(1);
        }
    };

    // ── 2. Fetch schemas + generate Python files ──────────────────
    let open_api_config = get_configuration(Some(token));
    let db_compose_config = read_db_config("db-compose.toml").unwrap();

    for db in db_compose_config
        .database
        .iter()
        .filter(|db| db.db_type == DbType::Rdbms)
    {
        println!("Processing RDBMS database: {}", db.name);

        let schemas: Vec<Schema> = match metadata_get_dbschema_by_id(
            &open_api_config,
            MetadataGetDbschemaByIdParams {
                schema_id: db.clone().id.unwrap(),
                branch: Some(db_compose_config.branch.to_string()),
            },
        )
        .await
        {
            Ok(response) => {
                println!("{:?}", response);
                match serde_json::from_str(&response.data.unwrap().unwrap()) {
                    Ok(s) => s,
                    Err(err) => {
                        eprintln!("Error parsing schema: {:?}", err);
                        return;
                    }
                }
            }
            Err(e) => {
                eprintln!("{:?}", e);
                eprintln!("Error getting the schema, please check your network");
                return;
            }
        };

        let schema_json_path = format!("{}/schema.json", db.clone().name);
        match File::create(&schema_json_path) {
            Ok(mut f) => {
                if let Err(err) =
                    f.write_all(serde_json::to_string_pretty(&schemas).unwrap().as_bytes())
                {
                    eprintln!("Error writing schema.json: {:?}", err);
                }
            }
            Err(err) => eprintln!("Error creating schema.json: {:?}", err),
        }

        generate_python_files_for_db(&db.name, &schemas, &tera);
        println!("Finished processing RDBMS database: {}", db.name);
    }

    // ── 3. Render docker-compose.yml ──────────────────────────────
    let mut tera_context = Context::new();
    tera_context.insert("databases", &db_compose_config.database);

    let rendered = match tera.render("docker-compose.yml.tpl", &tera_context) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Template error: {:?}", e);
            return;
        }
    };

    println!("docker-compose.yml rendered");

    if skip {
        print!("{}", rendered);
        return;
    }

    let mut output_file = match File::create("docker-compose.yml") {
        Ok(f) => f,
        Err(err) => {
            eprintln!("Error creating docker-compose.yml: {:?}", err);
            return;
        }
    };
    if let Err(err) = output_file.write_all(rendered.as_bytes()) {
        eprintln!("Error writing docker-compose.yml: {:?}", err);
        return;
    }

    // ── 4. Start docker-compose in the background ─────────────────
    println!("Starting services…");

    let mut child = match tokio::process::Command::new("docker-compose")
        .arg("up")
        .arg("--remove-orphans")
        // Detach stdout/stderr so they don't pollute the TUI
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to spawn docker-compose: {:?}", e);
            return;
        }
    };

    // ── 5. Resolve project name + expected service names ──────────
    // Give docker-compose a moment to register containers
    sleep(Duration::from_millis(800)).await;

    let project_name = get_compose_project_name().await.unwrap_or_else(|_| {
        std::env::current_dir()
            .ok()
            .and_then(|p| p.file_name().and_then(|n| n.to_str()).map(|s| s.to_string()))
            .unwrap_or_else(|| "project".to_string())
    });

    let expected_services = get_declared_services().await;

    // ── 6. Shared state polled in background ──────────────────────
    let boot_services: Arc<Mutex<Vec<BootService>>> = Arc::new(Mutex::new(
        expected_services
            .iter()
            .map(|name| BootService {
                name: name.clone(),
                status: BootStatus::Waiting,
                raw_status: String::new(),
            })
            .collect(),
    ));

    {
        let boot_services = boot_services.clone();
        let project_name = project_name.clone();
        let expected = expected_services.clone();

        tokio::spawn(async move {
            loop {
                let updated = poll_service_statuses(&project_name, &expected).await;
                *boot_services.lock().unwrap() = updated;
                sleep(Duration::from_secs(1)).await;
            }
        });
    }

    // ── 7. Run startup TUI ────────────────────────────────────────
    match run_startup_tui(project_name, boot_services).await {
        Ok(true) => {
            // All services up — hand off to the main monitoring UI
            
            match render_ui().await {
                Ok(_) => {}
                Err(e) => eprintln!("UI error: {:?}", e),
            }

            // When the user quits the UI, also stop docker-compose
            println!("Stopping services…");
            let _ = tokio::process::Command::new("docker-compose")
                .arg("down")
                .status()
                .await;
        }
        Ok(false) => {
            // User pressed q during startup — kill compose
            println!("Aborted. Stopping services…");
            let _ = child.kill().await;
            let _ = tokio::process::Command::new("docker-compose")
                .arg("down")
                .status()
                .await;
        }
        Err(e) => {
            eprintln!("Startup TUI error: {:?}", e);
            let _ = child.kill().await;
        }
    }
}