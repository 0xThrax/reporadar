use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, Cell, Row, Table, TableState};
use ratatui::Frame;

use crate::app::{App, RowState};

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let header = Row::new(vec![
        "Repo",
        "Branch",
        "Status",
        "Ahead/Behind",
        "Last Commit",
    ])
    .style(Style::default().add_modifier(Modifier::BOLD))
    .bottom_margin(1);

    let rows: Vec<Row> = app
        .filtered
        .iter()
        .map(|&i| {
            let repo = &app.repos[i];
            let branch = repo.branch.clone().unwrap_or_else(|| "-".to_string());

            let (status_text, status_style) = if let Some(err) = &repo.error {
                (format!("error: {err}"), Style::default().fg(Color::Red))
            } else {
                match repo.state {
                    RowState::Scanning => (
                        "scanning\u{2026}".to_string(),
                        Style::default().fg(Color::DarkGray),
                    ),
                    RowState::Fetching => (
                        "fetching\u{2026}".to_string(),
                        Style::default().fg(Color::Cyan),
                    ),
                    RowState::Pulling => (
                        "pulling\u{2026}".to_string(),
                        Style::default().fg(Color::Cyan),
                    ),
                    RowState::Idle if repo.dirty => (
                        "\u{25cf} dirty".to_string(),
                        Style::default().fg(Color::Yellow),
                    ),
                    RowState::Idle => (
                        "\u{2713} clean".to_string(),
                        Style::default().fg(Color::Green),
                    ),
                }
            };

            let ab = format!("\u{2191}{} \u{2193}{}", repo.ahead, repo.behind);
            let ab_style = if repo.ahead > 0 && repo.behind == 0 {
                Style::default().fg(Color::Cyan)
            } else if repo.behind > 0 && repo.ahead == 0 {
                Style::default().fg(Color::Magenta)
            } else {
                Style::default()
            };

            let commit = if repo.last_commit_msg.is_empty() {
                repo.last_commit_relative.clone()
            } else {
                format!("{}  {}", repo.last_commit_relative, repo.last_commit_msg)
            };

            Row::new(vec![
                Cell::from(repo.name.clone()),
                Cell::from(branch),
                Cell::from(status_text).style(status_style),
                Cell::from(ab).style(ab_style),
                Cell::from(commit),
            ])
        })
        .collect();

    let widths = [
        Constraint::Percentage(25),
        Constraint::Percentage(15),
        Constraint::Percentage(18),
        Constraint::Percentage(12),
        Constraint::Percentage(30),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(Block::default().borders(Borders::ALL).title(" repos "))
        .row_highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("\u{25b6} ");

    let mut state = TableState::default();
    state.select(Some(app.selected));

    frame.render_stateful_widget(table, area, &mut state);
}
