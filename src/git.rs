use std::path::Path;
use std::process::{Command, Stdio};

use color_eyre::eyre::{bail, Context, Result};

pub struct StatusInfo {
    pub branch: Option<String>,
    pub ahead: u32,
    pub behind: u32,
    pub dirty: bool,
}

pub struct CommitInfo {
    pub relative: String,
    pub message: String,
}

fn run_git(repo: &Path, args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .with_context(|| {
            format!(
                "failed to spawn `git {}` in {}",
                args.join(" "),
                repo.display()
            )
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git {} failed: {}", args.join(" "), stderr.trim());
    }

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Parses `git status --porcelain=v2 --branch` in one pass: branch name,
/// ahead/behind counts, and whether the working tree is dirty.
pub fn status(repo: &Path) -> Result<StatusInfo> {
    let out = run_git(repo, &["status", "--porcelain=v2", "--branch"])?;

    let mut branch = None;
    let mut ahead = 0u32;
    let mut behind = 0u32;
    let mut dirty = false;

    for line in out.lines() {
        if let Some(rest) = line.strip_prefix("# branch.head ") {
            branch = Some(rest.to_string());
        } else if let Some(rest) = line.strip_prefix("# branch.ab ") {
            for part in rest.split_whitespace() {
                if let Some(n) = part.strip_prefix('+') {
                    ahead = n.parse().unwrap_or(0);
                } else if let Some(n) = part.strip_prefix('-') {
                    behind = n.parse().unwrap_or(0);
                }
            }
        } else if line.starts_with('#') {
            continue;
        } else if !line.is_empty() {
            dirty = true;
        }
    }

    Ok(StatusInfo {
        branch,
        ahead,
        behind,
        dirty,
    })
}

/// Uses git's own `%ar` relative-time formatting, so we never need a
/// separate time-formatting dependency.
///
/// `git log -1` exits non-zero on a repo with no commits yet (e.g. right
/// after `git init`) - that's a normal state, not a failure, so it's handled
/// here directly rather than through `run_git`'s bail-on-failure path.
pub fn last_commit(repo: &Path) -> Result<CommitInfo> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["log", "-1", "--format=%ar%x1f%s"])
        .env("GIT_TERMINAL_PROMPT", "0")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .with_context(|| format!("failed to spawn `git log` in {}", repo.display()))?;

    if !output.status.success() {
        return Ok(CommitInfo {
            relative: "no commits yet".to_string(),
            message: String::new(),
        });
    }

    let out = String::from_utf8_lossy(&output.stdout);
    let out = out.trim();

    if out.is_empty() {
        return Ok(CommitInfo {
            relative: "no commits yet".to_string(),
            message: String::new(),
        });
    }

    let mut parts = out.splitn(2, '\u{1f}');
    let relative = parts.next().unwrap_or_default().to_string();
    let message = parts.next().unwrap_or_default().to_string();

    Ok(CommitInfo { relative, message })
}

pub fn fetch(repo: &Path) -> Result<()> {
    run_git(repo, &["fetch", "--all", "--prune"]).map(|_| ())
}

pub fn pull_ff_only(repo: &Path) -> Result<()> {
    run_git(repo, &["pull", "--ff-only"]).map(|_| ())
}
