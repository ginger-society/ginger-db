use crossterm::{
    cursor::MoveTo,
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, MouseButton,
        MouseEventKind,
    },
    execute,
    terminal::{
        disable_raw_mode, enable_raw_mode, Clear, ClearType, EnterAlternateScreen,
        LeaveAlternateScreen,
    },
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Clear as RatatuiClear, List, ListItem, Paragraph, Scrollbar,
        ScrollbarOrientation, ScrollbarState, Wrap,
    },
    Terminal,
};
use std::{
    collections::HashMap,
    io::{self, Write},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio::time::sleep;

use ginger_shared_rs::{read_db_config, DatabaseConfig, DbType};

/* ================================================================
   DATA TYPES
   ================================================================ */

#[derive(Clone, Debug)]
struct Service {
    name: String,
    container_id: String,
    status: String,
    image: String,
}

#[derive(PartialEq, Clone)]
enum Focus {
    Services,
    Logs,
}

#[derive(PartialEq)]
enum PopupAction {
    Start,
    Stop,
    Quit,
}

struct Popup {
    service_name: String,
    action: PopupAction,
    /// 0 = "Yes" highlighted, 1 = "No" highlighted
    selected: usize,
}

/// A brief auto-dismissing notification shown after clipboard copy.
struct Toast {
    message: String,
    expires_at: Instant,
}

impl Toast {
    fn new(msg: impl Into<String>, duration: Duration) -> Self {
        Toast {
            message: msg.into(),
            expires_at: Instant::now() + duration,
        }
    }
    fn is_expired(&self) -> bool {
        Instant::now() >= self.expires_at
    }
}

struct DevIdentity {
    username: &'static str,
    password: &'static str,
}

/* ================================================================
   IDENTITIES
   ================================================================ */

fn dev_identity(db_type: &DbType) -> Option<DevIdentity> {
    match db_type {
        DbType::Rdbms => Some(DevIdentity {
            username: "postgres",
            password: "postgres",
        }),
        DbType::DocumentDb => Some(DevIdentity {
            username: "mongo",
            password: "mongo",
        }),
        DbType::MessageQueue => Some(DevIdentity {
            username: "user",
            password: "password",
        }),
        DbType::Cache => None,
    }
}

/* ================================================================
   CONNECTION STRING  (single source of truth — used for display & clipboard)
   ================================================================ */

fn get_connection_string(service: &Service, db: &DatabaseConfig) -> Option<String> {
    if is_db_service(&service.name, db) {
        let conn = match db.db_type {
            DbType::Rdbms => {
                if let Some(id) = dev_identity(&db.db_type) {
                    format!(
                        "postgresql://{}:{}@localhost:{}/{}",
                        id.username, id.password, db.port, service.name
                    )
                } else {
                    format!("postgresql://localhost:{}/{}", db.port, service.name)
                }
            }
            DbType::DocumentDb => {
                if let Some(id) = dev_identity(&db.db_type) {
                    format!(
                        "mongodb://{}:{}@localhost:{}",
                        id.username, id.password, db.port
                    )
                } else {
                    format!("mongodb://localhost:{}", db.port)
                }
            }
            DbType::Cache => format!("redis://localhost:{}", db.port),
            DbType::MessageQueue => {
                if let Some(id) = dev_identity(&db.db_type) {
                    format!(
                        "amqp://{}:{}@localhost:{}",
                        id.username, id.password, db.port
                    )
                } else {
                    format!("amqp://localhost:{}", db.port)
                }
            }
        };
        Some(conn)
    } else if db.db_type == DbType::MessageQueue && is_ui_service(&service.name, db) {
        // RabbitMQ combined UI+service node — also has a usable connection string
        if let Some(id) = dev_identity(&db.db_type) {
            Some(format!(
                "amqp://{}:{}@localhost:{}",
                id.username, id.password, db.port
            ))
        } else {
            Some(format!("amqp://localhost:{}", db.port))
        }
    } else {
        None
    }
}

/* ================================================================
   SERVICE INFO PANEL LINES
   ================================================================ */

fn build_service_panel(service: &Service, db: Option<&DatabaseConfig>) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = vec![];

    let Some(db) = db else {
        lines.push(Line::from("No DB config found"));
        return lines;
    };

    let is_ui = is_ui_service(&service.name, db);
    let is_db = is_db_service(&service.name, db);
    let conn_str = get_connection_string(service, db);

    // ── UI link ──────────────────────────────────────────────────
    if is_ui {
        if let Some(port) = &db.studio_port {
            lines.push(Line::from(vec![
                Span::styled("🌐 UI:  ", Style::default().fg(Color::Cyan)),
                Span::styled(
                    format!("http://localhost:{}", port),
                    Style::default().fg(Color::Yellow),
                ),
                Span::styled(
                    "  [o] Open",
                    Style::default()
                        .fg(Color::DarkGray)
                        .add_modifier(Modifier::ITALIC),
                ),
            ]));
        }
    }

    // ── Connection String ─────────────────────────────────────────
    if let Some(ref conn) = conn_str {
        lines.push(Line::from(vec![
            Span::styled("🔌 Connection String: ", Style::default().fg(Color::Cyan)),
            Span::styled(conn.clone(), Style::default().fg(Color::Yellow)),
            Span::styled(
                "  [c] copy",
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::ITALIC),
            ),
        ]));
    }

    // ── Credentials ───────────────────────────────────────────────
    let show_creds = is_db || (db.db_type == DbType::MessageQueue && is_ui);
    let skip_creds_rdbms_ui = db.db_type == DbType::Rdbms && is_ui;

    if show_creds && !skip_creds_rdbms_ui {
        if let Some(id) = dev_identity(&db.db_type) {
            lines.push(Line::from(vec![
                Span::styled("👤 user: ", Style::default().fg(Color::Gray)),
                Span::raw(id.username),
            ]));
            lines.push(Line::from(vec![
                Span::styled("🔑 pass: ", Style::default().fg(Color::Gray)),
                Span::raw(id.password),
            ]));
        }
    } else if skip_creds_rdbms_ui {
        lines.push(Line::from("🔓 no auth required"));
    }

    if lines.is_empty() {
        lines.push(Line::from("No connection info available"));
    }

    lines
}

