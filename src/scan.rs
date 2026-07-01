use std::path::{Path, PathBuf};

use walkdir::WalkDir;

/// Finds every git repo under `root`, stopping descent as soon as a `.git`
/// entry is found (no nested-repo discovery in v1).
pub fn find_repos(root: &Path, max_depth: Option<usize>) -> Vec<PathBuf> {
    let mut repos = Vec::new();

    let mut walker = WalkDir::new(root).follow_links(false);
    if let Some(depth) = max_depth {
        walker = walker.max_depth(depth);
    }

    let mut it = walker.into_iter();
    loop {
        let entry = match it.next() {
            Some(Ok(entry)) => entry,
            Some(Err(_)) => continue,
            None => break,
        };

        if !entry.file_type().is_dir() {
            continue;
        }

        if entry.path().join(".git").exists() {
            repos.push(entry.path().to_path_buf());
            it.skip_current_dir();
        }
    }

    repos.sort();
    repos
}
