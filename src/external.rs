use std::path::Path;
use std::process::Command;

use color_eyre::eyre::Result;

use crate::tui::{self, Tui};

/// Suspends the TUI, hands the whole terminal to lazygit (or a plain `git
/// status` fallback), then re-enters the alternate screen and forces a full
/// repaint since the child process leaves the screen buffer in an unknown
/// state.
pub fn open_selected(terminal: &mut Tui, repo_path: &Path) -> Result<()> {
    tui::restore()?;

    let outcome = if let Ok(bin) = which::which("lazygit") {
        Command::new(bin).current_dir(repo_path).status()
    } else {
        println!(
            "lazygit not found on PATH; running `git status` in {}",
            repo_path.display()
        );
        let status = Command::new("git")
            .arg("status")
            .current_dir(repo_path)
            .status();
        println!("\n(press Enter to return to reporadar)");
        let mut buf = String::new();
        let _ = std::io::stdin().read_line(&mut buf);
        status
    };

    *terminal = tui::init()?;
    terminal.clear()?;
    outcome?;
    Ok(())
}
