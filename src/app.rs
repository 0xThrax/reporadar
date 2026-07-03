use std::path::PathBuf;

use crossbeam_channel::Sender;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config, Matcher, Utf32Str};

use crate::worker::{Job, JobResult};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RowState {
    Scanning,
    Fetching,
    Pulling,
    Idle,
}

pub struct RepoStatus {
    pub path: PathBuf,
    pub name: String,
    pub branch: Option<String>,
    pub ahead: u32,
    pub behind: u32,
    pub dirty: bool,
    pub last_commit_relative: String,
    pub last_commit_msg: String,
    pub state: RowState,
    pub error: Option<String>,
}

#[derive(PartialEq, Eq)]
pub enum Mode {
    Normal,
    Filter,
}

pub struct App {
    pub root: PathBuf,
    max_depth: Option<usize>,
    pub repos: Vec<RepoStatus>,
    pub filtered: Vec<usize>,
    pub selected: usize,
    pub mode: Mode,
    pub filter_input: String,
    pub inflight: usize,
    pub scanning: bool,
    pub status_line: Option<(String, bool)>,
    pub should_quit: bool,
    pending_external: Option<PathBuf>,
    job_tx: Sender<Job>,
}

impl App {
    pub fn new(root: PathBuf, max_depth: Option<usize>, job_tx: Sender<Job>) -> Self {
        App {
            root,
            max_depth,
            repos: Vec::new(),
            filtered: Vec::new(),
            selected: 0,
            mode: Mode::Normal,
            filter_input: String::new(),
            inflight: 0,
            scanning: false,
            status_line: None,
            should_quit: false,
            pending_external: None,
            job_tx,
        }
    }

    /// Kicks off the directory walk on the background pool rather than
    /// blocking here - walking a large tree (e.g. a home directory full of
    /// toolchain/package caches) can take several seconds, and doing it
    /// synchronously before the terminal is even initialized would cause
    /// exactly the startup freeze this architecture is meant to avoid.
    pub fn start_scan(&mut self) {
        self.scanning = true;
        self.job_tx
            .send(Job::ScanRoot(self.root.clone(), self.max_depth))
            .ok();
        self.inflight += 1;
    }

    pub fn load_repos(&mut self, paths: Vec<PathBuf>) {
        self.repos = paths
            .into_iter()
            .map(|path| {
                let name = path
                    .strip_prefix(&self.root)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .to_string();
                let name = if name.is_empty() {
                    ".".to_string()
                } else {
                    name
                };

                self.job_tx.send(Job::Status(path.clone())).ok();
                self.inflight += 1;

                RepoStatus {
                    path,
                    name,
                    branch: None,
                    ahead: 0,
                    behind: 0,
                    dirty: false,
                    last_commit_relative: String::new(),
                    last_commit_msg: String::new(),
                    state: RowState::Scanning,
                    error: None,
                }
            })
            .collect();

        self.apply_filter();
    }

    pub fn apply(&mut self, result: JobResult) {
        self.inflight = self.inflight.saturating_sub(1);

        match result {
            JobResult::Scanned(paths) => {
                self.scanning = false;
                self.load_repos(paths);
            }
            JobResult::Status(path, result) => {
                if let Some(repo) = self.repos.iter_mut().find(|r| r.path == path) {
                    match result {
                        Ok((status, commit)) => {
                            repo.branch = status.branch;
                            repo.ahead = status.ahead;
                            repo.behind = status.behind;
                            repo.dirty = status.dirty;
                            repo.last_commit_relative = commit.relative;
                            repo.last_commit_msg = commit.message;
                            repo.error = None;
                        }
                        Err(e) => repo.error = Some(e.to_string()),
                    }
                    repo.state = RowState::Idle;
                }
            }
            JobResult::Fetched(path, result) => match result {
                Ok(()) => {
                    self.job_tx.send(Job::Status(path.clone())).ok();
                    self.inflight += 1;
                }
                Err(e) => {
                    if let Some(repo) = self.repos.iter_mut().find(|r| r.path == path) {
                        repo.error = Some(e.to_string());
                        repo.state = RowState::Idle;
                    }
                    self.status_line = Some((format!("fetch failed: {e}"), true));
                }
            },
            JobResult::Pulled(path, result) => {
                if let Some(repo) = self.repos.iter_mut().find(|r| r.path == path) {
                    repo.state = RowState::Idle;
                }
                match result {
                    Ok(()) => {
                        self.job_tx.send(Job::Status(path.clone())).ok();
                        self.inflight += 1;
                        self.status_line = Some(("pull complete".to_string(), false));
                    }
                    Err(e) => {
                        if let Some(repo) = self.repos.iter_mut().find(|r| r.path == path) {
                            repo.error = Some(e.to_string());
                        }
                        self.status_line = Some((format!("pull failed: {e}"), true));
                    }
                }
            }
        }
    }

