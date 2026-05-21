# Implementation ToDo for Gitover Rust TUI

This is a living document.
New tasks are added as needed.
Done tasks are check-marked when implemented.

## Bugs

- [x] "Repositories" pane, action menu, first entry is selected, click on second entry does not trigger the action.

## Config & Repo Management

- [x] Add repo commands as a list of mappings to configuration
  - [x] Every repo command has key "name" that holds a string that describes the tool name.
  - [x] Every repo command has key "cmd" that holds a string that is the commandline
        of the tool.
  - [x] "cmd" supports variables like $VAR that are interpolated when the command gets executed
    - [x] Variable $ROOT: the git root path
    - [x] Variable $BRANCH: the current git branch
  - [x] Repo commands are executed with current-working-directory in the git root of
        current repository.
  - [x] Output of run repo command is append to the app output log.
  - [x] Repositories action menu contains the repo commands ( at the end of the list, separated
        from the built-in actions via a separator line "-------" )
  - [x] Repo commands support optional "background" boolean property (default false); when true
        the command is spawned without waiting and its output is discarded.

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

- [ ]