/* ================================================================
   CLIPBOARD
   ================================================================ */

fn copy_to_clipboard(text: &str) -> Result<(), String> {
    use arboard::Clipboard;
    let mut cb = Clipboard::new().map_err(|e| format!("Clipboard init failed: {e}"))?;
    cb.set_text(text).map_err(|e| format!("Clipboard write failed: {e}"))
}

/* ================================================================
   HELP TEXT
   ================================================================ */

fn help_text(focus: &Focus, show_open: bool, has_conn: bool) -> String {
    match focus {
        Focus::Services => {
            let mut parts = vec!["↑/↓ select", "←/→ switch", "Enter start/stop", "s shell"];
            if show_open {
                parts.push("o open UI");
            }
            if has_conn {
                parts.push("c copy conn");
            }
            parts.push("q quit");
            parts.join(" | ")
        }
        Focus::Logs => {
            "PgUp start | PgDn follow | ↑/↓ scroll | j/k scroll | g/G jump | ←/→ switch"
                .to_string()
        }
    }
}

/* ================================================================
   ICONS / HELPERS
   ================================================================ */

fn db_icon(db_type: &DbType) -> &'static str {
    match db_type {
        DbType::Rdbms => "🗄️",
        DbType::Cache => "⚡",
        DbType::MessageQueue => "📬",
        DbType::DocumentDb => "📄",
    }
}

fn find_db<'a>(
    service_name: &str,
    db_map: &'a HashMap<String, DatabaseConfig>,
) -> Option<&'a DatabaseConfig> {
    let name = service_name.to_lowercase();
    db_map.iter().find_map(|(key, db)| {
        if name.starts_with(&(key.clone() + "-")) {
            Some(db)
        } else {
            None
        }
    })
}

fn is_ui_service(service_name: &str, db: &DatabaseConfig) -> bool {
    let name = service_name.to_lowercase();
    let base = db.name.to_lowercase();
    match db.db_type {
        DbType::Rdbms => name == format!("{}-runtime", base),
        DbType::DocumentDb => name == format!("{}-mongo-gui", base),
        DbType::MessageQueue => name == format!("{}-messagequeue", base),
        DbType::Cache => false,
    }
}

