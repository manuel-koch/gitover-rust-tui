# Implementation ToDo for Gitover Rust TUI

This is a living document.
New tasks are added as needed.
Done tasks are check-marked when implemented.
Checkmarked tasks are removed on demand to merge them into `features.md`.

## Bugs

- [x] Commit history not updated properly:
  - Select a repo that has ahead/behind to trunk branch
  - Select show commit history for behind commits to trunk branch
  - History is updated with correct commits
  - Select another repo ( w/ ahead/behind changes )
  - Commit History just shows "No commits found" although the current branch has
    ahead/behind commits that should be shown.
  - Select another repo ( w/o ahead/behind changes )
  - Commit History just shows "No commits found" and title still says
    "Commit History  ( behind origin/master )" although there are no
    ahead/behind commits
  - In general I think the commit history should fallback to showing the
    commit history of
    current branch when current history mode (behind trunk) is not applicable.
- [x] Pull non-current branch doesn't work.
      On a repo with a branch behind-trunk commits, open the branches pane,
      go to non-current branch, open action menu, select "pull" action.
      Output log shows "pulling" action, but nothing changes, branch is not pulled,
      ahead/behind counters are unchanged.
- [x] After long sleep of computer ( overnight ) I sometimes see crashed
      executable in the morning
  - mouse handling is broken afterwards in terminal, prints strange chars
    on mouse movement
  - terminal shows error message of gitover
    ```text
    rary/std/src/thread/functions.rs:131:29: — /Users/manuelkoch/Documents/KnowledgeBase/personal-knowledge-base
    no changes —failed to spawn thread: Os { code: 35, kind: WouldBlock, message: "Resource temporarily unavailable" }
    note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
    ```

## Git Status Columns

- [ ]

## Git Status Pane

- [ ] 

## Git Branches Pane

- [ ]

## UX Polish

- [ ]

## UX Mouse Interaction

- [ ]

## Git Commit History Pane

- [ ]

## Git Details Pane

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
