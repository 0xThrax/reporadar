//! End-to-end tests: real git repos in a temp dir, driven through the real
//! worker pool and rendered through a real ratatui TestBackend terminal.
//! Requires `git` on PATH (same requirement as the app itself).

use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

use crossbeam_channel::unbounded;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{backend::TestBackend, Terminal};

use reporadar::app::App;
use reporadar::worker::{self, Job, JobResult};
use reporadar::{scan, ui};

fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0")
        .status()
        .expect("spawn git");
    assert!(status.success(), "git {args:?} failed in {}", dir.display());
}

fn make_repo(root: &Path, name: &str, dirty: bool) {
    let dir = root.join(name);
    std::fs::create_dir_all(&dir).unwrap();
    git(&dir, &["init", "-q", "-b", "main"]);
    git(&dir, &["config", "user.email", "test@example.com"]);
    git(&dir, &["config", "user.name", "Test"]);
    std::fs::write(dir.join("README.md"), name).unwrap();
    git(&dir, &["add", "."]);
    git(&dir, &["commit", "-qm", "initial commit"]);
    if dirty {
        std::fs::write(dir.join("untracked.txt"), "dirty").unwrap();
    }
}

/// Boot the real pipeline against `root` and pump results until quiescent.
fn spawn_and_settle(root: &Path) -> (App, crossbeam_channel::Receiver<JobResult>) {
    let (job_tx, job_rx) = unbounded::<Job>();
    let (result_tx, result_rx) = unbounded::<JobResult>();
    worker::spawn_pool(2, job_rx, result_tx);

    let mut app = App::new(root.to_path_buf(), None, job_tx);
    app.start_scan();
    settle(&mut app, &result_rx);
    (app, result_rx)
}

fn settle(app: &mut App, result_rx: &crossbeam_channel::Receiver<JobResult>) {
    let deadline = Instant::now() + Duration::from_secs(30);
    while app.inflight > 0 {
        assert!(Instant::now() < deadline, "pipeline did not settle in 30s");
        match result_rx.recv_timeout(Duration::from_secs(5)) {
            Ok(result) => app.apply(result),
            Err(e) => panic!("worker result timeout: {e}"),
        }
    }
}

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn render_to_string(app: &App) -> String {
    let backend = TestBackend::new(100, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| ui::draw(f, app)).unwrap();
    let buffer = terminal.backend().buffer().clone();
    let mut out = String::new();
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            out.push_str(buffer[(x, y)].symbol());
        }
        out.push('\n');
    }
    out
}

#[test]
fn scans_real_repos_and_reports_status() {
    let tmp = tempfile::tempdir().unwrap();
    make_repo(tmp.path(), "alpha", false);
    make_repo(tmp.path(), "beta", true);
    std::fs::create_dir_all(tmp.path().join("not-a-repo")).unwrap();

    let (app, _rx) = spawn_and_settle(tmp.path());

    assert_eq!(app.repos.len(), 2, "exactly the two git repos are found");
    let alpha = app.repos.iter().find(|r| r.name == "alpha").unwrap();
    let beta = app.repos.iter().find(|r| r.name == "beta").unwrap();

    assert_eq!(alpha.branch.as_deref(), Some("main"));
    assert!(!alpha.dirty);
    assert!(beta.dirty, "untracked file marks the repo dirty");
    assert_eq!(alpha.last_commit_msg, "initial commit");
    assert!(alpha.error.is_none() && beta.error.is_none());
}

#[test]
fn renders_repo_table_through_test_backend() {
    let tmp = tempfile::tempdir().unwrap();
    make_repo(tmp.path(), "alpha", false);
    make_repo(tmp.path(), "beta", true);

    let (app, _rx) = spawn_and_settle(tmp.path());
    let screen = render_to_string(&app);

    assert!(screen.contains("reporadar"), "title bar renders");
    assert!(screen.contains("alpha"), "repo row renders:\n{screen}");
    assert!(screen.contains("beta"));
    assert!(screen.contains("initial commit"), "commit message renders");
}

#[test]
fn filter_mode_narrows_and_escape_clears() {
    let tmp = tempfile::tempdir().unwrap();
    make_repo(tmp.path(), "alpha", false);
    make_repo(tmp.path(), "beta", false);

    let (mut app, _rx) = spawn_and_settle(tmp.path());
    assert_eq!(app.filtered.len(), 2);

    app.on_key(key(KeyCode::Char('/')));
    for c in "alp".chars() {
        app.on_key(key(KeyCode::Char(c)));
    }
    assert_eq!(app.filtered.len(), 1, "fuzzy filter narrows to alpha");
    assert_eq!(app.repos[app.filtered[0]].name, "alpha");

    let screen = render_to_string(&app);
    assert!(screen.contains("/alp"), "filter input renders");

    app.on_key(key(KeyCode::Esc));
    assert_eq!(app.filtered.len(), 2, "escape clears the filter");
}

#[test]
fn fetch_all_round_trips_back_to_idle() {
    let tmp = tempfile::tempdir().unwrap();
    make_repo(tmp.path(), "alpha", false);

    let (mut app, result_rx) = spawn_and_settle(tmp.path());
    app.on_key(key(KeyCode::Char('f')));
    settle(&mut app, &result_rx);

    // fetch --all on a remote-less repo is a no-op success in git; either
    // way the row must come back to a consistent idle state.
    assert_eq!(app.repos.len(), 1);
    assert_eq!(app.repos[0].branch.as_deref(), Some("main"));
}

#[test]
fn quit_keys_set_should_quit() {
    let tmp = tempfile::tempdir().unwrap();
    make_repo(tmp.path(), "alpha", false);
    let (mut app, _rx) = spawn_and_settle(tmp.path());

    assert!(!app.should_quit);
    app.on_key(key(KeyCode::Char('q')));
    assert!(app.should_quit);
}

#[test]
fn scan_respects_max_depth() {
    let tmp = tempfile::tempdir().unwrap();
    make_repo(tmp.path(), "top", false);
    make_repo(&tmp.path().join("nested/deeper"), "inner", false);

    let shallow = scan::find_repos(tmp.path(), Some(1));
    let deep = scan::find_repos(tmp.path(), None);
    assert_eq!(shallow.len(), 1, "depth 1 finds only the top-level repo");
    assert_eq!(deep.len(), 2, "unbounded scan finds nested repos too");
}
