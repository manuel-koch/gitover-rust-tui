# Implementation ToDo for Gitover Rust TUI

This is a living document.
New tasks are added as needed.
Done tasks are check-marked when implemented.

## Bugs

- [ ]

## Git Status Columns

- [ ]

## Git Status Pane

- [ ] 

## Git Branches Pane

- [ ]

## UX Polish

- [ ]

## UX Mouse Interaction

- [ ]

## Git History Pane

- [ ] Refactor the current "Diff" pane to be a "Details" pane
- [ ] The "Details" pane will show
  - [ ] file diff, if a file is selected in status pane or history pane
  - [ ] commit details, if a commit message row is selected
- [ ] Refactor the "d" keybinding to refer to "Details" pane
- [ ] "Details" pane content can be scrolled, has scroll indicator
- [ ] When "Details" pane is in commit-mode, it shows
  - [ ] pane title as "Commit"
  - [ ] short commit hash
  - [ ] commit timestamp in local time
  - [ ] Summary change indicator like in "Repositories" for branches
  - [ ] author name and email
  - [ ] full commit message
- [ ] When "Details" pane is in diff-mode, it shows
  - [ ] pane title as "Diff"
  - [ ] the diff as it was implemented in the former "Diff" pane
- [ ] If "Details" pane is enabled, show "Select file or commit for details." if
      neither file nor commit row is currently selected.

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
