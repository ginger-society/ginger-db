use crossterm::{
    cursor::MoveTo,
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{
        disable_raw_mode, enable_raw_mode, Clear, ClearType, EnterAlternateScreen,
        LeaveAlternateScreen,
    },
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Terminal,
};
use std::{
    collections::HashMap,
    io,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::time::sleep;

#[derive(Clone, Debug)]
struct Service {
    name: String,
    container_id: String,
    status: String,
}

#[derive(PartialEq)]
enum Focus {
    Services,
    Logs,
}

/* ---------------- HELP ---------------- */

fn help_text(focus: &Focus) -> String {
    match focus {
        Focus::Services => "↑/↓ select | ←/→ switch | s shell | q quit",
        Focus::Logs => "PgUp start | PgDn follow | ↑/↓ scroll | j/k scroll | g/G jump | ←/→ switch | s shell",
    }
    .to_string()
}

/* ---------------- MAIN ---------------- */

pub async fn render_ui() -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let project_name = get_compose_project_name().await?;

    let services = Arc::new(Mutex::new(Vec::<Service>::new()));
    let service_logs: Arc<Mutex<HashMap<String, Vec<String>>>> =
        Arc::new(Mutex::new(HashMap::new()));
    let selected_idx = Arc::new(Mutex::new(0));

    /* -------- SERVICES -------- */
    {
        let services = services.clone();
        let project_name = project_name.clone();

        tokio::spawn(async move {
            loop {
                if let Ok(updated) = get_docker_services(&project_name).await {
                    *services.lock().unwrap() = updated;
                }
                sleep(Duration::from_secs(2)).await;
            }
        });
    }

    /* -------- LOGS -------- */
    {
        let services = services.clone();
        let logs = service_logs.clone();

        tokio::spawn(async move {
            loop {
                let list = services.lock().unwrap().clone();

                for svc in list {
                    if svc.container_id.is_empty() {
                        continue;
                    }

                    if let Ok(new_logs) = get_container_logs(&svc.container_id).await {
                        logs.lock().unwrap().insert(svc.name.clone(), new_logs);
                    }
                }

                sleep(Duration::from_secs(1)).await;
            }
        });
    }

    let mut focus = Focus::Services;
    let mut auto_scroll = true;
    let mut scroll_offset: u16 = 0;

    /* ================= LOOP ================= */

    loop {
        let services_list = services.lock().unwrap().clone();
        let current_idx = *selected_idx.lock().unwrap();

        terminal.draw(|f| {
            let root = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(0),
                    Constraint::Length(2),
                ])
                .split(f.size());

            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(30),
                    Constraint::Percentage(70),
                ])
                .split(root[0]);

            /* -------- SERVICES -------- */

            let items: Vec<ListItem> = services_list
                .iter()
                .enumerate()
                .map(|(i, s)| {
                    let mut style = Style::default().fg(Color::Gray);

                    if i == current_idx {
                        style = style.bg(Color::DarkGray).add_modifier(Modifier::BOLD);
                    }

                    ListItem::new(format!("● {} [{}]", s.name, s.status)).style(style)
                })
                .collect();

            let services_block = Block::default()
                .borders(Borders::ALL)
                .title(format!(
                    "Services {}",
                    if focus == Focus::Services { "[ACTIVE]" } else { "" }
                ))
                .border_style(if focus == Focus::Services {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default()
                });

            f.render_widget(List::new(items).block(services_block), chunks[0]);

            /* -------- LOGS -------- */

            let logs_block = Block::default()
                .borders(Borders::ALL)
                .title(format!(
                    "Logs {} {}",
                    if focus == Focus::Logs { "[ACTIVE]" } else { "" },
                    if auto_scroll { "[FOLLOW]" } else { "[PAUSED]" }
                ))
                .border_style(if focus == Focus::Logs {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default()
                });

            if !services_list.is_empty() && current_idx < services_list.len() {
                let logs = service_logs.lock().unwrap();
                let selected = &services_list[current_idx].name;

                let log_text = logs
                    .get(selected)
                    .map(|l| l.join("\n"))
                    .unwrap_or_else(|| "Loading logs...".to_string());

                let num_lines = log_text.lines().count() as u16;
                let height = chunks[1].height.saturating_sub(2);
                let max_scroll = num_lines.saturating_sub(height);

                if auto_scroll {
                    scroll_offset = max_scroll;
                } else {
                    scroll_offset = scroll_offset.min(max_scroll);

                    if scroll_offset >= max_scroll {
                        auto_scroll = true;
                    }
                }

                f.render_widget(
                    Paragraph::new(log_text)
                        .block(logs_block)
                        .wrap(Wrap { trim: false })
                        .scroll((scroll_offset, 0)),
                    chunks[1],
                );
            } else {
                f.render_widget(
                    Paragraph::new("No services").block(logs_block),
                    chunks[1],
                );
            }

            /* -------- HELP BAR -------- */

            let help = Paragraph::new(help_text(&focus))
                .style(Style::default().fg(Color::DarkGray))
                .block(Block::default().borders(Borders::TOP));

            f.render_widget(help, root[1]);
        })?;

        /* ================= INPUT ================= */

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => break,

                    KeyCode::Left => focus = Focus::Services,
                    KeyCode::Right => focus = Focus::Logs,

                    KeyCode::Char('s') => {
                        let list = services.lock().unwrap().clone();
                        let idx = *selected_idx.lock().unwrap();

                        if let Some(service) = list.get(idx) {
                            let container_id = service.container_id.clone();

                            if !container_id.is_empty() {
                                /* -------- CLEAN EXIT -------- */
                                disable_raw_mode()?;
                                execute!(
                                    terminal.backend_mut(),
                                    LeaveAlternateScreen,
                                    Clear(ClearType::All),
                                    MoveTo(0, 0)
                                )?;
                                terminal.show_cursor()?;

                                // HARD RESET (fixes "starts from bottom")
                                println!("\x1b[2J\x1b[H");
                                println!("\nEntering shell: {}\n", container_id);

                                /* -------- SHELL -------- */
                                let _ = open_shell(&container_id).await;

                                /* -------- RESTORE -------- */
                                enable_raw_mode()?;
                                execute!(
                                    terminal.backend_mut(),
                                    EnterAlternateScreen,
                                    Clear(ClearType::All),
                                    MoveTo(0, 0)
                                )?;
                                terminal.hide_cursor()?;

                                terminal.clear()?;
                                terminal.draw(|_| {})?;
                            }
                        }
                    }

                    _ => {
                        /* -------- SERVICES -------- */
                        if focus == Focus::Services {
                            match key.code {
                                KeyCode::Down => {
                                    let len = services.lock().unwrap().len();
                                    if len > 0 {
                                        let mut idx = selected_idx.lock().unwrap();
                                        *idx = (*idx + 1).min(len - 1);
                                    }
                                }
                                KeyCode::Up => {
                                    let mut idx = selected_idx.lock().unwrap();
                                    *idx = idx.saturating_sub(1);
                                }
                                _ => {}
                            }
                        }

                        /* -------- LOGS -------- */
                        if focus == Focus::Logs {
                            match key.code {
                                KeyCode::PageUp => {
                                    auto_scroll = false;
                                    scroll_offset = 0;
                                }
                                KeyCode::PageDown => {
                                    auto_scroll = true;
                                }
                                KeyCode::Down | KeyCode::Char('j') => {
                                    auto_scroll = false;
                                    scroll_offset += 1;
                                }
                                KeyCode::Up | KeyCode::Char('k') => {
                                    auto_scroll = false;
                                    scroll_offset = scroll_offset.saturating_sub(1);
                                }
                                KeyCode::Char('g') => {
                                    auto_scroll = false;
                                    scroll_offset = 0;
                                }
                                KeyCode::Char('G') => {
                                    auto_scroll = true;
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        }
    }

    /* -------- CLEANUP -------- */

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}

async fn get_compose_project_name() -> Result<String, Box<dyn std::error::Error>> {
    let output = tokio::process::Command::new("docker-compose")
        .arg("config")
        .output()
        .await?;
    
    let config = String::from_utf8(output.stdout)?;
    
    // Try to extract project name from config, or use directory name
    for line in config.lines() {
        if line.starts_with("name:") {
            return Ok(line.split(':').nth(1).unwrap_or("").trim().to_string());
        }
    }
    
    // Fallback: use current directory name
    std::env::current_dir()?
        .file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.to_string())
        .ok_or_else(|| "Could not determine project name".into())
}

