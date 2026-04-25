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
   SCHEMA / PYTHON GENERATION
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
   SERVICE STATUS
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
        let total   = services.len();
        let running = services.iter().filter(|s| s.status == BootStatus::Running).count();
        let failed  = services.iter().filter(|s| s.status == BootStatus::Failed).count();

        let all_up   = total > 0 && running == total;
        let any_fail = failed > 0;

        let phase = if total == 0 {
            ("Preparing…", Color::Yellow)
        } else if all_up {
            ("All services running ✓", Color::Green)
        } else if any_fail {
            ("Some services failed ✗", Color::Red)
        } else {
            ("Starting services…", Color::Cyan)
        };

        let ratio = if total == 0 { 0.0 } else { running as f64 / total as f64 };
        let elapsed = started_at.elapsed().as_secs();

        terminal.draw(|f| {
            let area = f.area();
            let root = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Length(3),
                    Constraint::Min(0),
                    Constraint::Length(2),
                ])
                .split(area);

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

            let bar_color = if any_fail { Color::Red } else if all_up { Color::Green } else { Color::Cyan };
            let gauge = Gauge::default()
                .block(Block::default().borders(Borders::ALL).title(" Progress "))
                .gauge_style(Style::default().fg(bar_color).bg(Color::DarkGray))
                .ratio(ratio)
                .label(format!("{}/{} running", running, total));
            f.render_widget(gauge, root[1]);

            let items: Vec<ListItem> = services
                .iter()
                .map(|svc| {
                    let (icon, color) = match svc.status {
                        BootStatus::Running  => ("✓", Color::Green),
                        BootStatus::Failed   => ("✗", Color::Red),
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
                        Span::styled(format!("{:<30}", svc.name), Style::default().fg(Color::White)),
                        Span::styled(raw_display, Style::default().fg(Color::DarkGray)),
                    ]))
                })
                .collect();

            f.render_widget(
                List::new(items).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(format!(" Services ({}/{}) ", running, total))
                        .border_style(Style::default().fg(Color::Blue)),
                ),
                root[2],
            );

            let footer = Paragraph::new(if all_up {
                Line::from(Span::styled(
                    "  All services up — switching to UI…",
                    Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
                ))
            } else {
                Line::from(Span::styled("  q  abort", Style::default().fg(Color::DarkGray)))
            })
            .block(Block::default().borders(Borders::TOP));
            f.render_widget(footer, root[3]);
        })?;

        if all_up {
            sleep(Duration::from_millis(800)).await;
            break Ok(true);
        }

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
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;
    result
}

/* ================================================================
   SHUTDOWN TUI
   ================================================================ */

fn is_stopped(raw: &str) -> bool {
    if raw.is_empty() {
        return true; // removed from docker ps entirely
    }
    let lower = raw.to_lowercase();
    lower.contains("exit") || lower.contains("dead") || lower.contains("remov")
}

