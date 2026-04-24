use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
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

pub async fn render_ui() -> Result<(), Box<dyn std::error::Error>> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Get Docker project name from docker-compose
    let project_name = get_compose_project_name().await?;
    
    // Store services and their logs
    let services = Arc::new(Mutex::new(Vec::<Service>::new()));
    let service_logs: Arc<Mutex<HashMap<String, Vec<String>>>> = 
        Arc::new(Mutex::new(HashMap::new()));
    let selected_idx = Arc::new(Mutex::new(0));

    // Spawn task to update services list
    let services_clone = Arc::clone(&services);
    let project_name_clone = project_name.clone();
    tokio::spawn(async move {
        loop {
            if let Ok(updated_services) = get_docker_services(&project_name_clone).await {
                let mut services_lock = services_clone.lock().unwrap();
                *services_lock = updated_services;
            }
            sleep(Duration::from_secs(2)).await;
        }
    });

    // Spawn task to collect logs for each service
    let services_clone = Arc::clone(&services);
    let logs_clone = Arc::clone(&service_logs);
    tokio::spawn(async move {
        loop {
            let services_list = services_clone.lock().unwrap().clone();
            for service in services_list {
                if !service.container_id.is_empty() {
                    let logs = logs_clone.clone();
                    let container_id = service.container_id.clone();
                    let service_name = service.name.clone();
                    
                    tokio::spawn(async move {
                        if let Ok(new_logs) = get_container_logs(&container_id).await {
                            let mut logs_lock = logs.lock().unwrap();
                            logs_lock.insert(service_name, new_logs);
                        }
                    });
                }
            }
            sleep(Duration::from_secs(1)).await;
        }
    });

    let mut auto_scroll = true;
    let mut scroll_offset: u16 = 0;

    // Event loop
    loop {
        let services_list = services.lock().unwrap().clone();
        let current_idx = *selected_idx.lock().unwrap();
        
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(30),
                    Constraint::Percentage(70),
                ])
                .split(f.size());

            // Left panel - Services
            let items: Vec<ListItem> = services_list
                .iter()
                .enumerate()
                .map(|(i, service)| {
                    let status_color = match service.status.as_str() {
                        "running" => Color::Green,
                        "exited" => Color::Red,
                        "paused" => Color::Yellow,
                        "restarting" => Color::Cyan,
                        _ => Color::Gray,
                    };
                    
                    let content = format!("● {} [{}]", service.name, service.status);
                    let mut style = Style::default().fg(status_color);
                    
                    if i == current_idx {
                        style = style.bg(Color::DarkGray).add_modifier(Modifier::BOLD);
                    }
                    
                    ListItem::new(content).style(style)
                })
                .collect();

            let title = format!("Services ({}) - ↑/↓ select, q quit", services_list.len());
            let services_widget = List::new(items)
                .block(Block::default().borders(Borders::ALL).title(title));

            f.render_widget(services_widget, chunks[0]);

            // Right panel - Logs
            if !services_list.is_empty() && current_idx < services_list.len() {
                let logs = service_logs.lock().unwrap();
                let selected_service = &services_list[current_idx].name;
                
                let log_text = logs
                    .get(selected_service)
                    .map(|lines| lines.join("\n"))
                    .unwrap_or_else(|| "Loading logs...".to_string());

                if auto_scroll {
                    let num_lines = log_text.lines().count() as u16;
                    let visible_height = chunks[1].height.saturating_sub(2);
                    scroll_offset = num_lines.saturating_sub(visible_height);
                }

                let log_widget = Paragraph::new(log_text)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title(format!("Logs: {} (PgUp/PgDn/End)", selected_service)),
                    )
                    .wrap(Wrap { trim: false })
                    .scroll((scroll_offset, 0));

                f.render_widget(log_widget, chunks[1]);
            } else {
                let empty = Paragraph::new("No services found")
                    .block(Block::default().borders(Borders::ALL).title("Logs"));
                f.render_widget(empty, chunks[1]);
            }
        })?;

        // Handle input
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Down => {
                        let services_len = services.lock().unwrap().len();
                        if services_len > 0 {
                            let mut idx = selected_idx.lock().unwrap();
                            *idx = (*idx + 1).min(services_len - 1);
                            auto_scroll = true;
                            scroll_offset = 0;
                        }
                    }
                    KeyCode::Up => {
                        let mut idx = selected_idx.lock().unwrap();
                        *idx = idx.saturating_sub(1);
                        auto_scroll = true;
                        scroll_offset = 0;
                    }
                    KeyCode::PageDown => {
                        auto_scroll = false;
                        scroll_offset = scroll_offset.saturating_add(10);
                    }
                    KeyCode::PageUp => {
                        auto_scroll = false;
                        scroll_offset = scroll_offset.saturating_sub(10);
                    }
                    KeyCode::End => {
                        auto_scroll = true;
                    }
                    _ => {}
                }
            }
        }
    }

    // Cleanup
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