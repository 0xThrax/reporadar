use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::App;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let (text, style) = if let Some((msg, is_error)) = &app.status_line {
        let color = if *is_error { Color::Red } else { Color::Green };
        (msg.clone(), Style::default().fg(color))
    } else if app.scanning && app.repos.is_empty() {
        (
            format!(
                " scanning {} for git repositories\u{2026}",
                app.root.display()
            ),
            Style::default().fg(Color::Cyan),
        )
    } else {
        let mut hint = String::from(
            " \u{2191}/\u{2193} move  r rescan  f fetch-all  p pull  Enter open  / filter  q quit",
        );
        if app.inflight > 0 {
            hint.push_str(&format!("   ({} in flight)", app.inflight));
        }
        (hint, Style::default().fg(Color::DarkGray))
    };

    frame.render_widget(Paragraph::new(text).style(style), area);
}
