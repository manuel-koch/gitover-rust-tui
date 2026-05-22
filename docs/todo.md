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

- [x] Show "Branches" pane with keybinding "b", "Branches" pane replaces "Repositories" pane
      while "Branches" pane is open.
- [x] "Branches" uses title "Branches - <repo path>"
- [x] "Branches" pane shows ahead/behind counts with respect to upstream/trunk for every branch
- [x] Keybinding "enter" opens action menu for selected branch
  - [x] Keybinding "u" & "U" update commit history pane, showing ahead/behind commits of selected branch with respect to its upstream (if any)
  - [x] Keybinding "t" & "T" update commit history pane, showing ahead/behind commits of selected branch with respect to its trunk
  - [x] if branch is behind its upstream, show action entry "Pull" with Keybinding "p" to
        fast-forward pull changes without the need to checkout branch in the first place.
- [x] When "Branches" pane has focus, then pressing key "c" directly checks out selected branch locally
      ( bypassing the branch selection dialog )
- [x] Scrolling thru the branches list triggers update of "History" pane ( if it is open ),
      showing commit history of selected branch.
      "Branches" pane has precendence over current branch from "Repositories" pane.
      Closing the "Branches" pane updates "History" pane with commits from current repos branch.
- [x] Press "esc" key while "Branches" pane is open and has focus, close Branches pane and
      give focus to Repositories pane.

## UX Polish

- [x] Add keybinding "?" to show current active keybindings
  - [x] this help is only shown when no action menu is currently displayed
  - [x] Drop the footer line
  - [x] Just show keybinding "? help" in the title line

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
