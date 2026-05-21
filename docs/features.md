# Gitover TUI Features

## General

- Rust-based terminal UI application
- Tracks multiple git repositories simultaneously

## CLI options/flags

| Flag | Description |
|------|-------------|
| `--version` | Show version & built info and exit |
| `--config <path>` | Override the config file location (skips CWD-walk and global fallback) |
| `--state <path>` | Override the state file location (skips CWD-walk and global fallback); file is created on first save if absent |

## Configuration

- Config file lookup: searches for `gitover.config.yaml` starting from the current working directory
  and walking up to the filesystem root; falls back to `~/.config/gitover/config.yaml` if not found.
  Missing file is valid — default config is used.
- `general.git`: override the path to the git executable
- `general.auto_fetch_interval`: interval in seconds for automatic background fetch of all repos
  (default: 600 = 10 minutes; set to 0 to disable automatic fetch)
- `repo_commands`: list of commands that can be run for current repository
  - `repo_commands[].name`: Description of the command, will be shown in action menu
  - `repo_commands[].cmd`: The command line to be executed, supports variable expansion like `$ROOT` ( repo git root path ), `$BRANCH` ( current git branch name )
  - `repo_commands[].background`: Boolean flag whether the `cmd` should be executed in background
- Persisted app state (repo list, pane visibility):
  - State file lookup: searches for `gitover.state.yaml` starting from CWD and walking up to root;
    falls back to `~/.config/gitover/state.yaml` if not found.
  - Relative paths in the state file are resolved against the directory containing the state file.
  - When saving, repo paths that are under the state file's directory are stored as relative paths,
    keeping per-project state files portable.

## Repository Management

- Add a repository with `A`; opens a directory-browser to choose the repo root
  - `↑`/`↓` navigate the directory list
  - `→` / `Enter` navigates into the selected directory
  - `←` / `Backspace` goes to the parent directory
  - `Space` confirms the current directory as the repo to add — this allows adding a
    child repo even when its parent directory is itself a git repo
  - Auto-discovers and adds git submodules when a repo is added
- Remove a repository from the app ( not from disk ! ) with `D`; shows a confirmation dialog before removing
- Repo list is kept sorted by absolute path
- Repo list is persisted across sessions
- Invalid or missing repo paths are shown inline as error rows instead of being silently dropped

## Repository Table

Each tracked repository is shown as a table row with:

- **Repository**: directory name, green when working tree is clean
- **Branch**: current branch name, or `detached <sha8>` for detached HEAD; unborn
  branches (no commits yet) show the branch name correctly
- **Status**: combined change counts
  - `3-S 2-C 4-M 1-D 2-U` (S=staged blue, C=conflict yellow, M=modified green, D=deleted red, U=untracked gray)
  - Shows `clean` in dark gray when no changes
- **Activity**: spinner + operation name when a git operation is in progress (fetching / pulling / pushing / rebasing / scanning)
- **↑↓ Upstream**: ahead/behind vs configured tracking branch, yellow when out of sync
- **↑↓ Trunk**: ahead/behind vs trunk branch, red when behind, yellow when ahead only
  - Trunk resolution order:
    - git config `gitover.trunkbranch`
    - `origin/main`
    - `origin/develop`
    - `origin/master`
- Column widths are distributed so branch/upstream/trunk columns are wider than status

## Git Operations

Pressing `Enter` on a selected repository opens the per-repo action menu. The menu
lists all available actions with their shortcut key. Dismiss with `Esc`.

| Key (in menu) | Action |
|---------------|--------|
| `f` | Fetch — runs `git fetch origin --prune`; triggers a status refresh on completion |
| `p` | Pull — runs `git pull --prune`; auto-stashes dirty changes before pull, pops stash afterwards |
| `P` | Push — pushes current branch; automatically sets upstream (`--set-upstream origin HEAD`) if not configured |
| `F` | Force Push — pushes with `--force --set-upstream origin HEAD` (confirmation dialog shown first) |
| `c` | Checkout Branch — shows a list of local and remote branches; auto-stashes dirty changes before checkout, pops stash afterwards |
| `n` | Create New Branch — prompts for a branch name (input is sanitised), runs `git checkout -b <name>` |
| `x` | Delete Branch — shows list of local branches (excluding current); runs `git branch -D <name>` |
| `H` | Commit History — opens the history pane for the selected repo (full log) |
| `u` / `U` | Commit History ahead of / behind upstream (only shown when upstream is configured) |
| `t` / `T` | Commit History ahead of / behind trunk (only shown when trunk branch is resolvable) |

