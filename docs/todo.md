# Recent / Remaining Implementation Todo Tasks and Bugs

This is a living document, see [todo-workflow](./todo-workflow.md).

## Bugs

- [x] Sometimes pressing keybinding "f" on repo section title doesn't result in
      all repos of the section being fetched.
      It works sometimes though I can't say what makes it fail?
      A hint, I recall the case where it did not work:
      - "f" keybinding was working on (all) repo sections and (all) repos directly
      - I removed a repo section incl its repos from the app
        - the repos in that section were broken, due to revoked access permission
          I could not fetch them anymore
      - now "f" only works on (other) repos directly
      - restarted the app
      - now "f" works again on (all) repo sections and (all) repos directly
      - **Fixed:** repos in an `error` state were silently excluded from every
        fetch path (section `f`, repo `f`, `Alt-f`, and auto-fetch). They are now
        always retried on manual fetch and included in fetch-all; genuine
        failures surface as visible `OpResult` errors.

## UX Polish

- [x] Allow ammending last git commit even no current file is staged.

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

## Docs

- [ ]
