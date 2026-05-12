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

- [ ]

## Bugs

- [ ]

## Git History Pane

- [ ] When repo is selected, upon "h" key open git commit history for current branch
- [ ] Display the commit history using a table with the following columns
  - [ ] short-commit-hash
  - [ ] timstamp: YYYY-MM-DD HH:MM:SS in local time
  - [ ] username
  - [ ] Commit message, just first line
    - [ ] Sub-rows, a row for every file of that commit, content in the commit message column
      - [ ] file cell formatted as: "<change-identifier> <path>" where change identifier is a single key
        - [ ] M=modified (green)
        - [ ] D=deleted (red)
        - [ ] A=added (blue)
- [ ] Order commits ascending, newest commit at top of list
- [ ] Update commit history data only if pane is current open, force refresh on opening the pane
- [ ] Distribute the column widths so that columns with more content are wider

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

- [x] Write README.md
  - [x] brief description of features ( features.md will contain the full detailed description )
  - [x] how to build and install the tool
  - [x] basic usage examples
  - [x] keybindings
- [x] Add AGENTS.md notes for contributors
