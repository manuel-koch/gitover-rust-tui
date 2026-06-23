# Implementation ToDo for Gitover Rust TUI

This is a living document.
New tasks are added as needed.
Done tasks are check-marked when implemented.
Checkmarked tasks are removed on demand to merge them into `features.md`.

## Bugs

- [x] Pressing "space" in filepicker on a repo directory closes the file-picker instead of
      keeping it open to add more repositiories.
- [ ] Removing a repository breaks list of repositories.
      The order of the remaining displayed repos is mixed afterwards and
      some repos are have duplicated entries in the list.
      E.g. I open `gitover --state gitover.state.yaml` in the sandbox
      repositories base directory, add all repos.
      Restart the app, delete "repo-02" ( it is second entry ). Afterwards
      the list is mixed and has duplicate entries.

## Git Status Columns

- [ ]

## Git Status Pane

- [ ] 

## Git Branches Pane

- [ ]

## UX Polish

- [ ] In the file-picker popup, apply the sorting-flag for paths.
      ( blocked: ratatui-explorer sorts internally, no API to override,
      see https://github.com/tatounee/ratatui-explorer/issues/22 )

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
