# Gitover Rust TUI - Implementation ToDo

This is a living document.
New tasks are added as needed.
Done tasks are check-marked when implemented.

## Cleanup Todo and merge to features

When user requests cleanup of todo, then merge finished tasks with the features document.

Force re-reading features and todo document to fully grasp their current content !

- Check if there is an existing feature that matches task content fully/almost/partly
  - If feature is matched fully, just remove the task from todo
  - If feature is matched partly/almost, check whats the diff to task content and decide if feature
    text should be updated or a new distinct feature be introduced with the task content
  - If feature is not matched, introduce a new distinct feature with tasks content.
    If needed check if the new feature belongs to a new section/heading within the documents
    to group features by topics.
  - if in doubt if a task matches a feature, ask the user how to proceed, provide proposal what you think would fit best.
- Don't remove empty todo sections - we might add new tasks to it, just add a placeholder "- [ ]" task.
- Don't remove the section "Cleanup Todo and merge to features"

## Config & Repo Management

- [ ]

## Git Status Columns

- [ ]

## Git Status Misc

- [ ]

## UX Polish

- [x] Add global keybinding "alt-f" to fetch all tracked repos
- [x] Don't cycle round the edges of the repo list when moving cursor up/down, stick at the start/end

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

- [x] Introduce Makefile with useful targets to
  - [x] "lint": Run `cargo clippy` and fix all warnings
  - [x] "format": Run `cargo fmt` and enforce formatting
  - [x] "build-and-run": Build the app and run it with `cargo run`

## Testing & Quality

- [x] Unit tests for git status parsing logic (git.rs)
- [x] Unit tests for config load/save
- [x] Integration smoke test: spawn app against temp git repos
  - [x] if possible test various repo scenarios by creating/tweaking a temp git repo into a test scenario
        and run the tool on it.
  - [x] Add make target "test" to run all tests

## Documentation & Release

- [ ] Write README.md
  - [ ] brief description of features ( features.md will contain the full detailed description )
  - [ ] how to build and install the tool
  - [ ] basic usage examples
  - [ ] keybindings
- [ ] Add AGENTS.md notes for contributors