Direct shortcuts `f`, `p`, `P`, `c` also work from the normal Repositories view without opening the menu.

### Custom Repo Commands

Entries from `repo_commands` config are appended to the per-repo action menu below a separator line, after all built-in actions. Digit keys `1`–`9` (then `0`) are assigned in declaration order. Each command:

- Runs with the working directory set to the repo's git root
- Expands `$ROOT` (git root path) and `$BRANCH` (current branch name) in the command string before execution
- Appends its output to the Output Log pane on completion
- If `background: true`, is spawned without waiting and its output is discarded

`Alt-f` fetches all tracked repositories in parallel (global shortcut, works from any pane).

All git operations run in the background. Progress is shown via the Activity column spinner.
Output lines (stdout + stderr) are appended to the Output Log pane with timestamps.

## Status Details Pane

- Toggle with `s`; title shows "Status Details — <repo path>"
- Lists each changed file with a single-letter status code (C/S/M/D/U) in its status colour followed by the file path
- Files sorted by priority: Conflict → Staged → Modified → Deleted → Untracked, then alphabetically within each group
- Scrolls when file count exceeds panel height; cursor always stays visible
- Tab focus moves to this pane when opened; Tab cycles back to Repositories

## Per-file Actions

Pressing `Enter` or double-clicking a file in the Status Details pane opens the per-file action menu.
Available actions depend on the file's current git status:

| File status | Actions |
|-------------|---------|
| Staged | **Unstage file** — `git reset -- <path>` |
| Modified | **Stage file** — `git add -- <path>`; **Revert file** — `git checkout -- <path>` |
| Deleted | **Stage deletion** — `git add -- <path>`; **Revert file** — `git checkout -- <path>` |
| Conflict | **Revert file** — `git reset -- <path>` followed by `git checkout -- <path>` |
| Untracked | **Stage file** — `git add -- <path>`; **Discard file** — deletes the file from disk |

Dismiss the menu with `Esc` or by clicking outside it.

## Output Log Pane

- Toggle with `l`
- Shows timestamped lines from git command output in local time (`HH:MM:SS`)
- Auto-follows new entries (scrolls to tail) when cursor is at the last visible line
- When pane is not focused, always shows the tail (latest entries)
- User can scroll up into history; scrolling back to tail re-enables auto-follow
- Automatically shown when a git operation fails, so error output is immediately visible
- Pressing `Enter` when the Output Log pane has focus opens the log action menu
  - Menu entry "Copy log output" copies the entire log content to system clipboard
  - After copying, shows a transient popup notification "Log output copied to clipboard!" that auto-dismisses after 2 seconds

## Git History Pane

- Toggle with `h`; also opened via action menu entries `H` / `u` / `U` / `t` / `T`
- Title shows repo name and active filter (e.g. "ahead of origin/main")
- Displays commit history for the current branch, newest commit first, up to 200 commits
- Table columns: short hash (8 chars, yellow) | timestamp (YYYY-MM-DD HH:MM:SS local, gray) | author (cyan, up to 20 chars) | commit message (first line)
  - Column widths are distributed: author column sized to the widest name in the loaded history; summary takes all remaining space
- Each commit row is followed by file sub-rows indented in the summary column:
  - Format: `  <change-identifier> <path>`
  - A = added (blue), M = modified (green), D = deleted (red), R = renamed (yellow)
- `↑`/`↓` and `PgUp`/`PgDn` scroll through commits and file rows
- Commit counter shown top-right of the pane (e.g. `3/47`) based on commit index, not flat row index
- History reloads automatically when the selected repo changes while the pane is open
- History reloads automatically after a git operation completes on the shown repo
- Filtered views available from the action menu:
  - Ahead of upstream / trunk — commits in HEAD not yet in the remote ref
  - Behind upstream / trunk — commits in the remote ref not yet merged locally
- `h` closes the pane; `Tab` cycles focus between panes without closing it

## Real-time Updates

- File system watcher detects changes and refreshes the affected repo instantly
  - Watches the entire working tree: any file creation, modification, or deletion triggers a refresh
  - Git-aware filter: watches relevant `.git/` files (HEAD, refs, index, COMMIT_EDITMSG, rebase state, etc.) while ignoring noisy internals (objects, pack files, etc.)
  - 500 ms debounce prevents spurious updates during rapid saves