    pub fn on_key(&mut self, key: KeyEvent) {
        match self.mode {
            Mode::Filter => self.on_key_filter(key),
            Mode::Normal => self.on_key_normal(key),
        }
    }

    pub fn take_pending_external(&mut self) -> Option<PathBuf> {
        self.pending_external.take()
    }

    fn on_key_normal(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.should_quit = true
            }
            KeyCode::Esc => self.should_quit = true,
            KeyCode::Up | KeyCode::Char('k') => self.move_selection(-1),
            KeyCode::Down | KeyCode::Char('j') => self.move_selection(1),
            KeyCode::Char('r') => self.rescan(),
            KeyCode::Char('f') => self.fetch_all(),
            KeyCode::Char('p') => self.pull_selected(),
            KeyCode::Enter => self.open_selected(),
            KeyCode::Char('/') => self.mode = Mode::Filter,
            _ => {}
        }
    }

    fn on_key_filter(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.filter_input.clear();
                self.mode = Mode::Normal;
                self.apply_filter();
            }
            KeyCode::Enter => self.mode = Mode::Normal,
            KeyCode::Backspace => {
                self.filter_input.pop();
                self.apply_filter();
            }
            KeyCode::Char(c) => {
                self.filter_input.push(c);
                self.apply_filter();
            }
            _ => {}
        }
    }

    fn move_selection(&mut self, delta: i32) {
        if self.filtered.is_empty() {
            return;
        }
        let len = self.filtered.len() as i32;
        let idx = (self.selected as i32 + delta).clamp(0, len - 1);
        self.selected = idx as usize;
    }

    fn selected_repo_index(&self) -> Option<usize> {
        self.filtered.get(self.selected).copied()
    }

    fn rescan(&mut self) {
        self.start_scan();
        self.status_line = Some(("rescanning\u{2026}".to_string(), false));
    }

    fn fetch_all(&mut self) {
        let paths: Vec<PathBuf> = self
            .filtered
            .iter()
            .map(|&i| self.repos[i].path.clone())
            .collect();
        for path in paths {
            if let Some(repo) = self.repos.iter_mut().find(|r| r.path == path) {
                repo.state = RowState::Fetching;
            }
            self.job_tx.send(Job::Fetch(path)).ok();
            self.inflight += 1;
        }
        self.status_line = Some(("fetching all...".to_string(), false));
    }

    fn pull_selected(&mut self) {
        if let Some(i) = self.selected_repo_index() {
            let path = self.repos[i].path.clone();
            self.repos[i].state = RowState::Pulling;
            self.job_tx.send(Job::Pull(path)).ok();
            self.inflight += 1;
        }
    }

    fn open_selected(&mut self) {
        if let Some(i) = self.selected_repo_index() {
            self.pending_external = Some(self.repos[i].path.clone());
        }
    }

    fn apply_filter(&mut self) {
        if self.filter_input.is_empty() {
            self.filtered = (0..self.repos.len()).collect();
        } else {
            let mut matcher = Matcher::new(Config::DEFAULT);
            let pattern = Pattern::parse(
                &self.filter_input,
                CaseMatching::Ignore,
                Normalization::Smart,
            );

            let mut scored: Vec<(usize, u32)> = self
                .repos
                .iter()
                .enumerate()
                .filter_map(|(i, r)| {
                    let mut buf = Vec::new();
                    let haystack = Utf32Str::new(&r.name, &mut buf);
                    pattern
                        .score(haystack, &mut matcher)
                        .map(|score| (i, score))
                })
                .collect();

            scored.sort_by_key(|&(_, score)| std::cmp::Reverse(score));
            self.filtered = scored.into_iter().map(|(i, _)| i).collect();
        }

        if self.selected >= self.filtered.len() {
            self.selected = self.filtered.len().saturating_sub(1);
        }
    }
}
