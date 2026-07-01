use std::path::PathBuf;
use std::thread;

use color_eyre::eyre::Result;
use crossbeam_channel::{Receiver, Sender};

use crate::{git, scan};

pub enum Job {
    ScanRoot(PathBuf, Option<usize>),
    Status(PathBuf),
    Fetch(PathBuf),
    Pull(PathBuf),
}

pub enum JobResult {
    Scanned(Vec<PathBuf>),
    Status(PathBuf, Result<(git::StatusInfo, git::CommitInfo)>),
    Fetched(PathBuf, Result<()>),
    Pulled(PathBuf, Result<()>),
}

pub fn spawn_pool(count: usize, job_rx: Receiver<Job>, result_tx: Sender<JobResult>) {
    for _ in 0..count {
        let job_rx = job_rx.clone();
        let result_tx = result_tx.clone();
        thread::spawn(move || {
            for job in job_rx {
                if result_tx.send(run_job(job)).is_err() {
                    break;
                }
            }
        });
    }
}

fn run_job(job: Job) -> JobResult {
    match job {
        Job::ScanRoot(root, max_depth) => JobResult::Scanned(scan::find_repos(&root, max_depth)),
        Job::Status(path) => {
            let result = git::status(&path).and_then(|status| {
                git::last_commit(&path).map(|commit| (status, commit))
            });
            JobResult::Status(path, result)
        }
        Job::Fetch(path) => {
            let result = git::fetch(&path);
            JobResult::Fetched(path, result)
        }
        Job::Pull(path) => {
            let result = git::pull_ff_only(&path);
            JobResult::Pulled(path, result)
        }
    }
}
