use std::io::{stdout, Stdout};

use color_eyre::eyre::Result;
use crossterm::cursor::{Hide, Show};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, is_raw_mode_enabled, EnterAlternateScreen,
    LeaveAlternateScreen,
};
use crossterm::execute;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

pub type Tui = Terminal<CrosstermBackend<Stdout>>;

pub fn init() -> Result<Tui> {
    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen, Hide)?;
    Ok(Terminal::new(CrosstermBackend::new(stdout()))?)
}

/// Idempotent: safe to call even if the terminal is already restored (e.g.
/// from a panic hook racing with normal shutdown).
pub fn restore() -> Result<()> {
    if is_raw_mode_enabled().unwrap_or(false) {
        execute!(stdout(), LeaveAlternateScreen, Show)?;
        disable_raw_mode()?;
    }
    Ok(())
}

pub fn install_panic_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = restore();
        default_hook(info);
    }));
}
