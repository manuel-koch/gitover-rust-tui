# Keybindings

Global shortcuts and per-pane key references. For pane-by-pane behavior
descriptions, see [Features](./features.md).

## Global shortcuts

These keys work regardless of which pane currently has focus.

| Key | Action |
|-----|--------|
| `Tab` / `Shift-tab` | Cycle focus forward / backward between Repositories / Status Details / History / Details / Output Log panes |
| `r` | Refresh all repositories |
| `Alt-f` | Fetch all tracked repos in parallel |
| `s` | Toggle Status Details pane |
| `h` | Toggle Git History pane |
| `b` | Toggle Git Branches pane |
| `d` | Toggle Details pane |
| `l` | Toggle Output Log pane |
| `?` | Help popup, showing available keybindings |
| `Ctrl-c` | Quit application |

## Navigation

| Key | Action |
|-----|--------|
| `↑` / `↓` | Navigate up/down in focused pane |
| `PgUp` / `PgDn` | Jump 10 rows in focused pane or action menu; clamps at list boundaries, no wrap |
| `→` / `←` | Expand / Collapse selected named section in the Repositories pane (clicking the section's `▶`/`▼` indicator does the same) |
| `Shift-↑` / `Shift-↓` (or `,` / `.`) | Previous / next commit header row (History pane only); `,`/`.` are alternatives for terminals that intercept `Shift-↑`/`Shift-↓` (e.g. Zed) |

## Repositories pane

The keys below act on the selected row (repo or section-title) without opening
the action menu (which is opened with `Enter` — see
[Features → Git Operations](./features.md#git-operations)). Also see the
[Global shortcuts](#global-shortcuts) above, which work from any pane.

| Key | Action |
| ----- | -------- |
| `Shift-a` | Add repository to app (opens file picker — see below) |
| `Shift-d` | Remove selected repository from app (with confirmation) |
| `f` | On a repo row: fetch selected repo. On a section-title row: fetch all repos in that section in parallel |
| `p` | Pull selected repo |
| `Shift-p` | Push selected repo |
| `Alt-Shift-p` | Force push selected repo (with confirmation) |
| `c` | Checkout branch on selected repo |
| `r` | On a repo row: refresh selected repo. On a section-title row: refresh all repos in that section |

## Other panes

Per-pane keys are documented in the relevant pane section:

- [Features → Status Details Pane](./features.md#status-details-pane)
- [Features → Git History Pane](./features.md#git-history-pane)
  - [Per-commit Action Menu](./features.md#per-commit-action-menu)
- [Features → Output Log Pane](./features.md#output-log-pane)
- [Features → Git Branches Pane](./features.md#git-branches-pane)
  - [Per-branch Action Menu](./features.md#per-branch-action-menu)

## File picker (open with `Shift-a`)

| Key | Action |
|-----|--------|
| `↑` / `↓` | Navigate the directory list |
| `→` / `Enter` | Descend into the selected directory |
| `←` / `Backspace` | Go to parent directory |
| `Space` | Add the current directory as a tracked repo and keep the picker open |
| `Esc` | Cancel and close the picker |

`Space` is the primary add-confirm key — it lets you add multiple repos in one
session and also works when the current directory is itself a git repo (so you
can add a child submodule). The hint bar at the bottom of the picker
progressively hides navigation groups on narrow terminals but always keeps the
`Space` / `Esc` hints visible.
