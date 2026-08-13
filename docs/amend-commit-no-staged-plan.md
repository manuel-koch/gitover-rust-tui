# Allow amending the last commit when no file is staged

## Context

**System being changed:** the gitover Rust TUI (`src/app.rs`, `src/main.rs`,
`src/ops.rs`). This change touches the action menus and the commit/amend flow.

**Goal and motivation:** today "Amend Commit" is only reachable from the
per-file action menu, and that menu only shows the entry when the selected
file's git status is `Staged` (`src/app.rs::open_file_action_menu`, the
`FileStatusKind::Staged` arm). If a repo has no staged file — for example the
last commit message has a typo — the user has no way to amend it from the UI.
This task (todo item "Allow ammending last git commit even no current file is
staged", `docs/todo.md` → UX Polish) makes amend reachable regardless of the
staged-file count.

**Expected behavior after the change:**

- The per-repo action menu (Enter on a repo row) contains an "Amend Commit"
  entry for every error-free repo, independent of whether anything is staged.
- The per-commit action menu in the History pane (visible when the HEAD commit
  is selected in the full-history view) also lists "Amend Commit".
- Confirming either entry opens the existing pre-filled amend dialog (HEAD
  commit message + `N from HEAD` file count) and runs `git commit --amend -m
  <msg>`.
- The existing per-file "Amend Commit" path (staged-file rows) keeps working
  unchanged.

**Amend semantics (confirmed decision):** message-only. The amend runs
`git commit --amend -m <msg>` with no `-a` and no automatic staging: only
already-staged changes are folded into the rewritten commit, unstaged
working-tree changes are left untouched. Amending with zero staged files just
rewrites the HEAD commit record/message.

**Acceptance criteria / definition of done:**

1. `Amend Commit` (`a`) appears in the per-repo action menu for every
   error-free repo, including repos with zero staged files.
2. `Amend Commit` (`a`) appears in the History pane's per-commit action menu
   when HEAD is selected in the full-history view.
3. Amending with **zero staged files** succeeds and updates the HEAD commit
   message (verified by an integration test against a real repo).
4. The per-file amend path is unchanged.
5. Affected docs updated: `docs/features.md`, `docs/todo.md`
   (`docs/keybindings.md` verified unchanged).
6. `make format`, `make lint`, `make test`, `make test-coverage` all pass
   (≥80% line coverage on testable files; `ui.rs`/`main.rs` excluded).

**Key constraints:**

- The chosen menu key `a` must not collide with existing per-repo or
  per-commit menu keys.
- `main.rs` stays thin; menu dispatch lives in `dispatch_menu_action` /
  `dispatch_history_menu_action`.
- No new dependencies.
- No git I/O may be added to menu-open time. (The "always show" behavior for
  unborn repos is by design: on a repo with no commits the amend fails and the
  error surfaces in the Output Log, which auto-opens on failure.)
- No direct `a` shortcut on the Repositories pane — the feature is menu-only in
  both places.

**Background / this task and amend semantics:** `git commit --amend -m <msg>`
succeeds even with an empty index — it rewrites the HEAD commit record with the
new message (staged changes, if any, are folded in). So the amend operation
itself needs no code change; only reachability does. The amend dialog title
already renders `Amend Commit (0 staged + N from HEAD)` correctly when nothing
is staged (`src/ui.rs::draw_commit_message_input`).

## Implementation Plan

1. **Add the menu entry to the per-repo action menu** in
   `src/app.rs::open_repo_row_action_menu` (inside the `if !has_error { ... }`
   block, directly after the `Create Branch` item at `src/app.rs:1012`):
   `items.push(MenuItem::item("Amend Commit", 'a'));`.
   This makes amend available for every error-free repo, no extra state and no
   git call at menu-open time.

2. **Add the menu entry to the per-commit action menu** in
   `src/app.rs::open_history_action_menu` (`src/app.rs:1201`): change the
   `vec![MenuItem::item("Undo Commit", 'u')]` to list "Amend Commit" (`a`)
   first, then "Undo Commit" (`u`). The existing surrounding logic already
   restricts this menu to the HEAD commit in the full-history view, which
   guarantees a HEAD commit exists.

3. **Wire the repo-menu dispatch** in `src/main.rs::dispatch_menu_action`: add
   an `'a'` arm `app.close_menu(); app.open_amend_input();`.

4. **Wire the per-commit-menu dispatch** in
   `src/main.rs::dispatch_history_menu_action` (`src/main.rs:1518`): add an
   `'a'` arm `app.close_menu(); app.open_amend_input();`.

   Both reuse the existing `App::open_amend_input` (`src/app.rs:1366`) — it
   already resolves the selected repo, pre-fills the HEAD message, computes
   `commit_head_file_count`, sets `commit_is_amend = true`, and switches to
   `AppMode::CommitMessageInput`. No changes to `src/ops.rs` (commit op) or
   `src/ui.rs` (dialog) are required.

5. **Add an integration test** in `src/ops.rs` (tests module) proving amend
   works with an empty index: clone the existing
   `spawn_op_amend_commit_updates_message` test but **omit the `StageFile`
   call** — create a committed repo (`make_committed_repo`), run
   `OpRequest::Commit { amend: true, message: "amended message" }` directly,
   assert `result.success` and that HEAD's summary equals the new message.

6. **Add unit tests** in `src/app.rs` (tests module):
   - Mirror `repo_action_menu_includes_remove_repo_entry`: build an error-free
     fake repo via `make_app()` (push a path, set `repo.error = None`), call
     `open_repo_action_menu()`, and assert some item has
     `label == "Amend Commit"`, `key == 'a'`, `!is_separator`.
   - Mirror `open_repo_action_menu_repo_row_opens_repo_menu` for the History
     menu: set `history = vec![make_commit(0)]`, `history_filter = Full`,
     `history_selected = 0`, call `open_history_action_menu()`, and assert it
     contains both "Amend Commit" (`a`) and "Undo Commit" (`u`). If the test
     helper `make_commit` cannot be reused directly, build a minimal
     `CommitEntry` inline.

7. **Update documentation** (docs are part of the change set):
   - `docs/features.md`:
     - Git Operations action-menu table (~line 162): add a row
       `| \`a\` | Amend Commit — opens the amend dialog pre-filled with the HEAD message; available even when no file is staged |`.
     - Per-commit Action Menu table (~lines 336–339, currently only `u`):
       add a row
       `| \`a\` | Amend Commit — opens the amend dialog pre-filled with the HEAD message |`.
     - Per-file Actions section (~lines 245–256, the Commit/Amend paragraph):
       note that Amend Commit is also available from the per-repo action menu
       and the History pane's per-commit menu even when no file is staged, and
       that the `0 staged + N from HEAD` title applies.
   - `docs/todo.md`: flip the UX Polish task to `- [x]`.
   - `docs/keybindings.md`: action-menu keys are documented in `features.md`,
     not this file — verify no change needed and leave untouched unless the
     verification shows otherwise.
   - `README.md` / screenshot: not affected (no top-level feature-list or default
     view change).

8. **Verify:** run `make format`, `make lint`, `make test`, `make test-coverage`
   and fix any findings before reporting done.

## Proposed Improvements

- **Hide Amend Commit on unborn repos.** Removed from scope by decision (the
  entry is always shown and the git failure surfaces in the Output Log). A
  future improvement could gate the entry on `get_head_commit_message(path)
  .is_some()` at menu-open time, at the cost of extra git I/O when opening the
  menu.
- **Include unstaged changes in the amend.** Deliberately rejected: changing
  the semantics to `git commit --amend -a` (or auto-staging) would silently
  commit unstaged modifications. Amend only folds in what is staged.

## Implementation Hints

- Exact current menu keys in the per-repo action menu: `f p P F c n` then the
  history item(s) `h u U t T`, separator, then section/repo actions `N M D`,
  then custom-command digits — see `docs/features.md` "Git Operations" table and
  `src/app.rs::open_repo_row_action_menu` (`src/app.rs:997`). `a` is free.
- The per-commit History menu currently contains exactly one entry, "Undo
  Commit" (`u`), built in `src/app.rs::open_history_action_menu`
  (`src/app.rs:1201`); it only opens when `history_filter == Full`, the
  selected row is a commit header row, and that commit is HEAD (index 0). Its
  dispatch lives in `src/main.rs::dispatch_history_menu_action`
  (`src/main.rs:1518`); `a` is free there.
- Amend dialog title format lives in `src/ui.rs::draw_commit_message_input`
  (`src/ui.rs:2398`): `" Amend Commit ({staged} staged + {} from HEAD) "`; it
  already handles the `0 staged` case.
- Commit op is `OpRequest::Commit { message, amend }` → `git commit [-amend]
  -m <message>` in `src/ops.rs:423`. Its label is `"amend commit"`
  (`src/ops.rs:150`); the activity spinner shows `Committing`
  (`src/main.rs::launch_op`).
- After amend, `handle_op_result` refreshes the repo and reloads history
  (`src/main.rs:1409`), so status counts and the History pane update without
  further work.
- Test helpers: `make_app()` and the existing menu tests in `src/app.rs`; the
  ops tests use `make_committed_repo(&tempdir)` + `run_op_sync` in `src/ops.rs`.
  Integration tests in `tests/` reach items only via `src/lib.rs` re-exports.
- Coverage: ≥80% line coverage on testable files (`make test-coverage`);
  `ui.rs` and `main.rs` are excluded, so the new `dispatch_menu_action` /
  `dispatch_history_menu_action` arms are not coverage-tracked.
- Existing tests that must keep passing: `repo_action_menu_includes_remove_repo_entry`,
  `open_repo_action_menu_repo_row_opens_repo_menu`, and
  `open_history_action_menu`-related tests in `src/app.rs` (they assert
  presence/non-emptiness or specific entries, not exact menu length).
- Repo-menu git operations are all gated on `repo.error.is_none()`
  (`src/app.rs:1003`, `let has_error = repo.error.is_some();`).