async fn run_shutdown_tui(
    project_name: String,
    services: Vec<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let started_at = Instant::now();
    let mut tick: usize = 0;
    const TIMEOUT_SECS: u64 = 120;

    loop {
        let statuses = poll_service_statuses(&project_name, &services).await;

        let total   = statuses.len();
        let stopped = statuses.iter().filter(|s| is_stopped(&s.raw_status)).count();
        let elapsed = started_at.elapsed().as_secs();

        let all_done  = total == 0 || stopped == total;
        let timed_out = elapsed >= TIMEOUT_SECS;
        let finished  = all_done || timed_out;

        let ratio = if total == 0 { 1.0 } else { stopped as f64 / total as f64 };
        let phase = if finished {
            ("All services stopped ✓", Color::Green)
        } else {
            ("Stopping services…", Color::Yellow)
        };

        terminal.draw(|f| {
            let area = f.area();
            let root = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Length(3),
                    Constraint::Min(0),
                    Constraint::Length(2),
                ])
                .split(area);

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
                Span::styled(format!("  ({}s)", elapsed), Style::default().fg(Color::DarkGray)),
            ]))
            .block(Block::default().borders(Borders::ALL).border_style(Style::default().fg(Color::Yellow)));
            f.render_widget(header, root[0]);

            let gauge = Gauge::default()
                .block(Block::default().borders(Borders::ALL).title(" Shutdown progress "))
                .gauge_style(
                    Style::default()
                        .fg(if finished { Color::Green } else { Color::Yellow })
                        .bg(Color::DarkGray),
                )
                .ratio(ratio)
                .label(format!("{}/{} stopped", stopped, total));
            f.render_widget(gauge, root[1]);

            let items: Vec<ListItem> = statuses
                .iter()
                .map(|svc| {
                    let done = is_stopped(&svc.raw_status);
                    let (icon, color) = if done {
                        ("✓", Color::Green)
                    } else {
                        (spinner_frame(tick), Color::Yellow)
                    };
                    let raw_display = if svc.raw_status.is_empty() {
                        "removed".to_string()
                    } else {
                        svc.raw_status.clone()
                    };
                    ListItem::new(Line::from(vec![
                        Span::styled(
                            format!("  {} ", icon),
                            Style::default().fg(color).add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(format!("{:<30}", svc.name), Style::default().fg(Color::White)),
                        Span::styled(raw_display, Style::default().fg(Color::DarkGray)),
                    ]))
                })
                .collect();

            f.render_widget(
                List::new(items).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(format!(" Services ({}/{} stopped) ", stopped, total))
                        .border_style(Style::default().fg(Color::Blue)),
                ),
                root[2],
            );

            let footer_text = if finished {
                "  Shutdown complete — goodbye"
            } else {
                "  Waiting for containers to stop…  q  force quit"
            };
            f.render_widget(
                Paragraph::new(Span::styled(
                    footer_text,
                    Style::default().fg(if finished { Color::Green } else { Color::DarkGray }),
                ))
                .block(Block::default().borders(Borders::TOP)),
                root[3],
            );
        })?;

        if finished {
            sleep(Duration::from_millis(800)).await;
            break;
        }

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.code == KeyCode::Char('q') {
                    break;
                }
            }
        }

        tick = tick.wrapping_add(1);
        sleep(Duration::from_millis(500)).await;
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;
    Ok(())
}

/* ================================================================
   SHARED SHUTDOWN HELPER
   ================================================================ */

