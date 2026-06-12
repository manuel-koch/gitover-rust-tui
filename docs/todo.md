# Implementation ToDo for Gitover Rust TUI

This is a living document.
New tasks are added as needed.
Done tasks are check-marked when implemented.
Checkmarked tasks are removed on demand to merge them into `features.md`.

## Bugs

- [x] Added "repo-01.origin" from sandbox repos, nothing happened. Re-started gitover,
      now "repo-01.origin" was shown in repositories with an error message:
      "cannot status. This operation is not allowed against bare repositories."
      I guess adding such a repo should not be possible in the first place.

## Git Status Columns

- [ ]

## Git Status Pane

- [ ] 

## Git Branches Pane

- [x] Add new commit action to status-pane action-menu when current file is staged.
  - [x] Popup title includes number of staged files.
  - [x] Open a popup dialog to enter the (optionally multiline ) commit message.
  - [x] While user writes commit message, treat shift-enter or alt-enter as newline in the
        commit message rather then accepting/commiting the popup.

- [x] Add amend-commit action to status-pane action-menu when current file is staged.
  - [x] Popup title includes number of staged files and file count of former commit.
  - [x] Pre-fill the commit message dialog with the HEAD commit message.
  - [x] Runs `git commit --amend -m <message>`.

## UX Polish

- [ ] In the file-picker popup, apply the sorting-flag for paths.
      (blocked: ratatui-explorer sorts internally, no API to override)

## UX Mouse Interaction

- [ ]

## Git Commit History Pane

- [ ] Enter key on commit title row opens action-menu in commit-history-pane
      ( only if history is filtered on current branch )
  - [ ] Add action to undo HEAD commit, convert all changes of HEAD commit as local changes in worktree
    - [ ] Show this action only if current commit is HEAD

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