fn is_db_service(service_name: &str, db: &DatabaseConfig) -> bool {
    let name = service_name.to_lowercase();
    let base = db.name.to_lowercase();
    match db.db_type {
        DbType::Rdbms => name == format!("{}-db", base),
        DbType::DocumentDb => name == format!("{}-mongodb", base),
        DbType::MessageQueue => false,
        DbType::Cache => name == format!("{}-redis", base),
    }
}

fn status_color(status: &str) -> Color {
    match status {
        "running" => Color::Green,
        "exited" => Color::Red,
        "restarting" => Color::Yellow,
        "paused" => Color::Magenta,
        _ => Color::DarkGray,
    }
}

/* ================================================================
   POPUP — start/stop/quit confirm
   ================================================================ */

fn centered_rect(percent_x: u16, height: u16, r: Rect) -> Rect {
    let popup_width = r.width * percent_x / 100;
    let x = r.x + (r.width.saturating_sub(popup_width)) / 2;
    let y = r.y + (r.height.saturating_sub(height)) / 2;
    Rect {
        x,
        y,
        width: popup_width.min(r.width),
        height: height.min(r.height),
    }
}

fn render_popup(f: &mut ratatui::Frame, popup: &Popup, area: Rect) {
    let popup_area = centered_rect(50, 7, area);
    f.render_widget(RatatuiClear, popup_area);

    let (action_label, action_color) = match popup.action {
        PopupAction::Start => ("start", Color::Green),
        PopupAction::Stop => ("stop", Color::Red),
        PopupAction::Quit => ("quit", Color::Red),
    };

    let yes_style = if popup.selected == 0 {
        Style::default()
            .fg(Color::Black)
            .bg(action_color)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let no_style = if popup.selected == 1 {
        Style::default()
            .fg(Color::Black)
            .bg(Color::DarkGray)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let body = if popup.action == PopupAction::Quit {
        vec![
            Line::from(""),
            Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    "Are you sure you want to quit?",
                    Style::default().fg(Color::White),
                ),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::raw("        "),
                Span::styled("  Yes  ", yes_style),
                Span::raw("   "),
                Span::styled("  No  ", no_style),
            ]),
            Line::from(""),
        ]
    } else {
        vec![
            Line::from(""),
            Line::from(vec![
                Span::raw("  "),
                Span::styled(format!("{} ", action_label), Style::default().fg(Color::White)),
                Span::styled(
                    popup.service_name.clone(),
                    Style::default().fg(action_color).add_modifier(Modifier::BOLD),
                ),
                Span::raw("?"),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::raw("        "),
                Span::styled("  Yes  ", yes_style),
                Span::raw("   "),
                Span::styled("  No  ", no_style),
            ]),
            Line::from(""),
        ]
    };

    let title = if popup.action == PopupAction::Quit {
        " Confirm QUIT "
    } else {
        &format!(" Confirm {} ", action_label.to_uppercase())
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(action_color))
        .title(Span::styled(
            title,
            Style::default().fg(action_color).add_modifier(Modifier::BOLD),
        ));

    f.render_widget(Paragraph::new(body).block(block), popup_area);
}

/* ================================================================
   TOAST — clipboard confirmation (top-right corner, auto-dismisses)
   ================================================================ */

