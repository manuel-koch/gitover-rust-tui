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

- [x] Allow push changes on any local branch (including current branch)
  - [x] The action should only be shown when branch has no upstream yet or is ahead of upstream
  - [x] Add an action with keypress "P" to branches-pane action-menu to push current selected branch
  - [x] Add an action with keypress "F" to branches-pane action-menu to force-push current selected branch (with confirmation dialog)
  - [x] Show push/force-push in repos-pane action-menu regardless of whether branch has an upstream configured

- [ ] Add new commit action to status-pane action-menu when current file is staged.
  - [ ] Open a popup dialog to enter the (optionally multiline ) commit message.
  - [ ] While user writes commit message, treat shift-enter or alt-enter as newline in the
        commit message rather then accepting/commiting the popup.

## UX Polish

- [ ] In the file-picker popup, apply the sorting-flag for paths.
      (blocked: ratatui-explorer sorts internally, no API to override)

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


## Testing & Quality

- [ ]

## Configuration

- [ ]

## Documentation & Release

- [ ]