async fn shutdown_with_tui(project_name: String, expected_services: Vec<String>) {
    tokio::spawn(async {
        let _ = tokio::process::Command::new("docker-compose")
            .arg("down")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .await;
    });

    if let Err(e) = run_shutdown_tui(project_name, expected_services).await {
        eprintln!("Shutdown TUI error: {:?}", e);
    }
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

    // ── 2. Fetch schemas + generate Python files (only for dbs with an id) ──
    //
    // When a db has no `id`, we skip the API call and Python file generation
    // entirely. The template will render a pgweb explorer service instead of
    // the gingersociety runtime, so models.py / admin.py are not required.
    let open_api_config = get_configuration(Some(token));
    let db_compose_config = read_db_config("db-compose.toml").unwrap();

    for db in db_compose_config
        .database
        .iter()
        .filter(|db| db.db_type == DbType::Rdbms)
    {
        let schema_id = match db.id.clone() {
            Some(id) => id,
            None => {
                println!(
                    "Skipping schema fetch for '{}' (no id configured — pgweb will be used as the DB explorer).",
                    db.name
                );
                continue;
            }
        };

        println!("Processing RDBMS database: {}", db.name);

        let schemas: Vec<Schema> = match metadata_get_dbschema_by_id(
            &open_api_config,
            MetadataGetDbschemaByIdParams {
                schema_id,
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
                        eprintln!("Error parsing schema for '{}': {:?}", db.name, err);
                        return;
                    }
                }
            }
            Err(e) => {
                eprintln!("{:?}", e);
                eprintln!(
                    "Error fetching schema for '{}', please check your network.",
                    db.name
                );
                return;
            }
        };

        let schema_json_path = format!("{}/schema.json", db.name);
        match File::create(&schema_json_path) {
            Ok(mut f) => {
                if let Err(err) =
                    f.write_all(serde_json::to_string_pretty(&schemas).unwrap().as_bytes())
                {
                    eprintln!("Error writing schema.json for '{}': {:?}", db.name, err);
                }
            }
            Err(err) => eprintln!("Error creating schema.json for '{}': {:?}", db.name, err),
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
        let project_name  = project_name.clone();
        let expected      = expected_services.clone();

        tokio::spawn(async move {
            loop {
                let updated = poll_service_statuses(&project_name, &expected).await;
                *boot_services.lock().unwrap() = updated;
                sleep(Duration::from_secs(1)).await;
            }
        });
    }

    let _ = show_splash_screen().await;

    // ── 7. Run startup TUI ────────────────────────────────────────
    match run_startup_tui(project_name.clone(), boot_services).await {
        Ok(true) => {
            // All services up — hand off to the main monitoring UI.
            match render_ui().await {
                Ok(_) => {}
                Err(e) => eprintln!("UI error: {:?}", e),
            }

            // User quit the main UI — show shutdown TUI.
            shutdown_with_tui(project_name, expected_services).await;
        }
        Ok(false) => {
            // User pressed q during startup — kill compose then show shutdown TUI.
            let _ = child.kill().await;
            shutdown_with_tui(project_name, expected_services).await;
        }
        Err(e) => {
            eprintln!("Startup TUI error: {:?}", e);
            let _ = child.kill().await;
            // Best-effort silent shutdown; terminal state may be broken.
            let _ = tokio::process::Command::new("docker-compose")
                .arg("down")
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .await;
        }
    }
}


async fn show_splash_screen() -> Result<(), Box<dyn std::error::Error>> {
    use crossterm::{
        cursor::MoveTo,
        execute,
        terminal::{size, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
    };
    use std::io::{stdout, Write};

    let splash = vec![
        "      +@@@+-..:          ",
        "     =@@*%*++=-::..      ",
        "  =*%+*+++++=+==-....    ",
        " -#%@======:+=:=-:...    ",
        " -++:.#.-:-@%..+::....   ",
        " :-=@##@@@@@%+#@+.....   ",
        " ..#%  @@@@@+  %%:.+-.   ",
        " ..#%%@@%#@@@%##*..+..   ",
        " ..:%@@%#%+=@%%#-....    ",
        "  ...+%%#**%%#*.....     ",
        " %@%-:.. -:- .-.. -#@@+= ",
        "#@@=:=- ++=+= --:.=@@@%=:",
        "+%* ::.=*##*=-.... *%%#-:",
        " =+ ...:*=.*+:..... :=-: ",
        "    ....:==.:.......     ",
        "    ....      ......     ",
        "     ....    .....       ",
        "          ..             ",
        "       GingerDB          ",
        "          By             ",
        "    Ginger Society       ",
    ];

    let mut stdout = stdout();

    execute!(stdout, EnterAlternateScreen)?;
    execute!(stdout, Clear(ClearType::All))?;

    let (cols, rows) = size()?;

    let splash_height = splash.len() as u16;
    let splash_width = splash.iter().map(|l| l.len()).max().unwrap_or(0) as u16;

    let start_y = rows.saturating_sub(splash_height) / 2;
    let start_x = cols.saturating_sub(splash_width) / 2;

    for (i, line) in splash.iter().enumerate() {
        execute!(stdout, MoveTo(start_x, start_y + i as u16))?;
        print!("{}", line);
    }

    stdout.flush()?;

    // ⏳ show for 3 seconds
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;

    execute!(stdout, LeaveAlternateScreen)?;

    Ok(())
}