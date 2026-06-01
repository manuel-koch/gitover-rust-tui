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

- [x] Mark local/remote branches that have been merged to trunk.
      This can be a hint for user to remove the local (abandoned) branch.
- [x] Add action to action-menu in branches pane to remove selected local branch 

## UX Polish

- [x] Show action menu aligned to current pane ( not always at y of repositories pane ) 
  - [x] Width of an action menu should be clamped at 80% width of the pane, use a
        width that stems from the title/actions content length instead
  - [x] Align action menu horizontally centered on current pane

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
