# Implementation ToDo for Gitover Rust TUI

This is a living document.
New tasks are added as needed.
Done tasks are check-marked when implemented.

## Config & Repo Management

- [ ]

## Git Status Columns

- [ ]

## Git Status Misc

- [ ]

## UX Polish

- [x] Vertical sizing of "Repositories" pane is not good, ensure that this pane is always the biggest.
      When evenly distibuting the vertical space, add any remaining space to "Repositories".
      If total available height for panes is 20
      and open panes are: Repositories, Status & History
      then distribute height like: Repositories=8 Status=6 History=6
- [ ] Show repo root path in "Status" pane title

## Bugs

- [ ]

## Git History Pane

- [ ]

## Git Basic Operations

- [ ]

## Git Rebase Operation

- [ ] Rebase onto trunk branch: run `git rebase <trunkbranch>`
      Auto-stash before rebase, pop stash after rebase completes
- [ ] Rebase controls when rebase is in progress: continue / skip / abort

## Per-file Actions

- [ ] Available per-file-actions are shown in a poup-menu for the current selected file when enter key is hit
- [ ] Stage file: run `git add -- <path>` from the changed-files detail panel
- [ ] Unstage file: run `git reset -- <path>` from the changed-files detail panel
- [ ] Revert file: run `git checkout -- <path>` to discard working-tree changes
      Handle merge-conflict case: `git reset -- <path>` first, then `git checkout -- <path>`
- [ ] Discard untracked file: delete the file from disk

## Tooling

- [ ]

## Testing & Quality

- [ ]

## Documentation & Release

- [ ]
