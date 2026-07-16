# Keybindings

Global shortcuts and per-pane key references. For pane-by-pane behavior
descriptions, see [Features](./features.md).

## Global shortcuts

| Key | Action |
|-----|--------|
| `Tab` / `Shift+Tab` | Cycle focus forward / backward between Repositories / Status Details / History / Details / Output Log panes |
| `r` | Refresh all repositories |
| `Alt-f` | Fetch all tracked repos in parallel |
| `s` | Toggle Status Details pane |
| `h` | Toggle Git History pane |
| `b` | Toggle Git Branches pane |
| `d` | Toggle Details pane |
| `l` | Toggle Output Log pane |
| `?` | Help popup, showing available keybindings |
| `Ctrl-C` | Quit application |
| `T` | Cycle through built-in themes |

## Navigation

| Key | Action |
|-----|--------|
| `↑` / `↓` | Navigate up/down in focused pane |
| `PgUp` / `PgDn` | Jump 10 rows in focused pane or action menu; clamps at list boundaries, no wrap |
| `→` / `←` | Expand / Collapse selected named section in the Repositories pane |
| `Shift-↑` / `Shift-↓` (or `,` / `.`) | Previous / next commit header row (History pane only); `,`/`.` are alternatives for terminals that intercept Shift+Arrow (e.g. Zed) |

## Repositories pane

Shortcuts that work directly on the focused row (repo or section-title), without
opening the action menu (the menu itself is opened with `Enter` — see
[Features → Git Operations](./features.md#git-operations)):

|| Key | Action |
|-----|--------|
|| `A` | Add repository (opens file picker — see below) |
|| `D` | Remove selected repository (with confirmation) |
|| `f` | On a repo row: fetch selected repo. On a section-title row: fetch all repos in that section in parallel |
|| `p` | Pull selected repo |
|| `P` | Push selected repo |
|| `c` | Checkout branch on selected repo or currently selected branch |
|| `r` | On a repo row: refresh selected repo. On a section-title row: refresh all repos in that section |

## Other panes

Action triggers and per-pane keys are documented in the relevant pane
section — they are not global shortcuts:

- [Features → Status Details Pane](./features.md#status-details-pane)
- [Features → Git History Pane](./features.md#git-history-pane)
  - [Per-commit Action Menu](./features.md#per-commit-action-menu)
- [Features → Output Log Pane](./features.md#output-log-pane)
- [Features → Git Branches Pane](./features.md#git-branches-pane)
  - [Per-branch Action Menu](./features.md#per-branch-action-menu)

## File picker (open with `A`)

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
