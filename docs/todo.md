# Implementation ToDo for Gitover Rust TUI

This is a living document.
New tasks are added as needed.
Done tasks are check-marked when implemented.

## Bugs

- [ ]

## Config & Repo Management

- [x] Save state of panes, to reopen them when the app starts

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

- [x] Allow shift-tab to reverse focus cycle direction, moving focus to the previous focused pane.
- [x] If horizontal space is too small, hide common keybinding hints for "tab", "↑↓", "PgUp/Dn".
- [x] In footer, re-group keybindings in the following order: "A", "D", "r", "Alt-f", "s",
      "h", "l", "c", "Enter".
- [x] In "Repositories" action menu, the keybinding for "History" shows "H",
      but actually "h" is implemented. Fix the displayed keybinding to "h" too.

---

- [ ] Add keybinding of "enter" to "Output Log" pane, to show an action menu
  - [ ] Add action menu entry "Copy log output" ( without a keybinding ) to copy whole content
        of "Output Log" into clipboard.

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

- [ ] Available per-file-actions are shown in a poup-menu for the current selected file when enter key is hit
  - [ ] Stage file: run `git add -- <path>` from the changed-files detail panel
  - [ ] Unstage file: run `git reset -- <path>` from the changed-files detail panel
  - [ ] Revert file: run `git checkout -- <path>` to discard working-tree changes
        Handle merge-conflict case: `git reset -- <path>` first, then `git checkout -- <path>`
  - [ ] Discard untracked file: delete the file from disk

---

- [ ] Keybinding "d" toggle visibility of sub "Diff" pane within "Status" pane, but don't give focus to it yet
  - [ ] "Diff" pane has 50% horizontal size of parent "Status" and is right aligned
  - [ ] Render a visible divider between file list and "Diff" pane
  - [ ] "Diff" pane shows diff of current selected file ( diff against HEAD ) in "patch" format
    - [ ] Truncate huge diffs, only show first 1MB of diff text, add truncation indicator "...diff truncated"
          as last line if the displayed diff content was truncated.
  - [ ] Moving cursor in "Status" file list, triggers refresh of "Diff" pane content
  - [ ] Keybinding "tab" and "shift-tab" cycles includes the "Diff" pane when cycling thru panes

## Testing & Quality

- [ ]

## Documentation & Release

- [ ]
