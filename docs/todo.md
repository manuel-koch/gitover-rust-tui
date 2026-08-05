# Recent / Remaining Implementation Todo Tasks and Bugs

This is a living document, see [todo-workflow](./todo-workflow.md).

## Bugs

- [x] Error when building binary
  ```text
  $ make build-release-and-run
  cargo build --release
    Compiling gitover v0.8.9 (/Users/manuelkoch/workspace/gitover-rust-tui)
  error: linking with `cc` failed: exit status: 1
    = note: some arguments are omitted. use `--verbose` to show all linker arguments
    = note: ld: warning: search path '/opt/homebrew/Cellar/libgit2/1.9.4/lib' not found
            ld: library 'git2' not found
            clang: error: linker command failed with exit code 1 (use -v to see invocation)
  error: could not compile `gitover` (bin "gitover") due to 1 previous error
  ```

## UX Polish

- [x] Click on the collapse/expand indicator of repository section should
      toggle expand/collapse state of that section.

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

- [x] Unify the pane naming between "Status Details" and "Details" in
      `docs/features.md` (the Mouse Interaction section and some headings
      refer to "Details").
- [x] Align the "Polish UI"/"UX Polish" heading terminology between project
      prompts and `docs/todo.md` (the todo section is titled "UX Polish";
      the underlying plan recommends keeping that heading).
