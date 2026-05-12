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

- [x] Immediately show the "Output Log" pane when a git command like fetch/checkout/fetch/pull failed.
- [x] Remove keybinding for "Q" to quit the app, we just rely on ctrl-c to quit the app.
      Adapt documentation *.md files too.
      Adapt app footer too, to remove the "Q" key hint.

## Bugs

- [ ]

## Git History Pane

- [x] When repo is selected, upon "h" key open git commit history for current branch
- [x] Display the commit history using a table with the following columns
  - [x] short-commit-hash
  - [x] timstamp: YYYY-MM-DD HH:MM:SS in local time
  - [x] username
  - [x] Commit message, just first line
    - [x] Sub-rows, a row for every file of that commit, content in the commit message column
      - [x] file cell formatted as: "<change-identifier> <path>" where change identifier is a single key
        - [x] M=modified (green)
        - [x] D=deleted (red)
        - [x] A=added (blue)
- [x] Order commits ascending, newest commit at top of list
- [x] Update commit history data only if pane is current open, force refresh on opening the pane
- [x] Distribute the column widths so that columns with more content are wider
- [x] When repo action is triggered, add action to show commit history of
  - [x] ahead/behind commits with respect to upstream
  - [x] ahead/behind commits with respect to trunk
- [x] Update commit history when current selected repo changes

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
