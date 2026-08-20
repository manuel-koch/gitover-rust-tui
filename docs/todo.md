# Recent / Remaining Implementation Todo Tasks and Bugs

This is a living document, see [todo-workflow](./todo-workflow.md).

## Bugs

- [x] After computer wakeup the TUI can contain rendering artefacts because
      the alternate screen buffer may be corrupted while the machine is sleeping.
      Ratatui 0.30 handles terminal resize automatically (via `autoresize()` inside
      `draw()`), so resize is not affected. Fix: call `terminal.clear()` on wakeup
      detection, and add a `Ctrl-L` keybinding to force a full repaint at any time.

- [x] Functionality automatically opens log-pane when e.g. an error happens
      from a triggered repo pull action.
      But that immediately gives focus to the log-pane.
      Thus if user tries to pull one repo after another, the focus suddenly switches
      to log-pane and no key-press "p" appearently triggers another pull action.
      Only give focus to log-pane when it was explicitly opened by "h" keybinding.

- [ ] A modified file in repo is selected in status pane, details pane shows the diff.
      Reverting the modified file via action menu removes the file
      from the list, but details pane keeps showing the former diff although
      another ( or no ) file is currently selected in status pane afterwards.

## Git Branches

- [x] New action in branches-pane to rename selected (local) branch locally and
      its upstream name. But only change the upstream name if there is one ( to be used on next push ), don't rename the remote branch.

## Git Rebase Operation

> **Design needs rework.** The action-menu entry, conflict-resolution flow,
> rebase-in-progress state tracking, and abort/skip UX are not specified yet;
> the items below are placeholders, not an implementation plan. Rework the
> design before starting on these tasks.

- [ ] Rebase onto trunk branch: run `git rebase <trunkbranch>`
      Auto-stash before rebase, pop stash after rebase completes
- [ ] Rebase controls when rebase is in progress: continue / skip / abort

## Blocked

Tasks blocked on an external dependency stay here until unblocked.
Do NOT implement workarounds without explicit user direction.

- [ ] In the file-picker popup, apply the sorting-flag for paths.
      ( blocked: ratatui-explorer sorts internally, no API to override,
      see https://github.com/tatounee/ratatui-explorer/issues/22 )
