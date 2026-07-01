# reporadar

A live, interactive terminal dashboard for the status of every git repo under a directory tree.

If you've got dozens of project folders and dotfiles scattered around your home directory, `reporadar` gives you a single screen showing which ones are dirty, ahead/behind their remote, and when they were last touched — with one keypress to fetch everything, pull a fast-forward, or drop into [lazygit](https://github.com/jesseduffield/lazygit) for the ones that need attention.

```
┌─ reporadar — ~/projects ─────────────────────────────────────────────────┐
│ Repo          Branch   Status      Ahead/Behind  Last Commit             │
│ ▶ api-server  main     ● dirty     ↑0 ↓2         2 hours ago  fix bug    │
│   dotfiles    main     ✓ clean     ↑1 ↓0         yesterday    add plan   │
│   website     master   ✓ clean     ↑0 ↓0         3 days ago   bump deps  │
└────────────────────────────────────────────────────────────────────────┘
 ↑/↓ move  r rescan  f fetch-all  p pull  Enter open  / filter  q quit
```

## Install

```sh
cargo install --path .
```

or clone and run directly:

```sh
cargo run --release -- ~/projects
```

## Usage

```sh
reporadar [PATH]            # scan PATH (default: current directory)
reporadar --max-depth 3 ~   # limit how deep the scan walks
```

`reporadar` walks the given directory looking for anything containing a `.git` — it stops descending as soon as it finds one, so it won't waste time indexing every file inside your repos.

### Keybindings

| Key         | Action                                                        |
|-------------|----------------------------------------------------------------|
| `↑`/`↓`, `j`/`k` | Move selection                                            |
| `r`         | Rescan the directory tree                                      |
| `f`         | Fetch all visible repos in the background (non-blocking)       |
| `p`         | Pull the selected repo (fast-forward only)                     |
| `Enter`     | Suspend the dashboard and open the selected repo in `lazygit` (falls back to `git status` if lazygit isn't installed) |
| `/`         | Fuzzy-filter repos by name                                      |
| `Esc`       | Clear the filter, or quit if not filtering                      |
| `q` / `Ctrl+C` | Quit                                                          |

## How it works

`reporadar` shells out to the system `git` binary rather than linking `libgit2` — a single `git status --porcelain=v2 --branch` call gives branch, ahead/behind counts, and dirty state in one pass, and `git log -1 --format=...` supplies the last commit's relative time and message using git's own formatting. This keeps the build free of native/OpenSSL dependencies and reuses whatever credential helpers (SSH agent, Windows Credential Manager, etc.) your system git is already configured with.

Background work (status scans, fetches, pulls) runs on a small pool of OS threads so the UI never blocks on network I/O; background fetches set `GIT_TERMINAL_PROMPT=0` so an auth-walled remote fails fast instead of hanging.

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option.
