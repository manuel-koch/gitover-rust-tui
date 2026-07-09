# Implementation ToDo for Gitover Rust TUI

This is a living document.
New tasks are added as needed.
Done tasks are check-marked when implemented.
Checkmarked tasks are removed on demand to merge them into `features.md`.

## Bugs

- [x] Activity indicator of a repository ( e.g. for fetching, pulling )
      vanish after short delay although the action is not done yet
      ( long running pull ).
      Activity indicator should be visible as long as an activity in in progress.

- [x] j/k keybinding doesn't seem to work in file-picker dialog to
      add a repository. The help hint is "↑↓/jk" but should be
      "↑↓/kj".

## UX Polish

- [ ] Include the currently selected repository info in the repositories-pane
      title, like "Repositories ( <repo-group> - <repo-name> )" when there is a repo selected.

- [ ] In the file-picker popup, apply the sorting-flag for paths.
      ( blocked: ratatui-explorer sorts internally, no API to override,
      see https://github.com/tatounee/ratatui-explorer/issues/22 )

## Git Rebase Operation

- [ ] Rebase onto trunk branch: run `git rebase <trunkbranch>`
      Auto-stash before rebase, pop stash after rebase completes
- [ ] Rebase controls when rebase is in progress: continue / skip / abort

## Check for new app release

- [ ] Frequently check if there is a new released version of the app on github available
      and show a hint in TUI for the user.
