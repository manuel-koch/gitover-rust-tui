# Gitover TUI Features

## General

- Rust-based terminal UI application
- Tracks multiple git repositories simultaneously
- Configuration loaded from `.gitover` YAML file, searched upward from `$HOME`
- Persisted state (repo list, recents) stored at `~/.config/gitover/state.yaml`

## Configuration

- Config file format: YAML at `~/.config/gitover/config.yaml`
- `general.git`: override the path to the git executable
- `repo_commands`: named custom commands to run per repo
- `status_commands`: named commands scoped to a specific file status

## Repository Management

- Add a repository with `A`; opens a directory-browser to choose the repo root
  - Also accepts a plain path typed directly
  - Auto-discovers and adds git submodules when a repo is added
  - Recently-used repos are offered for quick re-add
- Remove a repository with `D`; shows a confirmation dialog before removing
- Repo list is kept sorted by absolute path
- Repo list and recents are persisted across sessions
- Invalid or missing repo paths are shown inline as error rows instead of being silently dropped

## Repository Table

Each tracked repository is shown as a table row with:

- **Repository**: directory name, green when working tree is clean
- **Branch**: current branch name, or `detached <sha8>` for detached HEAD
- **Status**: combined change counts — `3-S 2-C 4-M 1-D 2-U` (S=staged blue, C=conflict yellow, M=modified green, D=deleted red, U=untracked gray); shows `clean` in dark gray when no changes
- **Activity**: spinner + operation name when a git operation is in progress (fetching / pulling / pushing / rebasing / scanning)
- **↑↓ Upstream**: ahead/behind vs configured tracking branch, yellow when out of sync
- **↑↓ Trunk**: ahead/behind vs trunk branch, red when behind, yellow when ahead only
  - Trunk resolution order: `gitover.trunkbranch` config → `origin/main` → `origin/develop` → `origin/master`
- Column widths are distributed so branch/upstream/trunk columns are wider than status

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
| `Q` | Quit |
| `j` / `k` or `↑` / `↓` | Navigate up/down in focused pane |
| `PgUp` / `PgDn` (Fn-Up/Down) | Jump 10 rows; clamps at list boundaries, no wrap |
| `Tab` | Cycle focus between Repositories / Status Details / Output Log panes |
| `A` | Add repository |
| `D` | Remove selected repository (with confirmation) |
| `s` | Toggle Status Details pane |
| `l` | Toggle Output Log pane |
| `r` | Refresh all repositories |
| `Ctrl-C` | Quit (works in all modes) |

## User Interface

- Three-pane layout (vertical): Repositories table / Status Details / Output Log
- Status Details and Output Log are optional; shown only when toggled open
- Focused pane highlighted with cyan border; unfocused panes use dark-gray border
- Loading spinner in header while repos are being scanned
- Refresh timestamp shown right-aligned in the header bar
- Single-line help bar at the bottom showing active key bindings
- Confirmation dialog for destructive actions (remove repo)
- File picker popup for adding repos with vim-style navigation (`j`/`k`/`h`/`l`)

## Branch Information (per repo, available for future use)

- Full list of local branches
- Remote branches not yet checked out locally
- Local branches already merged into the trunk branch
