# Gitover

A terminal UI for monitoring multiple git repositories simultaneously.

See [docs/features.md](docs/features.md) for the full feature reference.

## Features (brief)

- Live status for all tracked repos in a single table
  - current branch, ahead/behind upstream and trunk
  - change counter: staged / conflict / modified / deleted / untracked
- Background git operations: fetch, pull, push, force-push, checkout, create & delete branch
- Fetch all repos in parallel (`Alt-f`)
- Status Details pane — per-file change list with priority sorting and scroll indicators
- Commit History pane — full log or filtered ahead/behind upstream & trunk; file sub-rows per commit; `,`/`.` jump between commits
- Details pane — diff of selected file, or full commit details (hash, author, change summary, word-wrapped message) with position indicator
- Output Log pane — timestamped, severity-coloured git command output with auto-follow
- Per-file actions: stage, unstage, revert, discard
- Branches pane — full list of local and remote-only branches with ahead/behind counts; direct checkout and branch action menu
- Help overlay (`?`) — Show available keybindings
- Custom repo commands configurable per project
- File-system watcher for instant refresh (no polling)
- Persistent repo list and pane state across sessions

![screenshot](./docs/screenshot.jpg)

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

Or build and install in one step:

```shell
make install
```

Or install directly from the remote git repo:

```shell
cargo install --git https://github.com/manuel-koch/gitover-rust-tui
```

## Configuration

Config file lookup: searches for `gitover.config.yaml` starting from the current working directory,
walking up to the filesystem root; falls back to `~/.config/gitover/config.yaml`.
A missing file is valid — defaults are used.

```yaml
general:
  git: /usr/local/bin/git        # optional: override git executable path
  auto_fetch_interval: 600       # seconds between background fetches (0 = disabled)
  debug_log: ~/logs/gitover.log  # optional: persistent debug log (supports ~ and ${VAR})

repo_commands:
  - name: Open in editor
    cmd: code ${ROOT}             # ${ROOT} and ${BRANCH} are repo vars; ${HOME} etc. are env vars
    background: true
```

State (repo list, pane visibility) is saved automatically to `~/.config/gitover/state.yaml`
(or a `gitover.state.yaml` found by the same CWD-walk).

## Usage

```shell
gitover [--config <path>] [--state <path>] [--debug-log <path>]
```

Debug logging can also be enabled persistently via `general.debug_log` in the config file; the CLI flag takes precedence when both are set. Both paths support `~` and `${VAR}` expansion — the app terminates if a variable cannot be resolved.

Repo command strings use `${VAR}` substitution: repo variables (`${ROOT}`, `${BRANCH}`) are resolved first, then any remaining references are resolved from the process environment. The command is not run if any variable is unresolvable.

On first launch the repo list is empty. Press `A` to add a repository using the file picker.
If the current working directory is a git repository it is added automatically.

## Keybindings

### Global

| Key         | Action                                                                 |
|-------------|------------------------------------------------------------------------|
| `Ctrl-C`    | Quit                                                                   |
| `Tab`       | Cycle focus forward: Repos → Status Details → History → Diff → Log     |
| `Shift+Tab` | Cycle focus backward                                                   |
| `↑` / `↓`  | Navigate in focused pane                                                |
| `PgUp/Dn`  | Jump 10 rows in focused pane                                            |
| `r`         | Refresh all repositories                                               |
| `Alt-f`     | Fetch all tracked repos in parallel                                    |
| `s`         | Toggle Status Details pane                                             |
| `h`         | Toggle Git History pane                                                |
| `b`         | Toggle Branches pane                                                   |
| `d`         | Toggle Details pane                                                    |
| `l`         | Toggle Output Log pane                                                 |
| `?`         | Open keybinding help overlay                                           |

### Repositories pane

| Key     | Action                                      |
|---------|---------------------------------------------|
| `Enter` | Open per-repo action menu                   |
| `f`     | Fetch selected repo                         |
| `p`     | Pull selected repo                          |
| `P`     | Push selected repo                          |
| `c`     | Checkout branch                             |
| `A`     | Add repository (file picker)                |
| `D`     | Remove selected repository (with confirm)   |

### Branches pane

| Key         | Action                                             |
|-------------|----------------------------------------------------|
| `↑/↓`      | Navigate branches                                   |
| `c`         | Checkout highlighted branch directly               |
| `Enter`     | Open branch action menu                            |
| `b` / `Esc` | Close Branches pane                                |

### Action menu (opened with `Enter`)

| Key   | Action                                            |
|-------|---------------------------------------------------|
| `f`   | Fetch (`git fetch origin --prune`)                |
| `p`   | Pull (auto-stash/pop, `git pull --prune`)         |
| `P`   | Push (sets upstream automatically if needed)      |
| `F`   | Force Push (confirmation dialog)                  |
| `c`   | Checkout Branch (auto-stash/pop)                  |
| `n`   | New Branch (prompts for name)                     |
| `x`   | Delete Branch (select from list)                  |
| `h`   | Commit History (full log)                         |
| `u/U` | Commit History ahead of / behind upstream         |
| `t/T` | Commit History ahead of / behind trunk            |
| `Esc` | Dismiss menu                                      |

### Status Details pane

| Key     | Action                           |
|---------|----------------------------------|
| `Enter` | Open per-file action menu        |
| `↑/↓`  | Select file                       |
| `PgUp/Dn` | Jump 10 files                  |

### Git History pane

| Key             | Action                                          |
|-----------------|-------------------------------------------------|
| `↑/↓`          | Navigate commits and file rows                   |
| `PgUp/Dn`      | Jump 10 rows                                    |
| `Shift-↑` / `,` | Jump to previous commit header row             |
| `Shift-↓` / `.` | Jump to next commit header row                 |
| `h`             | Close history pane                              |

### Details pane

| Key       | Action                  |
|-----------|-------------------------|
| `↑/↓`    | Scroll content           |
| `PgUp/Dn` | Jump 10 lines           |
