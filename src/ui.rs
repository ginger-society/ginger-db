use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style, Stylize},
    widgets::{Block, Borders, Paragraph, Row, Table},
    Terminal,
};
use std::{
    io::{self},
    process::Stdio,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

pub async fn render_ui() -> Result<(), Box<dyn std::error::Error>> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Store the output of the command in an Arc<Mutex<Vec<String>>>
    let output_lines = Arc::new(Mutex::new(Vec::new()));

    // Clone the Arc for use in the async task
    let output_lines_clone = Arc::clone(&output_lines);

    // Run the "docker-compose up" command asynchronously and capture its output
    tokio::spawn(async move {
        let mut child = Command::new("docker-compose")
            .arg("up")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to spawn command");

        if let Some(stdout) = child.stdout.take() {
            let mut reader = BufReader::new(stdout).lines();

            while let Ok(Some(line)) = reader.next_line().await {
                let mut lines = output_lines_clone.lock().unwrap();
                lines.push(line);
            }
        }

        if let Some(stderr) = child.stderr.take() {
            let mut reader = BufReader::new(stderr).lines();

            while let Ok(Some(line)) = reader.next_line().await {
                let mut lines = output_lines_clone.lock().unwrap();
                lines.push(line);
            }
        }
    });

    // State for the table selection
    let selected_row = Arc::new(Mutex::new(0));
    let selected_row_clone = Arc::clone(&selected_row);

    // Scroll position
    let mut scroll: u16 = 0;

    // Event loop
    loop {
        // UI Layout
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(
                    [
                        Constraint::Percentage(50), // Left panel
                        Constraint::Percentage(50), // Right panel
                    ]
                    .as_ref(),
                )
                .split(f.size());

            // Left panel displaying a static table
            let rows = [
                Row::new(vec!["RDBMS", "MySQL", "Active"]),
                Row::new(vec!["DocumentDB", "MongoDB", "Inactive"]),
                Row::new(vec!["Cache", "Redis", "Active"]),
            ];

            let widths = [
                Constraint::Length(10),
                Constraint::Length(10),
                Constraint::Length(10),
            ];

            // Apply highlight style based on the selected row
            let rows: Vec<Row> = rows
                .iter()
                .enumerate()
                .map(|(i, row)| {
                    if i == *selected_row.lock().unwrap() {
                        row.clone().style(Style::default().bg(Color::LightYellow))
                    } else {
                        row.clone()
                    }
                })
                .collect();

            let table = Table::new(rows, widths)
                .header(
                    Row::new(vec!["Type", "Name", "Active"])
                        .style(Style::default().bold())
                        .bottom_margin(1),
                )
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Configuration"),
                )
                .column_spacing(1)
                .style(Style::default().fg(Color::White))
                .highlight_style(Style::default().bg(Color::LightYellow))
                .highlight_symbol(">>");

            f.render_widget(table, chunks[0]);

            // Right panel displaying command output
            let lines = output_lines.lock().unwrap();
            let content_height = lines.len() as u16;
            // Update scroll position to follow the new output
            scroll = if content_height > f.size().height {
                content_height - f.size().height
            } else {
                0
            };

            let right_panel = Paragraph::new(lines.join("\n"))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Command Output"),
                )
                .scroll((scroll, 0)); // Apply scroll
            f.render_widget(right_panel, chunks[1]);
        })?;

        // Handle input events
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Down => {
                        let mut row = selected_row.lock().unwrap();
                        *row = (*row + 1).min(2); // Adjust based on the number of rows
                    }
                    KeyCode::Up => {
                        let mut row = selected_row.lock().unwrap();
                        *row = (*row as u16).saturating_sub(1) as usize; // Adjust based on the number of rows
                    }
                    KeyCode::Enter => {
                        let mut rows = ["RDBMS", "DocumentDB", "Cache"];

                        let mut status = ["Active", "Inactive", "Active"];

                        let mut row = selected_row.lock().unwrap();
                        let row_idx = *row;
                        status[row_idx] = if status[row_idx] == "Active" {
                            "Inactive"
                        } else {
                            "Active"
                        };

                        // Apply the updated status
                        rows[row_idx] = "Active";
                    }
                    _ => {}
                }
            }
        }
    }

    // Cleanup terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}
