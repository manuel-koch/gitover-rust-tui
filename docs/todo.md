# Implementation ToDo for Gitover Rust TUI

This is a living document.
New tasks are added as needed.
Done tasks are check-marked when implemented.

## Bugs

- [ ]

## Git Status Columns

- [ ]

## Git Status Pane

- [ ] 

## Git Branches Pane

- [ ] Show "Branches" pane with keybinding "b", "Branches" pane replaces "Repositories" pane
      while "Branches" pane is open.
- [ ] "Branches" uses title "Branches - <repo path>"
- [ ] "Branches" pane shows ahead/behind counts with respect to upstream/trunk for every branch
- [ ] Keybinding "enter" opens action menu for selected branch
  - [ ] Keybinding "u" & "U" update commit history pane, showing ahead/behind commits of selected branch with respect to upstream (if any)
  - [ ] Keybinding "t" & "T" update commit history pane, showing ahead/behind commits of selected branch with respect to trunk
  - [ ] if branch is behind its upstream, show action entry "Pull" with Keybinding "p" to
        fast-forward pull changes without the need to checkout branch in the first place.
- [ ] Keybinding "c" when "Branches" pane has focus directly checks out selected branch locally
      ( bypassing the branch selection dialog )
- [ ] Scrolling thru the branches list triggers update of "History" pane ( if it is open ),
      showing commit history of selected branch.
      "Branches" pane has precendence over current branch from "Repositories" pane.
      Closing the "Branches" pane updates "History" pane with commits from current repos current branch.

## UX Polish

- [ ]

## UX Mouse Interaction

- [ ]

## Git History Pane

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
