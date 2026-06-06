# Implementation ToDo for Gitover Rust TUI

This is a living document.
New tasks are added as needed.
Done tasks are check-marked when implemented.
Checkmarked tasks are removed on demand to merge them into `features.md`.

## Bugs

## Git Status Columns

- [ ]

## Git Status Pane

- [ ] 

## Git Branches Pane

- [ ]

## UX Polish

- [ ] When using "A" keypress to add a repositiory, the popup dialog to select a repo should stay open after
      a "space" keypress, allowing to add more repos.

- [ ] Change the title of the "space" action in the add-repo file-picker from "select repo" to "add repo".

- [ ] Add a boolean config option for `config.yaml` to adjust how paths are sorted: `general.case_sensitive_path_sorting`
  - [ ] In the file-picker popup, apply the sorting-flag for paths.
  - [ ] In the repositories pane, apply the sorting-flag for repo paths.
  - [ ] In the status pane, apply the sorting-flag for commit paths.
  - [ ] In the history pane, apply the sorting-flag for commit paths.

## UX Mouse Interaction

- [ ]

## Git Commit History Pane

- [ ]

## Git Details Pane

- [ ]

## Git Basic Operations

- [ ]

## Git Rebase Operation

- [ ] Rebase onto trunk branch: run `git rebase <trunkbranch>`
      Auto-stash before rebase, pop stash after rebase completes
- [ ] Rebase controls when rebase is in progress: continue / skip / abort

## Per-file Actions

- [ ]

## Status-File / Commit-History-File Diff

- [ ]

## Testing & Quality

- [ ]

## Configuration

- [ ]

## Documentation & Release

- [ ]
