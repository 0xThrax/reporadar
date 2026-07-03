# reporadar

## What this project is

A live terminal dashboard showing the git status (branch, ahead/behind, dirty,
last commit) of every repo under a directory tree, with fuzzy filtering,
fetch-all, and ff-only pull. Single-user TUI tool, publish-ready for crates.io.

## Stack

Rust (pinned 1.95.0, MSRV 1.82), ratatui + crossterm (TUI), crossbeam-channel
worker pool, nucleo-matcher (fuzzy filter), clap. Git operations shell out to
the `git` CLI; no libgit2.

## Repo layout

- `src/lib.rs` — exposes all modules so tests drive the real pipeline
- `src/main.rs` — thin binary: CLI parse, event loop
- `src/app.rs` — all state + key handling (App is the unit under test)
- `src/worker.rs` — background job pool (scan/status/fetch/pull)
- `src/git.rs` — git CLI wrappers (porcelain v2 parsing)
- `src/ui/` — ratatui rendering (pure functions of &App)
- `tests/e2e.rs` — real git repos in tempdirs + TestBackend rendering

## Key concepts and terminology

- **Jobs/JobResults**: all slow work goes through the worker channel; App is
  single-threaded and only mutates in `apply()` and `on_key()`.
- **filtered**: indices into `repos` after fuzzy filtering; selection operates
  on filtered space.
- **inflight**: pending job count; the tests' settle loop pumps until 0.

## How to run locally

`just run` (scans ~/Work) or `cargo run -- <dir>`.

## How to test end-to-end

`just test` — creates real git repos in tempdirs, runs them through the real
worker pool, and asserts on TestBackend-rendered frames. Full gate: `just ci`.

## Conventions specific to this project

- UI functions take `&App` and must stay side-effect free.
- New key bindings go in `App::on_key_normal/on_key_filter` plus an e2e test.
- Errors surface in `RepoStatus.error` and the status line; never panic in
  the event loop.
