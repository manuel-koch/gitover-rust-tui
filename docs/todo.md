# Implementation ToDo for Gitover Rust TUI

This is a living document.
New tasks are added as needed.
Done tasks are check-marked when implemented.
Checkmarked tasks are removed on demand to merge them into `features.md`.

## Bugs

- [x] Commit history not updated properly:
  - Select a repo that has ahead/behind to trunk branch
  - Select show commit history for behind commits to trunk branch
  - History is updated with correct commits
  - Select another repo ( w/ ahead/behind changes )
  - Commit History just shows "No commits found" although the current branch has ahead/behind commits
    that should be shown.
  - Select another repo ( w/o ahead/behind changes )
  - Commit History just shows "No commits found" and title still says
    "Commit History  ( behind origin/master )" although there are no
    ahead/behind commits
  - In general I think the commit history should fallback to showing the commit history of
    current branch when current history mode (behind trunk) is not applicable.

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
