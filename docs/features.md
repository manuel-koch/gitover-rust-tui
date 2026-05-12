# Gitover TUI Features

## General

- Rust-based terminal UI application
- Tracks multiple git repositories simultaneously
- Configuration loaded from `~/.config/gitover/config.yaml` (optional; missing file is valid)
- Persisted state (repo list, recents) stored at `~/.config/gitover/state.yaml`

## Configuration

- Config file: `~/.config/gitover/config.yaml` (optional; missing file is valid)
- `general.git`: override the path to the git executable

## Repository Management

- Add a repository with `A`; opens a directory-browser to choose the repo root
  - `↑`/`↓` navigate the directory list
  - `→` / `Enter` navigates into the selected directory
  - `←` / `Backspace` goes to the parent directory
  - `Space` confirms the current directory as the repo to add — this allows adding a
    child repo even when its parent directory is itself a git repo
  - Auto-discovers and adds git submodules when a repo is added
  - Recently-used repos are offered for quick re-add
- Remove a repository from the app ( not from disk ! ) with `D`; shows a confirmation dialog before removing
- Repo list is kept sorted by absolute path
- Repo list and recents are persisted across sessions
- Invalid or missing repo paths are shown inline as error rows instead of being silently dropped

## Repository Table

Each tracked repository is shown as a table row with:

- **Repository**: directory name, green when working tree is clean
- **Branch**: current branch name, or `detached <sha8>` for detached HEAD; unborn
  branches (no commits yet) show the branch name correctly
- **Status**: combined change counts — `3-S 2-C 4-M 1-D 2-U` (S=staged blue, C=conflict yellow, M=modified green, D=deleted red, U=untracked gray); shows `clean` in dark gray when no changes
- **Activity**: spinner + operation name when a git operation is in progress (fetching / pulling / pushing / rebasing / scanning)
- **↑↓ Upstream**: ahead/behind vs configured tracking branch, yellow when out of sync
- **↑↓ Trunk**: ahead/behind vs trunk branch, red when behind, yellow when ahead only
  - Trunk resolution order: `gitover.trunkbranch` config → `origin/main` → `origin/develop` → `origin/master`
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

`Alt-f` fetches all tracked repositories in parallel (global shortcut, works from any pane).

All git operations run in the background. Progress is shown via the Activity column spinner.
Output lines (stdout + stderr) are appended to the Output Log pane with timestamps.

## Status Details Pane

- Toggle with `s`; title shows "Status Details — <repo path>"
- Lists each changed file with a single-letter status code (C/S/M/D/U) in its status colour followed by the file path
- Files sorted by priority: Conflict → Staged → Modified → Deleted → Untracked, then alphabetically within each group
- Scrolls when file count exceeds panel height; cursor always stays visible
- Tab focus moves to this pane when opened; Tab cycles back to Repositories

## Output Log Pane

- Toggle with `l`
- Shows timestamped lines from git command output in local time (`HH:MM:SS`)
- Auto-follows new entries (scrolls to tail) when cursor is at the last visible line
- When pane is not focused, always shows the tail (latest entries)
- User can scroll up into history; scrolling back to tail re-enables auto-follow
- Automatically shown when a git operation fails, so error output is immediately visible

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
  - Git-aware filter: watches relevant `.git/` files (HEAD, refs, index, COMMIT_EDITMSG, rebase state, etc.) while ignoring noisy internals (objects, pack files, etc.)
  - 500 ms debounce prevents spurious updates during rapid saves
- Wake-from-sleep detection: if a tick gap exceeds 3 s the system likely woke from sleep; a full refresh fires to catch missed events
- No unconditional background polling — the file watcher handles real-time updates
- Manual refresh with `r` key available from any pane

## Navigation & Keyboard

| Key | Action |
|-----|--------|
| `↑` / `↓` | Navigate up/down in focused pane |
| `PgUp` / `PgDn` (Fn-Up/Down) | Jump 10 rows; clamps at list boundaries, no wrap |
| `Tab` | Cycle focus between Repositories / Status Details / Output Log / Git History panes |
| `A` | Add repository (opens file picker) |
| `D` | Remove selected repository (with confirmation) |
| `Enter` | Open per-repo action menu |
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
- Loading spinner in header while repos are being scanned
- Refresh timestamp shown right-aligned in the header bar
- Single-line help bar at the bottom showing active key bindings
- Confirmation dialogs for destructive actions (remove repo, force push)
- File picker popup for adding repos; `↑`/`↓` navigate, `→`/`Enter` descend into directory, `←`/`Backspace` go to parent, `Space` selects current directory as repo to add
- Per-repo action menu popup (opened with `Enter`); dismissed with `Esc`

## Branch Information (per repo, available for future use)

- Full list of local branches
- Remote branches not yet checked out locally
- Local branches already merged into the trunk branch

## Tooling

- `Makefile` at the project root with the following targets:
  - `make lint` — runs `cargo clippy`
  - `make format` — runs `cargo fmt`
  - `make build-and-run` — builds and launches the app via `cargo run`
  - `make test` — runs all unit and integration tests via `cargo test`
