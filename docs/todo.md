# Recent / Remaining Implementation Todo Tasks and Bugs

This is a living document, see [todo-workflow](./todo-workflow.md).

## Bugs

- [x] After computer wakeup the TUI can contain rendering artefacts because
      the alternate screen buffer may be corrupted while the machine is sleeping.
      Ratatui 0.30 handles terminal resize automatically (via `autoresize()` inside
      `draw()`), so resize is not affected. Fix: call `terminal.clear()` on wakeup
      detection, and add a `Ctrl-L` keybinding to force a full repaint at any time.

## Git History

- [x] Annotate commit title rows in history-pane with git-tag(s) that are
      avail for current commit.
      Tags are rendered as distinct spans on right side of title row.
      Truncate the displayed title so that tags have enough space.

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
