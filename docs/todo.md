# Implementation ToDo for Gitover Rust TUI

This is a living document.
New tasks are added as needed.
Done tasks are check-marked when implemented.

## Bugs

- [x] Truncating / hiding the keybindings when the terminal is too small doesn't work as expected.
      Given a small terminal width, I see those keybindings that should have been hidden and
      the last keybinding for "checkout" is truncated to "chec".
- [x] "Repositories" pane -> double-click repo -> click on checkout-action
      -> click on a branch entry in the branch popup menu doesn't checkout that branch.
 
## Config & Repo Management

- [ ] Add repo commands as a list of mappings to configuration
  - [ ] Every repo command has key "name" that holds a string that describes the tool name.
  - [ ] Every repo command has key "cmd" that holds a string that is the commandline
        of the tool.
  - [ ] "cmd" supports variables like $VAR that are interpolated when the command gets executed
    - [ ] Variable $ROOT: the git root path
    - [ ] Variable $BRANCH: the current git branch
  - [ ] Repo commands are executed with current-working-directory in the git root of
        current repository.
  - [ ] Output of run repo command is append to the app output log.
  - [ ] Repositories action menu contains the repo commands ( at the end of the list, separated
        from the built-in actions via a separator line "-------" )

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

- [x] Add keybinding of "enter" to "Output Log" pane, to show an action menu
  - [x] Add action menu entry "Copy log output" ( without a keybinding )
        to copy whole content of "Output Log" into clipboard.
  - [x] After logs have been copied to clipboard, show a popup message
        with notice "Log output copied to clipboard!", auto hide the popup after 2sec.
  
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

- [ ] Available per-file-actions are shown in a poup-menu for the current selected file
      when enter key is hit or file is double-clicked
  - [ ] Stage file: run `git add -- <path>` from the changed-files detail panel
  - [ ] Unstage file: run `git reset -- <path>` from the changed-files detail panel
  - [ ] Revert file: run `git checkout -- <path>` to discard working-tree changes
        Handle merge-conflict case: `git reset -- <path>` first, then `git checkout -- <path>`
  - [ ] Discard untracked file: delete the file from disk

## Per-file Diff

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

- [ ] Add make target to build executable for release
- [ ] Include release info into build artifact: build timestamp, version, git commit short-hash
  - [ ] Show version in title of app
  - [ ] Print build timestamp, version & git commit short-hash in terminal
        when app is run with "--version" commandline argument and immediately exit.
