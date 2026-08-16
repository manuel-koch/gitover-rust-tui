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

- [x] Problem reappeared: Sometimes pressing keybinding "f" on repo section
      title doesn't result in all repos of the section being fetched.
      "f" just started to non-function on section-titles, though it still
      works on individual repos.
      I can't tell if sometimes some repos can fetch when hitting "f" on the
      section title, I can only see that none of the repo in the section get
      fetched, no error in the logs either.
      Moving the selection via mouse or keyboard to a section title makes no
      difference, same non-functioning "f" keybinding afterwards.

      **Root cause** (confirmed via the diagnostic `dlog!` lines added for
      this bug — see the debug log shipped alongside this change set):
      when the History pane is open (`app.mode == AppMode::History`), the
      main event loop dispatches to `handle_history_key` regardless of
      focus. That handler has no section-aware branch for `'f'` — it was
      never updated when sections landed (`d62cc0e`). So even with focus on
      the Repos pane and the cursor on a section-title row, `f` calls
      `launch_op` directly, which aborts with "no repo selected". The log
      evidence: every abort is preceded by `mode=History` (no
      `key 'f': section title selected → launch_section_fetch` line ever
      appears for those presses).

      **Fixed:** key handling now follows the focused pane (not the mode).
      A single `handle_global_key` owns the section-aware logic, so pressing
      `f` on a section-title row fetches every repo in the section regardless
      of whether the History pane is open or which pane has focus. `h` now
      toggles the History pane, and `A` / `D` only fire while the
      Repositories pane has focus.
      Plan + regression tests: see
      [`investigate-and-fix-fetch-issue-plan.md`](./investigate-and-fix-fetch-issue-plan.md).

## UX Polish

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

## Docs

- [ ]
