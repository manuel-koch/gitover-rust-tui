# Gitover

A terminal UI for monitoring multiple git repositories simultaneously.

See [docs/features.md](docs/features.md) for the full feature reference.

## Features (brief)

- Live status for all tracked repos in a single table
  - current branch
  - change counter for staged / conflict / modified / deleted / untracked files
  - ahead / behind counter for upstream and trunk
- Background git operations: fetch, pull, push, force-push, checkout branch, create branch, delete branch
- Fetch all repos in parallel
- Status Details pane — per-file change list with priority sorting and scroll
- Output Log pane — timestamped git command output with auto-follow
- File-system watcher for instant refresh (no polling)
- Persistent repo list and recent-repo history across sessions
- Config file for custom git path

![screenshot](screenshot.jpg)

## Build & Install

Requirements: Rust toolchain (stable, 1.70+)

```shell
git clone <repo-url>
cd gitover-rust-tui
cargo build --release
```

The binary is at `target/release/gitover`. Copy it anywhere on your PATH:

```shell
cp target/release/gitover ~/.local/bin/gitover
```

Or run directly without installing:

```shell
cargo run --release
```

Or build executable locally from the remote git repo and install it

```shell
cargo install --git https://github.com/manuel-koch/gitover-rust-tui
```

## Configuration

Gitover reads its config from `~/.config/gitover/config.yaml`.

The file is optional — a missing or empty file is valid. Example:

```yaml
general:
  git: /usr/local/bin/git   # optional: override git executable path
```

State (repo list, recents) is saved automatically to `~/.config/gitover/state.yaml`.

## Usage

```shell
gitover
```

On first launch the repo list is empty. Press `A` to add a repository using the file picker.

## Keybindings

### Global

| Key       | Action                                      |
|-----------|---------------------------------------------|
| `Ctrl-C`  | Quit                                        |
| `Tab`     | Cycle focus: Repositories / Status Details / Output Log |
| `r`       | Refresh all repositories                    |
| `Alt-f`   | Fetch all tracked repos in parallel         |

### Repositories pane

| Key           | Action                                  |
|---------------|-----------------------------------------|
| `↓`           | Move cursor down                        |
| `↑`           | Move cursor up                          |
| `PgDn`        | Jump 10 rows down                       |
| `PgUp`        | Jump 10 rows up                         |
| `Enter`       | Open per-repo action menu               |
| `f`           | Fetch selected repo                     |
| `p`           | Pull selected repo                      |
| `P`           | Push selected repo                      |
| `c`           | Checkout branch (opens branch list)     |
| `A`           | Add repository (file picker)            |
| `D`           | Remove selected repository              |
| `s`           | Toggle Status Details pane              |
| `l`           | Toggle Output Log pane                  |

### Action menu (opened with Enter)

| Key   | Action                                              |
|-------|-----------------------------------------------------|
| `f`   | Fetch (`git fetch origin --prune`)                  |
| `p`   | Pull (auto-stash/pop, `git pull --prune`)           |
| `P`   | Push (sets upstream automatically if needed)        |
| `F`   | Force Push (confirmation dialog shown first)        |
| `c`   | Checkout Branch (auto-stash/pop)                    |
| `n`   | New Branch (prompts for name)                       |
| `x`   | Delete Branch (select from list)                    |
| `Esc` | Dismiss menu                                        |

### Status Details / Output Log panes

| Key       | Action              |
|-----------|---------------------|
| `↓`       | Scroll down        |
| `↑`       | Scroll up          |
| `PgDn`    | Jump 10 lines down  |
| `PgUp`    | Jump 10 lines up    |
