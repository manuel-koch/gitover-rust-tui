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

## Git Commit History Pane

- [ ] Keybinding shift-up/down to jump to next/previous commit when "Commit History"
      pane has focus

## Git Details Pane

- [x] Refactor the current "Diff" pane to be a "Details" pane
- [x] The "Details" pane will show
  - [x] file diff, if a file is selected in status pane or history pane
  - [x] commit details, if a commit message row is selected
- [x] Refactor the "d" keybinding to refer to "Details" pane
- [x] "Details" pane content can be scrolled, has scroll indicator
- [x] When "Details" pane is in commit-mode, it shows
  - [x] pane title as "Commit"
  - [x] short commit hash
  - [x] commit timestamp in local time
  - [x] Summary change indicator like in "Repositories" for branches
  - [x] author name and email
  - [x] full commit message
- [x] When "Details" pane is in diff-mode, it shows
  - [x] pane title as "Diff"
  - [x] the diff as it was implemented in the former "Diff" pane
- [x] If "Details" pane is enabled, show "Select file or commit for details." if
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