- Wake-from-sleep detection: if a tick gap exceeds 3 s the system likely woke from sleep; a full refresh fires to catch missed events
- Automatic background fetch of all tracked repos every 10 minutes; manual `Alt-f` resets the timer
- No unconditional background polling — the file watcher handles real-time updates
- Manual refresh with `r` key available from any pane

## Navigation & Keyboard

| Key | Action |
|-----|--------|
| `↑` / `↓` | Navigate up/down in focused pane |
| `PgUp` / `PgDn` (Fn-Up/Down) | Jump 10 rows in focused pane or action menu; clamps at list boundaries, no wrap |
| `Tab` / `Shift+Tab` | Cycle focus forward / backward between Repositories / Status Details / Output Log / Git History / Diff panes |
| `A` | Add repository (opens file picker) |
| `D` | Remove selected repository (with confirmation) |
| `Enter` | Open per-repo action menu (Repositories pane); open per-file action menu (Status Details pane); open log action menu (Output Log pane) |
| `f` | Fetch selected repo (shortcut, no menu needed) |
| `p` | Pull selected repo (shortcut, no menu needed) |
| `P` | Push selected repo (shortcut, no menu needed) |
| `c` | Checkout branch on selected repo (shortcut, no menu needed) |
| `h` | Toggle Git History pane |
| `Alt-f` | Fetch all tracked repos in parallel |
| `s` | Toggle Status Details pane |
| `l` | Toggle Output Log pane |
| `r` | Refresh all repositories |
| `Ctrl-C` | Quit (works in all modes) |

In the action menu, `Esc` dismisses the menu without taking any action.

## User Interface

- Four-pane layout (vertical): Repositories / Status Details / Output Log / Git History
- Status Details, Output Log, and Git History are optional; shown only when toggled open
- Focused pane highlighted with cyan border; unfocused panes use dark-gray border
- App version shown in the header title (e.g. `Git Repository Overview  v0.1.0`)
- Loading spinner in header while repos are being scanned
- Refresh timestamp shown right-aligned in the header bar
- Auto-fetch countdown shown right-aligned in the header bar (e.g. "fetching all in 30s"; hidden when auto-fetch is disabled)
- Single-line help bar at the bottom showing active key bindings; keybinding hints for navigation
  (`Tab`, `↑↓`, `PgUp`/`PgDn`) are hidden when horizontal space is too small
- Confirmation dialogs for destructive actions (remove repo, force push)
- File picker popup for adding repos
  - `↑`/`↓` navigate in list
  - `→`/`Enter` descend into directory
  - `←`/`Backspace` go to parent
  - `Space` selects current directory as repo to add
- Per-repo action menu popup (opened with `Enter`); dismissed with `Esc`

## Mouse Interaction

- Left-click on a pane sets focus to that pane
- Mouse wheel scrolls the content of the currently focused pane
- Left-click inside the Status Details pane selects the file under the cursor
- Left-click inside the History pane selects the commit/change under the cursor
- Double-click on a repository row opens the per-repo action menu (same as `Enter`)
- Double-click on a file row in the Status Details pane opens the per-file action menu (same as `Enter`)
- Left-click on an action menu entry executes the selected action
- Clicking outside the action menu dismisses it, same as pressing `Esc`

## Branch Information (per repo)

- Full list of local branches
- Remote branches not yet checked out locally
- Local branches already merged into the trunk branch

## Release Info

The binary embeds build metadata at compile time via `build.rs`:

- **Version**: taken from `Cargo.toml` (`CARGO_PKG_VERSION`)
- **Git commit**: short hash of HEAD at build time (`GIT_SHORT_HASH`)
- **Build timestamp**: UTC date/time captured when `cargo build` runs (`BUILD_TIMESTAMP`)

Running `gitover --version` (or `-V`) prints this info and exits immediately without starting the TUI:

```
gitover v0.1.0 (commit abc1234, built 2026-05-20 11:51:06 UTC)
```

## Tooling

- `Makefile` at the project root with the following targets:
  - `make lint` — runs `cargo clippy`
  - `make format` — runs `cargo fmt`
  - `make build-and-run` — builds and launches the app via `cargo run`
  - `make test` — runs all unit and integration tests via `cargo test`
  - `make release` — builds an optimized release binary (`target/release/gitover`)
