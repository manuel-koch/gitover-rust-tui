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

- [ ] In the file-picker popup, apply the sorting-flag for paths. (blocked: ratatui-explorer sorts internally, no API to override)

- [ ] Allow drag'n'drop of pane vertical divider between status-pane/commit-pane and diff-pane.
      This should work like resizing the repositories-pane.
      Don't preserve the selected x-position of the divider in state ( like repositories-pane height ), startup position of divider will still be at 50% view width.

- [ ] Allow action-menu popup to be greater than the pane from which
      it got started, e.g. to overlap the pane vertically below it
      (partly) too.

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

- [x] Add action ( title "Save as patch and revert file" ) to status-pane action-menu
      to save current file change as patch ( using original path + ".patch" postfix )
      and revert the changed file.
- [x] When current file in status-pane matches *.patch then show a apply-patch action in
      action-menu of status-pane.

## Testing & Quality

- [ ]

## Configuration

- [ ]

## Documentation & Release

- [ ]
