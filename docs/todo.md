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

- [x] Keybinding "d" toggle visibility of sub "Diff" pane,
      but don't give focus to it yet
  - [x] "Diff" pane has 50% horizontal size of vertical width
        and is right aligned to "Status" and "History" panes ( they get smaller when 
        diff is visible ).
  - [x] "Diff" pane shows diff ( against its predecessor ) of current selected file
        from "Status" or "History" pane ( most recent focused pane )
        in "patch" format.
    - [x] Truncate huge diffs, only show first 1MB of diff text, add truncation indicator "...diff truncated"
          as last line if the displayed diff content was truncated.
  - [x] Moving cursor in "Status" file list or "History" file list or switching
        focus between panes, triggers refresh of "Diff" pane content.
  - [x] Keybinding "tab" and "shift-tab" cycles includes the "Diff" pane when
        cycling thru panes.

## Testing & Quality

- [ ]

## Configuration

- [ ]

## Documentation & Release

- [ ]
