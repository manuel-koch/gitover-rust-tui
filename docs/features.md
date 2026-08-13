# Gitover TUI Features

## General

- Rust-based terminal UI application
- Tracks multiple git repositories simultaneously

![screenshot](./screenshot.jpg)

## Contents

- [Configuration](#configuration)
- [Repository Management](#repository-management)
- [Repository Table](#repository-table)
- [Status letter legend](#status-letter-legend)
- [Git Operations](#git-operations)
  - [Custom Repo Commands](#custom-repo-commands)
- [Status Details Pane](#status-details-pane)
- [Per-file Actions](#per-file-actions)
- [Output Log Pane](#output-log-pane)
- [Debug Logging](#debug-logging)
- [Git History Pane](#git-history-pane)
  - [Per-commit Action Menu](#per-commit-action-menu)
- [Details Pane](#details-pane)
- [Real-time Updates](#real-time-updates)
- [Navigation & Keyboard](#navigation--keyboard)
- [User Interface](#user-interface)
- [Mouse Interaction](#mouse-interaction)
- [Git Branches Pane](#git-branches-pane)
  - [Per-branch Action Menu](#per-branch-action-menu)
- [Release Info](#release-info)
- [Release Notifications](#release-notifications)

## Configuration

See [Configuration](./configuration.md) for CLI flags, `general.*` options,
`repo_commands`, and the persisted state file. Both files have JSON Schemas
in [`docs/config.schema.json`](./config.schema.json) and
[`docs/state.schema.json`](./state.schema.json).

## Repository Management

- Repositories can be organised into named **sections** in the Repositories pane:
  - A default (unnamed) section always exists, is shown first, and cannot be renamed,
    removed, or collapsed
  - Named sections appear in case-insensitive alphabetical order; their repos are
    indented two spaces and sorted by absolute path within the section.
    Per-section repo-path sort honors `general.case_sensitive_path_sorting`
    (default: case-insensitive)
  - Section title rows show `▶` (collapsed) or `▼` (expanded); `←` collapses a
    named section, `→` expands it, and left-clicking the `▶`/`▼` indicator toggles
    collapse/expand; collapse state is stored per section in the state file
  - A collapsed section shows an aggregated summary row (dirty count, upstream/trunk
    divergence, active operations); the summary is hidden when the section is expanded
  - When a section-title row is selected, the Status Details pane shows a
    "No repository selected" placeholder and the History pane is cleared
  - The Repositories pane title shows context about the current selection:
    - `Repositories ( <section> - <repo> )` when a repo in a named section is selected
    - `Repositories ( <section> )` when only a named-section row is selected
    - `Repositories ( <repo> )` when a repo in the default section is selected
    - `Repositories` when nothing is selected
  - `f` on a section-title row fetches all repos in that section in parallel
  - `r` on a section-title row refreshes all repos in that section in parallel
  - Each repo shows a `scan` spinner in the Activity column while a refresh is
    in progress (applies to both single-repo `r` and section-level `r`)
- Section management actions (available via action menu when a section-title
  row is selected):
  - **Create Section** — prompts for a name; duplicate check is case-insensitive;
    newly created section is selected
  - **Rename Section** — prompts for a new name (not available for the default section;
    duplicate check applied); renamed section remains selected
  - **Remove Section** — shows confirmation dialog; all repos are moved to the default
    section; not available for the default section
- Repository section actions (available via action menu when a repo row is selected):
  - **Move to Section** — presents all sections except the current one (default section
    listed first); not shown when only a default section exists; moved repo stays
    selected in the target section
- When adding a repository with `A`, it is added to the currently selected section;
  if no default-section repos exist, the picker shows a hint about how to move the
  repo to the default section afterwards
- Add a repository with `A`; opens a directory-browser starting at the current
  working directory
  - `↑`/`↓` navigate the directory list
  - `→` / `Enter` navigates into the selected directory
  - `←` / `Backspace` goes to the parent directory
  - `Space` adds the current directory as a tracked repo and keeps the picker open,
    allowing multiple repos to be added in one session; this also lets you add a
    child repo even when its parent directory is itself a git repo
  - Hint bar at the bottom of the picker progressively hides navigation groups when the
    terminal is too narrow, always keeping the `Space` / `Esc` hints visible
  - Auto-discovers and adds git submodules when a repo is added
  - Newly added repo is immediately selected in the Repositories pane
- Remove a repository from the app ( not from disk ! ) with `D`; shows a
  confirmation dialog before removing
- Each section's repos are kept sorted by absolute path; case-insensitive by default,
  configurable via `general.case_sensitive_path_sorting`. Section names are always
  sorted case-insensitively and are not affected by this setting.
  The Repositories pane is not a single flat sorted list — repos are grouped
  under their section, and the default (unnamed) section is always shown first.
- Repo list is persisted across sessions
- On first launch with an empty state file, the current working directory is automatically
  added as a tracked repo if it is a git repository
- Invalid or missing repo paths are shown as error rows: repo name in the first column,
  full error message spanning all remaining columns
- Bare repositories are rejected when adding: an error is logged immediately
  and the path is not tracked

## Repository Table

Each tracked repository is shown as a table row with:

- **Repository**: directory name, green when working tree is clean
- **Branch**: current branch name, or `detached <sha8>` for detached HEAD; unborn
  branches (no commits yet) show the branch name correctly
- **Status**: combined change counts
  - `3-S 2-C 4-M 1-D 2-U` (see [Status letter legend](#status-letter-legend))
  - Shows `clean` in dark gray when no changes
- **Activity**: spinner + operation name when a git operation is in progress
  (fetching / pulling / pushing / rebasing / scanning)
- **↑↓ Upstream**: ahead/behind vs configured tracking branch, yellow when out of sync
- **↑↓ Trunk**: ahead/behind vs trunk branch, red when behind, yellow when ahead only
  - Trunk resolution order:
    - git config `gitover.trunkbranch`
    - `origin/main`
    - `origin/develop`
    - `origin/master`
- Column widths are distributed so branch/upstream/trunk columns are wider than status
- Scroll indicators (▲ / ▼) appear at the top-right / bottom-right of the table when the
  repo list overflows the visible area

Section-title rows appear above each named section's repositories:

- Repository column: section name with `▶` (collapsed) or `▼` (expanded) prefix;
  repos are indented two spaces
- When collapsed, the remaining columns show an aggregated summary:
  - **Status**: `N dirty` (dirty colour) when any repo has local changes, otherwise `clean`
  - **Activity**: spinner + operation label (or `N active`) when any repos have
    running operations
  - **↑↓ Upstream**: `N ↑↓` (warning colour) when any repo has upstream divergence,
    otherwise `–`
  - **↑↓ Trunk**: `N ↑↓` when any repo diverges from trunk, otherwise `–`
- When expanded, per-repo rows show full individual details

## Status letter legend

The Repositories pane Status column and the Status Details pane both use
single-letter status codes with the same colour coding:

| Letter | Meaning | Colour |
|--------|---------|--------|
| `S` | Staged | blue |
| `C` | Conflict | yellow |
| `M` | Modified | green |
| `D` | Deleted | red |
| `U` | Untracked | gray |

These letters are used in the combined Status column (`3-S 2-C 4-M 1-D 2-U`)
and as the per-file marker in the Status Details pane.

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
| `n` | Create Branch — prompts for a branch name (input is sanitised), runs `git checkout -b <name>` |
| `a` | Amend Commit — opens the amend dialog pre-filled with the HEAD message; available even when no file is staged |
| `h` | Commit History — opens the history pane for the selected repo (full log) |
| `u` / `U` | Commit History ahead of / behind upstream (only shown when upstream is configured) |
| `t` / `T` | Commit History ahead of / behind trunk (only shown when trunk branch is resolvable) |
| `D` | Remove Repo from App — removes the selected repo from the app (with confirmation) |

Direct shortcuts `f`, `p`, `P`, `c` also work from the normal Repositories view without opening the menu.
The `D` (remove repo) shortcut also works directly from the Repositories view without opening the menu.
See [Keybindings → Repositories pane](./keybindings.md#repositories-pane) for the full set of direct shortcuts.

### Custom Repo Commands

Entries from `repo_commands` config are appended to the per-repo action menu below a
separator line, after all built-in actions. Digit keys `1`–`9` (then `0`) are assigned
in declaration order. Each command:

- Runs with the working directory set to the repo's git root
- Variable substitution uses `${VAR}` syntax only, applied in two steps:
  1. Repo variables: `${ROOT}` (git root path), `${BRANCH}` (current branch name)
  2. Environment variables: any remaining `${VAR}` references resolved from the
     process environment
- Command is not executed if any variable cannot be resolved; an error is logged for
  each unknown variable
- Appends its output to the Output Log pane on completion
- If `background: true`, is spawned without waiting and its output is discarded

`Alt-f` fetches all tracked repositories in parallel (global shortcut, works from any pane).

All git operations run in the background. Progress is shown via the Activity column spinner.
Output lines (stdout + stderr) are appended to the Output Log pane with timestamps.

## Status Details Pane

- Toggle with `s`; title shows "File Status — <repo path>"
- When the repo path is too wide to fit the title area, the path is truncated
  in the middle with a `...` ellipsis (e.g. `/home/user/.../project`)
- Lists each changed file with a single-letter status code (see
  [Status letter legend](#status-letter-legend)) in its status colour followed
  by the file path
- Files sorted by priority: Conflict → Staged → Modified → Deleted → Untracked, then
  alphabetically within each group; case-insensitive by default, configurable
  via `general.case_sensitive_path_sorting`
- Scrolls when file count exceeds panel height; cursor always stays visible
- Scroll indicators (▲ / ▼) appear when content overflows above or below the
  visible area; coloured with focused/unfocused border colour
- Tab focus moves to this pane when opened; Tab cycles back to Repositories

## Per-file Actions

Pressing `Enter` or double-clicking a file in the Status Details pane opens
the per-file action menu.
Available actions depend on the file's current git status:

| File status | Actions |
|-------------|---------|
| Staged | **Commit** — opens commit message dialog; **Amend Commit** — opens amend commit dialog pre-filled with HEAD message; **Unstage File** — `git reset -- <path>`; **Save as Patch and Revert File** |
| Modified | **Stage file** — `git add -- <path>`; **Revert file** — `git checkout -- <path>`; **Save as patch and revert file** |
| Deleted | **Stage deletion** — `git add -- <path>`; **Revert file** — `git checkout -- <path>`; **Save as patch and revert file** |
| Conflict | **Revert file** — `git reset -- <path>` followed by `git checkout -- <path>` |
| Untracked | **Stage file** — `git add -- <path>`; **Discard file** — deletes the file from disk |

**Save as patch and revert file** (`p`) for a modified file: saves the current diff
as `<file>.patch` (relative to the repo root) then reverts the file to HEAD.
Staged changes are unstaged first.

When the selected file has a `.patch` extension, an additional action is available:

**Apply patch** (`P`): applies the patch file via `git apply`. First attempts a
plain apply; if that fails, retries with `--ignore-whitespace`. Applies with
`--ignore-whitespace` to tolerate trailing-whitespace differences in context lines.

Dismiss the menu with `Esc` or by clicking outside it.

**Commit** (`c`) and **Amend Commit** (`a`) are shown when the selected file is
staged and open a multiline commit message popup:

- Title shows `Commit (N staged)` or `Amend Commit (N staged + M from HEAD)`
  where N is the current staged-file count and M is the number of files changed
  in the HEAD commit
- `Enter` submits the commit; `Shift-Enter` / `Alt-Enter` inserts a newline into the
  message; `Esc` cancels
- Arrow keys (`←` / `→`) move the cursor within the message; full in-line cursor
  navigation is supported
- Amend Commit pre-fills the message with the current HEAD commit message
- Runs `git commit -m <message>` or `git commit --amend -m <message>`

**Amend Commit** is also available from the per-repo action menu (`a`, see
[Git Operations](#git-operations)) and from the History pane's per-commit action
menu (when HEAD is selected in the full-history view), even when no file is
staged — amending with zero staged files simply rewrites the HEAD commit
message. The dialog title then renders `Amend Commit (0 staged + M from HEAD)`.

## Output Log Pane

- Toggle with `l`
- Shows timestamped lines from git command output in local time;
  each line displays `[HH:MM:SS LEVEL] message`
- Severity is colour-coded: `INFO` default, `WARN` yellow, `ERROR` red, `DEBUG` dim gray
- Auto-follows new entries (scrolls to tail) when cursor is at the last visible line
- When pane is not focused, always shows the tail (latest entries)
- User can scroll up into history; scrolling back to tail re-enables auto-follow
- Automatically shown when a git operation fails, so error output is immediately visible
- Scroll indicators (▲ / ▼) appear when content overflows above or below the visible area
- Pressing `Enter` when the Output Log pane has focus opens the log action menu
  - `c` — "Copy log output": copies the entire log content to system clipboard;
    shows a transient popup "Log output copied to clipboard!" that auto-dismisses
    after 2 seconds
  - `x` — "Clear log": clears all log entries and resets scroll position to tail;
    shows a brief "Log cleared" header flash

## Debug Logging

When `--debug-log <path>` is passed on the command line, gitover writes a
structured log to the specified file.

- Enabled via `--debug-log <path>` CLI flag or `general.debug_log` config option;
  CLI flag takes precedence
- Both path sources support `~` and `${VAR}` expansion; the app terminates with
  an error if any variable cannot be resolved
- File is appended to if it already exists, created otherwise; no file is
  written when neither source is set
- Every entry in the Output Log pane is mirrored to the debug log file
- Internal debug events (raw key events, operation dispatch) that are not
  shown in the UI are also written
- Each line uses the format: `[HH:MM:SS LEVEL PID0000] message`
  - `LEVEL` is right-aligned in a 5-character field: `DEBUG`, `INFO`, `WARN`, `ERROR`
  - `PID` is the process identifier of the running app
- Severity levels:
  - `DEBUG` — internal events (key presses, operation routing); file only,
    not shown in the Output Log pane
  - `INFO` — normal operation output (fetch started, scan complete, etc.)
  - `WARN` — non-fatal anomalies
  - `ERROR` — failed git operations

## Git History Pane

- Toggle with `h`; also opened via action menu entries `h` / `u` / `U` / `t` / `T`
- Title shows pane name, commit position indicator, and active filter.
  E.g. `Commit History [3/42]` or `Commit History [3/42] (ahead of origin/main)`
- Displays commit history for the current branch, newest commit first, up to 200 commits
- Two-column table layout: short hash (8 chars, yellow) | rest of row as a single styled line
  - Rest of row: timestamp (YYYY-MM-DD HH:MM:SS local, gray) · author (cyan, up to 20 chars,
    padded) · commit message (first line)
- Each commit row is followed by file sub-rows aligned with the timestamp column:
  - Format: `<change-identifier>  <path>` (two spaces between code and path)
  - A = added (blue), M = modified (green), D = deleted (red), R = renamed (yellow)
  - File sub-rows within each commit are sorted alphabetically by path; case-insensitive
    by default, configurable via `general.case_sensitive_path_sorting`
- `↑`/`↓` and `PgUp`/`PgDn` scroll through commits and file rows
- `Shift-↑` / `Shift-↓` (or `,` / `.`) jump directly to the previous/next commit
  header row, skipping file sub-rows; `,`/`.` are provided as alternatives for
  terminals that intercept Shift+Arrow (e.g. Zed)
- Scroll indicators (▲ / ▼) appear when content overflows above or below the visible area;
  coloured with focused/unfocused border colour
- History reloads automatically when the selected repo changes while the pane is open
- History reloads automatically after a git operation completes on the shown repo
- When switching repos, if the active filter (e.g., "behind trunk") is not applicable
  to the new repo (no ahead/behind commits), the view automatically falls back to
  showing the full branch history
- Filtered views available from the action menu:
  - Ahead of upstream / trunk — commits in HEAD not yet in the remote ref
  - Behind upstream / trunk — commits in the remote ref not yet merged locally
- `h` closes the pane; `Tab` cycles focus between panes without closing it
- `Enter` on a commit header row opens the per-commit action menu.
  Only available when the full current-branch history is shown and the selected
  commit is HEAD.

### Per-commit Action Menu

Opened with `Enter` on the HEAD commit row. Dismiss with `Esc`.

| Key | Action |
|-----|--------|
| `a` | Amend Commit — opens the amend dialog pre-filled with the HEAD message |
| `u` | Undo Commit — runs `git reset --mixed HEAD~1`; removes the HEAD commit and leaves all its changes as unstaged working-tree modifications (no data is lost; `git reflog` can recover the commit) |

## Details Pane

- Toggle with `d`; visibility is persisted in the state file across sessions
- Occupies the right half of the combined Status Details + History vertical area;
  those panes shrink to the left half when the Details pane is open
- Three display modes selected automatically based on what is focused/selected:
  - **Diff mode** — title `Diff — <filename>`; shows a patch diff for the selected file
  - **Commit mode** — title `Commit [n/m]`; shows full commit details for
    the selected commit header row
  - **Empty mode** — shows `Select file or commit for details.` placeholder when
    nothing relevant is selected
- **Diff mode** content sources:
  - When focus is on Status Details: shows `git diff HEAD -- <file>` for the selected file
  - When focus is on History and a file sub-row is selected: shows the file diff
    against the first parent (`git diff <hash>^1 <hash> -- <file>`),
    correctly handling merge commits; falls back to `git show` for root commits
  - Untracked files: shows raw file content instead of a patch diff
  - Binary files: shows `<binary file>`
- **Diff mode** syntax colouring:
  - Added lines (`+`) — green
  - Removed lines (`−`) — red
  - Hunk headers (`@@`) — cyan/author colour
  - File header lines (`diff`, `index`, `+++`, `---`, `Binary`) — gray
- **Commit mode** shows (top to bottom):
  - Short hash (yellow) and commit timestamp in local time (gray)
  - Change summary: `N-A N-M N-D N-R` with per-kind colours, matching the
    Repositories pane status format
  - Author name and email
  - Full commit message (summary in bold, body below a blank line); both summary
    and body lines are word-wrapped to the pane width
  - Position indicator in the title (`[n/m]`) reflects the commit's position
    in the loaded history
- Content is truncated at 1 MB; a `...diff truncated` line is appended when cut
- Content refreshes automatically when cursor moves in Status Details or History,
  or when focus switches between those panes
- While the Details pane itself has focus, scroll position is preserved (no content
  reload on navigation keys)
- `↑`/`↓` and `PgUp`/`PgDn` scroll the content when Details pane has focus;
  mouse wheel also scrolls
- Scroll indicators (▲ / ▼) appear when content overflows above or below the visible area
- `Tab` / `Shift-Tab` cycles focus to/from the Details pane like any other pane

## Real-time Updates

- File system watcher detects changes and refreshes the affected repo instantly
  - Watches the entire working tree: any file creation, modification,
    or deletion triggers a refresh
  - Git-aware filter: watches relevant `.git/` files (HEAD, refs, index, COMMIT_EDITMSG,
    rebase state, etc.) while ignoring noisy internals (objects, pack files, etc.)
  - 500 ms debounce prevents spurious updates during rapid saves
- Wake-from-sleep detection: if a tick gap exceeds 3 s the system likely woke from sleep;
  a full refresh fires to catch missed events
- Automatic background fetch of all tracked repos every 10 minutes;
  manual `Alt-f` resets the timer
- No unconditional background polling — the file watcher handles real-time updates
- Manual refresh with `r` key available from any pane

## Navigation & Keyboard

See [Keybindings](./keybindings.md) for the global, navigation, and per-pane
key tables (including the file picker). Action-menu semantics (`Enter` to
open, `Esc` to dismiss) are described per pane in the sections below.

## User Interface

- Layout (vertical): Repositories / Status Details / Git History / Output Log
  - Status Details, Output Log, and Git History are optional; shown only when toggled open
  - When the Details pane is open it occupies the right half of the combined
    Status Details + History area
- Focused pane highlighted with focused border colour; unfocused panes use
  unfocused border colour
- The Repositories pane can be resized by dragging the bottom divider (the border row
  between the Repositories pane and the pane below it)
  - When optional panes (Status Details, History, Output Log) are shown or hidden
    the custom pane height is preserved so re-opening any of them restores the last
    user-set size
  - The custom size is not saved to the state file (it is terminal-size dependent and
    would not be meaningful across sessions)
  - A ↕ indicator appears on the divider when the mouse hovers over it, signalling
    it is draggable
- The Details pane can be resized by dragging the vertical divider between the
  Status Details / History panes and the Details pane
  - Startup position is 50% of the terminal width; the custom width is not saved to state
  - A ↔ indicator appears on the divider when the mouse hovers over it, signalling
    it is draggable
  - Minimum width of 15 columns is enforced on both sides
- Scroll indicators (▲ / ▼) in all scrollable panes use focused/unfocused border colours
  to match the pane border
- App version shown in the header title (e.g. `Git Repository Overview  v0.1.0`)
- Loading spinner in header while repos are being scanned
- Refresh timestamp shown right-aligned in the header bar
- Auto-fetch countdown shown right-aligned in the header bar (e.g. "fetching all in 30s";
  hidden when auto-fetch is disabled)
- `? help` hint shown in the header title bar; pressing `?` opens a help overlay
  to show available keybindings
- Confirmation dialogs for destructive actions (remove repo, force push)
- Per-repo action menu popup (opened with `Enter`); dismissed with `Esc`
- Action menus are sized and positioned relative to the pane they belong to:
  width is derived from menu content (minimum 40 % of the terminal width,
  clamped at 80 % of the pane width), the popup is centered horizontally over the
  current pane
- Multiple built-in themes selectable at runtime with `T`

## Mouse Interaction

- **Selecting and copying text**: since gitover captures mouse events,
  the terminal cannot do native text selection by default. Hold **Option** (macOS)
  while dragging to bypass the app's mouse capture and select text normally; then copy
  with **Cmd-C** or the terminal context menu.
- Left-click on a pane sets focus to that pane
- Mouse wheel over any pane gives focus to that pane first, then scrolls its content
- Left-click inside the Status Details pane selects the file under the cursor;
  scroll position is preserved
- Left-click inside the History pane selects the commit/change under the cursor;
  scroll position is preserved
- Left-click inside the Details pane sets focus to the Details pane
- Double-click on a repository row opens the per-repo action menu (same as `Enter`)
- Double-click on a file row in the Status Details pane opens the per-file action
  menu (same as `Enter`)
- Left-click on an action menu entry executes the selected action
- Left-click on the `▶`/`▼` indicator of a repository section-title row toggles
  that section expanded/collapsed (same as `←`/`→`)
- Clicking outside the action menu dismisses it, same as pressing `Esc`

## Git Branches Pane

- Toggle with `b`; while open it replaces the Repositories pane; title shows
  "Branches — <repo path>"
- Lists every local branch and every remote-only branch (remote branches not yet
  checked out locally)
- Each branch row has a 3-character marker column followed by branch name,
  upstream ahead/behind, and trunk ahead/behind:
  - `*` — current branch
  - `✓` — branch has been fully merged to trunk (0 commits ahead, ≥1 behind);
    a hint to clean up the branch
  - `*✓` — current branch that is also merged to trunk
  - Upstream column shows `remote only` for branches that exist only on the remote
  - Trunk column shows `is trunk` instead of ahead/behind numbers for the trunk branch itself
- `c` directly checks out the highlighted branch without opening a selection dialog
  (auto-stash/pop applied)
- `Enter` opens the per-branch action menu (see below)
- Scrolling through the branch list updates the History pane (if open) to show commits
  for the highlighted branch; the Branches pane takes precedence over the current repo
  branch for History content
- Closing the Branches pane (`b` or `Esc`) restores the History pane to show commits
  for the current repo branch
- `Esc` closes the Branches pane and returns focus to the Repositories pane

### Per-branch Action Menu

Opened with `Enter` on the highlighted branch row. Dismiss with `Esc`.

| Key   | Action                                                                                                    |
|-------|-----------------------------------------------------------------------------------------------------------|
| `c`   | Checkout — checks out this branch with auto-stash/pop (shown only when not the current branch)            |
| `p`   | Pull Branch (fast-forward) — fast-forward pull from upstream (shown only when branch is behind upstream with no local commits ahead) |
| `P`   | Push Branch — pushes this branch to origin with `--set-upstream`; shown for any local branch that has no upstream yet or is ahead of its upstream |
| `F`   | Force Push Branch — force-pushes this branch to origin (confirmation dialog shown first); same visibility condition as Push Branch |
| `n`   | Create Branch — prompts for a name and runs `git checkout -b <name> <this-branch>`                        |
| `h`   | Commit History — opens the History pane for this branch (full log)                                        |
| `u`   | Commit History Ahead of upstream — commits in this branch not yet in its upstream                         |
| `U`   | Commit History Behind upstream — commits in the upstream not yet merged into this branch                  |
| `t`   | Commit History Ahead of trunk — commits in this branch not yet in the trunk branch                        |
| `T`   | Commit History Behind trunk — commits in the trunk branch not yet merged into this branch                 |
| `d`   | Delete Branch — removes the local branch with `git branch -D` after yes/no confirmation (not shown for the current branch, remote-only branches, or the trunk branch) |
| `Esc` | Dismiss menu                                                                                              |

## Release Info

The binary embeds build metadata at compile time via `build.rs`:

- **Version**: taken from `Cargo.toml` (`CARGO_PKG_VERSION`)
- **Git commit**: short hash of HEAD at build time (`GIT_SHORT_HASH`)
- **Build timestamp**: UTC date/time captured when `cargo build` runs (`BUILD_TIMESTAMP`)

Running `gitover --version` (or `-V`) prints this info and exits immediately
without starting the TUI.

## Release Notifications

- On startup and periodically (default every 24 hours), gitover checks the
  GitHub Releases API for a newer version of the app
- Configurable via `general.release_check_interval` (set to 0 to disable)
- The check runs in the background with silent retry on network errors — no error log spam
- When a newer release is detected:
  - A `✦` indicator appears in the header title bar next to the version
  - An auto-dismissing popup notification is shown on first detection:
    `"New release v0.9.0 available! → …/releases"`
  - The help overlay (`?`) shows a "Release Notifications" section with the new
    version and link
- Per-version dismissal: the popup only appears once per version; the header indicator
  and help entry remain visible
- The last-seen version is persisted in the state file, so the popup won't repeat
  across restarts for the same version