fn render_toast(f: &mut ratatui::Frame, toast: &Toast, area: Rect) {
    let msg = &toast.message;
    let width = (msg.len() as u16 + 8).min(area.width);
    let height = 3u16;
    let toast_area = Rect {
        x: area.x + area.width.saturating_sub(width + 2),
        y: area.y + 1,
        width,
        height,
    };

    f.render_widget(RatatuiClear, toast_area);

    let paragraph = Paragraph::new(Line::from(vec![
        Span::styled(" ✓ ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
        Span::styled(msg.clone(), Style::default().fg(Color::White)),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Green))
            .title(Span::styled(
                " Copied! ",
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
            )),
    );

    f.render_widget(paragraph, toast_area);
}

/* ================================================================
   HIT TESTING
   ================================================================ */

fn click_service_index(
    col: u16,
    row: u16,
    list_area: Rect,
    services: &[Service],
) -> Option<usize> {
    if col < list_area.x
        || col >= list_area.x + list_area.width
        || row < list_area.y + 1
        || row >= list_area.y + list_area.height - 1
    {
        return None;
    }
    let relative_row = (row - list_area.y - 1) as usize;
    let lines_per_item = 2usize;
    let idx = relative_row / lines_per_item;
    if idx < services.len() { Some(idx) } else { None }
}

fn point_in_rect(col: u16, row: u16, area: Rect) -> bool {
    col >= area.x && col < area.x + area.width && row >= area.y && row < area.y + area.height
}

/* ================================================================
   MAIN RENDER LOOP
   ================================================================ */

pub async fn render_ui() -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let project_name = get_compose_project_name().await?;
    let db_config = read_db_config("db-compose.toml")?;
    let db_map: HashMap<String, DatabaseConfig> = db_config
        .database
        .into_iter()
        .map(|db| (db.name.to_lowercase(), db))
        .collect();

    let services = Arc::new(Mutex::new(Vec::<Service>::new()));
    let service_logs: Arc<Mutex<HashMap<String, Vec<String>>>> =
        Arc::new(Mutex::new(HashMap::new()));
    let selected_idx = Arc::new(Mutex::new(0usize));

    // ── Background pollers ──────────────────────────────────────
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
    let mut scroll_offset: usize = 0;
    let mut popup: Option<Popup> = None;
    let mut toast: Option<Toast> = None;

    // Saved areas from last draw — used for mouse hit testing
    let mut services_list_area = Rect::default();
    let mut logs_area = Rect::default();

    loop {
        // Expire toast before drawing so it disappears cleanly
        if let Some(ref t) = toast {
            if t.is_expired() {
                toast = None;
            }
        }

        let services_list = services.lock().unwrap().clone();
        let current_idx = *selected_idx.lock().unwrap();

        // Compute per-frame derived state once, used in both draw and input
        let selected_service = services_list.get(current_idx).cloned();
        let current_conn_str = selected_service.as_ref().and_then(|svc| {
            find_db(&svc.name, &db_map).and_then(|db| get_connection_string(svc, db))
        });

        let mut new_services_area = Rect::default();
        let mut new_logs_area = Rect::default();

        terminal.draw(|f| {
            let root = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(0), Constraint::Length(2)])
                .split(f.area());

            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
                .split(root[0]);

            new_services_area = chunks[0];

            /* ── Services list ── */
            let items: Vec<ListItem> = services_list
                .iter()
                .enumerate()
                .map(|(i, service)| {
                    let db_info = find_db(&service.name, &db_map);
                    let is_selected = i == current_idx;

                    let mut base_style = Style::default().fg(Color::Gray);
                    if is_selected {
                        base_style = if focus == Focus::Services {
                            base_style
                                .bg(Color::Yellow)
                                .fg(Color::Black)
                                .add_modifier(Modifier::BOLD)
                        } else {
                            base_style.bg(Color::DarkGray).add_modifier(Modifier::BOLD)
                        };
                    }

                    let (icon, icon_color) = if let Some(db) = db_info {
                        let color = match db.db_type {
                            DbType::Rdbms => Color::Blue,
                            DbType::Cache => Color::Yellow,
                            DbType::MessageQueue => Color::Magenta,
                            DbType::DocumentDb => Color::Green,
                        };
                        (db_icon(&db.db_type), color)
                    } else {
                        ("●", Color::DarkGray)
                    };

                    // 2 lines per item: name+status, image
                    // Port removed — lives in the Service Info panel
                    let lines = vec![
                        Line::from(vec![
                            Span::styled(format!("{} ", icon), Style::default().fg(icon_color)),
                            Span::styled(format!("{} ", service.name), base_style),
                            Span::styled(
                                format!("[{}]", service.status),
                                Style::default().fg(status_color(&service.status)),
                            ),
                        ]),
                        Line::from(Span::styled(
                            format!("  img: {}", service.image),
                            Style::default().fg(Color::DarkGray),
                        )),
                    ];

                    ListItem::new(lines).style(base_style)
                })
                .collect();

            f.render_widget(
                List::new(items).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Services")
                        .border_style(if focus == Focus::Services {
                            Style::default().fg(Color::Yellow)
                        } else {
                            Style::default()
                        }),
                ),
                chunks[0],
            );

            /* ── Right panel split ── */
            let right_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(6), Constraint::Min(0)])
                .split(chunks[1]);

            new_logs_area = right_chunks[1];

            /* ── Service Info panel ── */
            let panel_lines = if let Some(ref svc) = selected_service {
                build_service_panel(svc, find_db(&svc.name, &db_map))
            } else {
                vec![Line::from("No service selected")]
            };

            f.render_widget(
                Paragraph::new(panel_lines)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title("Service Info")
                            .border_style(Style::default().fg(Color::Blue)),
                    )
                    .wrap(Wrap { trim: true }),
                right_chunks[0],
            );

            /* ── Logs + scrollbar ── */
            let log_text = if !services_list.is_empty() && current_idx < services_list.len() {
                let logs = service_logs.lock().unwrap();
                logs.get(&services_list[current_idx].name)
                    .map(|l| l.join("\n"))
                    .unwrap_or_else(|| "Loading logs...".to_string())
            } else {
                "No services".to_string()
            };

            let num_lines = log_text.lines().count();
            let height = right_chunks[1].height.saturating_sub(2) as usize;
            let max_scroll = num_lines.saturating_sub(height);

            if auto_scroll {
                scroll_offset = max_scroll;
            } else {
                scroll_offset = scroll_offset.min(max_scroll);
                if scroll_offset >= max_scroll && max_scroll > 0 {
                    auto_scroll = true;
                }
            }

            // Shrink width by 1 col so scrollbar doesn't overlap text
            let inner_log_area = Rect {
                x: right_chunks[1].x,
                y: right_chunks[1].y,
                width: right_chunks[1].width.saturating_sub(1),
                height: right_chunks[1].height,
            };

            f.render_widget(
                Paragraph::new(log_text)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title(if auto_scroll { "Logs [FOLLOW]" } else { "Logs [PAUSED]" })
                            .border_style(if focus == Focus::Logs {
                                Style::default().fg(Color::Yellow)
                            } else {
                                Style::default()
                            }),
                    )
                    .wrap(Wrap { trim: false })
                    .scroll((scroll_offset as u16, 0)),
                inner_log_area,
            );

            let mut scrollbar_state =
                ScrollbarState::new(max_scroll.max(1)).position(scroll_offset);

            f.render_stateful_widget(
                Scrollbar::new(ScrollbarOrientation::VerticalRight)
                    .begin_symbol(Some("▲"))
                    .end_symbol(Some("▼"))
                    .track_symbol(Some("│"))
                    .thumb_symbol("█"),
                Rect {
                    x: right_chunks[1].x + right_chunks[1].width.saturating_sub(1),
                    y: right_chunks[1].y + 1,
                    width: 1,
                    height: right_chunks[1].height.saturating_sub(2),
                },
                &mut scrollbar_state,
            );

            /* ── Help bar ── */
            let show_open = selected_service
                .as_ref()
                .and_then(|s| find_db(&s.name, &db_map))
                .map(|db| {
                    is_ui_service(
                        selected_service.as_ref().unwrap().name.as_str(),
                        db,
                    ) && db.studio_port.is_some()
                })
                .unwrap_or(false);

            f.render_widget(
                Paragraph::new(help_text(&focus, show_open, current_conn_str.is_some()))
                    .style(Style::default().fg(Color::DarkGray))
                    .block(Block::default().borders(Borders::TOP)),
                root[1],
            );

            /* ── Popup (on top of everything else) ── */
            if let Some(ref p) = popup {
                render_popup(f, p, f.area());
            }

            /* ── Toast (topmost layer) ── */
            if let Some(ref t) = toast {
                render_toast(f, t, f.area());
            }
        })?;

        services_list_area = new_services_area;
        logs_area = new_logs_area;

        /* ================================================================
           INPUT
           ================================================================ */

        if event::poll(Duration::from_millis(100))? {
            match event::read()? {
                /* ── Mouse ── */
                Event::Mouse(mouse) => {
                    // Any click dismisses popup
                    if popup.is_some() {
                        if mouse.kind == MouseEventKind::Down(MouseButton::Left) {
                            popup = None;
                        }
                        continue;
                    }

                    match mouse.kind {
                        MouseEventKind::Down(MouseButton::Left) => {
                            let (col, row) = (mouse.column, mouse.row);
                            if let Some(idx) =
                                click_service_index(col, row, services_list_area, &services_list)
                            {
                                focus = Focus::Services;
                                *selected_idx.lock().unwrap() = idx;
                            } else if point_in_rect(col, row, logs_area) {
                                focus = Focus::Logs;
                            }
                        }
                        MouseEventKind::ScrollUp => {
                            if focus == Focus::Logs {
                                auto_scroll = false;
                                scroll_offset = scroll_offset.saturating_sub(3);
                            }
                        }
                        MouseEventKind::ScrollDown => {
                            if focus == Focus::Logs {
                                let snap = services.lock().unwrap().clone();
                                let log_text = if current_idx < snap.len() {
                                    let logs = service_logs.lock().unwrap();
                                    logs.get(&snap[current_idx].name)
                                        .map(|l| l.join("\n"))
                                        .unwrap_or_default()
                                } else {
                                    String::new()
                                };
                                let max_scroll = log_text
                                    .lines()
                                    .count()
                                    .saturating_sub(logs_area.height.saturating_sub(2) as usize);
                                scroll_offset = (scroll_offset + 3).min(max_scroll);
                                auto_scroll = scroll_offset >= max_scroll;
                            }
                        }
                        _ => {}
                    }
                }

                /* ── Keyboard ── */
                Event::Key(key) => {
                    /* ── Popup active — handle its own keys ── */
                    if let Some(ref mut p) = popup {
                        match key.code {
                            KeyCode::Left | KeyCode::Char('h') => p.selected = 0,
                            KeyCode::Right | KeyCode::Char('l') => p.selected = 1,
                            KeyCode::Tab => p.selected = (p.selected + 1) % 2,
                            KeyCode::Enter => {
                                if p.selected == 0 {
                                    if p.action == PopupAction::Quit {
                                        // User confirmed quit
                                        popup = None;
                                        break;
                                    } else {
                                        // Start/Stop service
                                        let list = services.lock().unwrap().clone();
                                        let idx = *selected_idx.lock().unwrap();
                                        if let Some(svc) = list.get(idx) {
                                            let action = match p.action {
                                                PopupAction::Start => "start",
                                                PopupAction::Stop => "stop",
                                                PopupAction::Quit => unreachable!(),
                                            };
                                            let cid = svc.container_id.clone();
                                            tokio::spawn(async move {
                                                let _ = tokio::process::Command::new("docker")
                                                    .args([action, &cid])
                                                    .output()
                                                    .await;
                                            });
                                        }
                                    }
                                }
                                popup = None;
                            }
                            KeyCode::Esc => popup = None,
                            _ => {}
                        }
                        continue;
                    }

                    /* ── Normal keys ── */
                    match key.code {
                        KeyCode::Char('q') => {
                            // Show quit confirmation popup
                            popup = Some(Popup {
                                service_name: String::new(),
                                action: PopupAction::Quit,
                                selected: 1, // Default to "No" for safety
                            });
                        }

                        KeyCode::Left => focus = Focus::Services,
                        KeyCode::Right => focus = Focus::Logs,

                        // ── Copy connection string to clipboard ─────
                        KeyCode::Char('c') => {
                            if let Some(ref conn) = current_conn_str {
                                toast = Some(match copy_to_clipboard(conn) {
                                    Ok(()) => Toast::new(
                                        "Connection string copied to clipboard",
                                        Duration::from_secs(3),
                                    ),
                                    Err(e) => Toast::new(
                                        format!("Copy failed: {}", e),
                                        Duration::from_secs(4),
                                    ),
                                });
                            }
                        }

                        // ── Enter → start/stop confirm popup ────────
                        KeyCode::Enter => {
                            if focus == Focus::Services {
                                let list = services.lock().unwrap().clone();
                                let idx = *selected_idx.lock().unwrap();
                                if let Some(svc) = list.get(idx) {
                                    popup = Some(Popup {
                                        service_name: svc.name.clone(),
                                        action: if svc.status == "running" {
                                            PopupAction::Stop
                                        } else {
                                            PopupAction::Start
                                        },
                                        selected: 0,
                                    });
                                }
                            }
                        }

                        // ── Open UI in browser ───────────────────────
                        KeyCode::Char('o') => {
                            let list = services.lock().unwrap().clone();
                            let idx = *selected_idx.lock().unwrap();
                            if let Some(svc) = list.get(idx) {
                                if let Some(db) = find_db(&svc.name, &db_map) {
                                    if is_ui_service(&svc.name, db) {
                                        if let Some(port) = &db.studio_port {
                                            let _ = open::that(format!("http://localhost:{}", port));
                                        }
                                    }
                                }
                            }
                        }

                        // ── Shell into container ─────────────────────
                        KeyCode::Char('s') => {
                            let list = services.lock().unwrap().clone();
                            let idx = *selected_idx.lock().unwrap();
                            if let Some(svc) = list.get(idx) {
                                if !svc.container_id.is_empty() {
                                    disable_raw_mode()?;
                                    execute!(
                                        terminal.backend_mut(),
                                        LeaveAlternateScreen,
                                        DisableMouseCapture,
                                        Clear(ClearType::All),
                                        MoveTo(0, 0)
                                    )?;
                                    terminal.show_cursor()?;
                                    io::stdout().flush()?;

                                    let _ = open_shell(&svc.container_id).await;

                                    enable_raw_mode()?;
                                    execute!(
                                        terminal.backend_mut(),
                                        EnterAlternateScreen,
                                        EnableMouseCapture
                                    )?;
                                    terminal.hide_cursor()?;
                                    terminal.clear()?;
                                }
                            }
                        }

                        // ── Scroll / navigation ──────────────────────
                        KeyCode::Down | KeyCode::Char('j') => {
                            if focus == Focus::Services {
                                let len = services.lock().unwrap().len();
                                if len > 0 {
                                    let mut idx = selected_idx.lock().unwrap();
                                    *idx = (*idx + 1).min(len - 1);
                                }
                            } else {
                                auto_scroll = false;
                                scroll_offset += 1;
                            }
                        }

                        KeyCode::Up | KeyCode::Char('k') => {
                            if focus == Focus::Services {
                                let mut idx = selected_idx.lock().unwrap();
                                *idx = idx.saturating_sub(1);
                            } else {
                                auto_scroll = false;
                                scroll_offset = scroll_offset.saturating_sub(1);
                            }
                        }

                        KeyCode::PageDown => {
                            if focus == Focus::Logs {
                                auto_scroll = true;
                            }
                        }

                        KeyCode::PageUp => {
                            if focus == Focus::Logs {
                                auto_scroll = false;
                                scroll_offset = 0;
                            }
                        }

                        KeyCode::Char('g') => {
                            if focus == Focus::Logs {
                                auto_scroll = false;
                                scroll_offset = 0;
                            }
                        }

                        KeyCode::Char('G') => {
                            if focus == Focus::Logs {
                                auto_scroll = true;
                            }
                        }

                        _ => {}
                    }
                }

                _ => {}
            }
        }
    }

    // Clean up terminal on exit
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
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

