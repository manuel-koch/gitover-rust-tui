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

- [ ]

## Bugs

- [ ] repo actions menu should not use "q" keybinding, just "esc", which actually works although it is not contained in the menu.
- [ ] Attempt to add a new repo, that is within another parent repo directory will not work.
      Likely because the "enter" key in the file/dir picker is reused to: 1. dive into a dir 2. select a repo.
      if dir is a git repo and has child repos, then enter will select it and never dive into it.

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

- [x] Available per-repo-actions are shown in a poup-menu for the current selected repo when enter key is hit
- [x] Fetch: run `git fetch origin --prune` per repo (keybinding: `f`)
      After fetch completes, trigger a status update
- [x] Pull: run `git pull --prune` per repo (keybinding: `p`)
      Auto-stash dirty changes before pull, pop stash afterwards
- [x] Push: push current branch, set upstream automatically if no tracking branch is configured
      (keybinding: `P`)
- [x] Force push: push with `--force` (keybinding: `shift+P` or prompt)
- [x] Checkout branch: select from list of local and available remote branches
      Auto-stash dirty changes before checkout, pop stash afterwards
- [x] Create new branch: prompt for branch name, sanitise input, run `git checkout -b <name>`
- [x] Delete branch: select from list of local branches, run `git branch -D <name>`

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

## Testing & Quality

- [ ] Unit tests for git status parsing logic (git.rs)
- [ ] Unit tests for config load/save
- [ ] Integration smoke test: spawn app against a temp git repo
- [ ] Run `cargo clippy` and fix all warnings
- [ ] Run `cargo fmt` and enforce formatting

## Documentation & Release

- [ ] Write README.md (features, install, usage, keybindings)
- [ ] Add AGENTS.md notes for contributors
- [ ] Add `cargo install` instructions