async fn get_docker_services(project_name: &str) -> Result<Vec<Service>, Box<dyn std::error::Error>> {
    // List containers with the project label
    let output = tokio::process::Command::new("docker")
        .args(&[
            "ps",
            "-a",
            "--filter",
            &format!("label=com.docker.compose.project={}", project_name),
            "--format",
            "{{.ID}}|{{.Label \"com.docker.compose.service\"}}|{{.Status}}",
        ])
        .output()
        .await?;

    let result = String::from_utf8(output.stdout)?;
    let services: Vec<Service> = result
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| {
            let parts: Vec<&str> = line.split('|').collect();
            Service {
                container_id: parts.get(0).unwrap_or(&"").to_string(),
                name: parts.get(1).unwrap_or(&"unknown").to_string(),
                status: parse_status(parts.get(2).unwrap_or(&"unknown")),
            }
        })
        .collect();

    Ok(services)
}

fn parse_status(status_str: &str) -> String {
    let lower = status_str.to_lowercase();
    if lower.contains("up") {
        "running".to_string()
    } else if lower.contains("exited") {
        "exited".to_string()
    } else if lower.contains("paused") {
        "paused".to_string()
    } else if lower.contains("restarting") {
        "restarting".to_string()
    } else {
        "unknown".to_string()
    }
}

async fn get_container_logs(container_id: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let output = tokio::process::Command::new("docker")
        .args(&[
            "logs",
            "--tail",
            "500",  // Get last 500 lines
            container_id,
        ])
        .output()
        .await?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    
    let mut lines: Vec<String> = stdout
        .lines()
        .chain(stderr.lines())
        .map(|s| s.to_string())
        .collect();
    
    // Keep only last 500 lines
    if lines.len() > 500 {
        lines.drain(0..lines.len() - 500);
    }
    
    Ok(lines)
}

async fn open_shell(container_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    use tokio::process::Command;
    use std::process::Stdio;

    // Try bash first, fallback to sh
    let status = Command::new("docker")
        .args(["exec", "-it", container_id, "bash"])
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .await;

    if let Ok(s) = status {
        if s.success() {
            return Ok(());
        }
    }

    // fallback to sh
    Command::new("docker")
        .args(["exec", "-it", container_id, "sh"])
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .await?;

    Ok(())
}