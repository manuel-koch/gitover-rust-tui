# Recent / Remaining Implementation Todo Tasks and Bugs

This is a living document, see [todo-workflow](./todo-workflow.md).

## Bugs

- [ ] 

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
