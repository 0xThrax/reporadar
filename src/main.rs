mod app;
mod event;
mod external;
mod git;
mod scan;
mod tui;
mod ui;
mod worker;

use std::path::PathBuf;
use std::time::Duration;

use clap::Parser;
use color_eyre::eyre::Result;
use crossbeam_channel::unbounded;
use crossterm::event::{Event, KeyEventKind};

use app::App;
use worker::{Job, JobResult};

#[derive(Parser)]
#[command(
    name = "reporadar",
    about = "A live terminal dashboard for the status of every git repo under a directory tree"
)]
struct Cli {
    /// Root directory to scan for git repos
    #[arg(default_value = ".")]
    root: PathBuf,

    /// Maximum directory depth to scan
    #[arg(long)]
    max_depth: Option<usize>,
}

fn main() -> Result<()> {
    color_eyre::install()?;
    tui::install_panic_hook();

    let cli = Cli::parse();

    if !cli.root.exists() {
        eprintln!("error: path {} does not exist", cli.root.display());
        std::process::exit(1);
    }

    let (job_tx, job_rx) = unbounded::<Job>();
    let (result_tx, result_rx) = unbounded::<JobResult>();

    let workers = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
        .min(8);
    worker::spawn_pool(workers, job_rx, result_tx);

    let mut app = App::new(cli.root.clone(), cli.max_depth, job_tx);
    app.start_scan();

    let mut terminal = tui::init()?;

    loop {
        while let Ok(result) = result_rx.try_recv() {
            app.apply(result);
        }

        terminal.draw(|f| ui::draw(f, &app))?;

        if let Some(event) = event::next_event(Duration::from_millis(120))? {
            if let Event::Key(key) = event {
                if key.kind == KeyEventKind::Press {
                    app.on_key(key);
                }
            }
        }

        if let Some(path) = app.take_pending_external() {
            external::open_selected(&mut terminal, &path)?;
        }

        if app.should_quit {
            break;
        }
    }

    tui::restore()?;
    Ok(())
}