async fn get_docker_services(
    project_name: &str,
) -> Result<Vec<Service>, Box<dyn std::error::Error>> {
    let output = tokio::process::Command::new("docker")
        .args(&[
            "ps",
            "-a",
            "--filter",
            &format!("label=com.docker.compose.project={}", project_name),
            "--format",
            "{{.ID}}|{{.Label \"com.docker.compose.service\"}}|{{.Status}}|{{.Image}}",
        ])
        .output()
        .await?;

    let result = String::from_utf8(output.stdout)?;
    Ok(result
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| {
            let parts: Vec<&str> = line.split('|').collect();
            Service {
                container_id: parts.get(0).unwrap_or(&"").to_string(),
                name: parts.get(1).unwrap_or(&"unknown").to_string(),
                status: parse_status(parts.get(2).unwrap_or(&"unknown")),
                image: parts.get(3).unwrap_or(&"unknown").to_string(),
            }
        })
        .collect())
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

async fn get_container_logs(
    container_id: &str,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let output = tokio::process::Command::new("docker")
        .args(&["logs", "--tail", "500", container_id])
        .output()
        .await?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    let mut lines: Vec<String> = stdout
        .lines()
        .chain(stderr.lines())
        .map(|s| s.to_string())
        .collect();

    if lines.len() > 500 {
        lines.drain(0..lines.len() - 500);
    }

    Ok(lines)
}

async fn open_shell(container_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    use std::process::Stdio;
    use tokio::process::Command;

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

    Command::new("docker")
        .args(["exec", "-it", container_id, "sh"])
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .await?;

    Ok(())
}