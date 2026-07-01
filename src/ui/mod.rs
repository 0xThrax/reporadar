mod statusbar;
mod table;

use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::{App, Mode};

pub fn draw(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(3),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(frame.area());

    let title = Paragraph::new(format!(" reporadar \u{2014} {}", app.root.display()))
        .style(Style::default().add_modifier(Modifier::BOLD));
    frame.render_widget(title, chunks[0]);

    table::render(frame, chunks[1], app);
    statusbar::render(frame, chunks[2], app);

    let filter_text = match app.mode {
        Mode::Filter => format!(" /{}", app.filter_input),
        Mode::Normal if !app.filter_input.is_empty() => {
            format!(" /{} (Esc to clear)", app.filter_input)
        }
        _ => String::new(),
    };
    frame.render_widget(Paragraph::new(filter_text), chunks[3]);
}
