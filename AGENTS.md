# Agent Guidelines for Gitover Rust TUI

A rust based terminal UI to track git status of multiple repositories.

Check [features](./docs/features.md) for list of implemented features.

Check [todo](./docs/todo.md) for recent implemenation tasks and remaining tasks.

## Task Execution

Follow the user's instructions precisely!
Do not add extra steps, merge documents, or perform cleanup actions unless explicitly asked!
If you are unsure what the user wants, ask back using `clarify` with specific options
or an open-ended question — do not guess or assume unstated intent.

## Cleanup Todo and merge to Features

Only perform cleanup of todo document when user requests it explicitly !
This means the user says "clean up the todo" or "merge done tasks".
Implementing tasks or marking them `[x]` does NOT trigger this section.

When it does apply, follow these steps.

### Cleanup and merge steps

Force re-reading features and todo document to fully grasp their current content !

Merge finished todo tasks with the features document:

- Check if there is an existing feature that matches task content fully/almost/partly
  - If feature is matched fully, just remove the task from todo
  - If feature is matched partly/almost, check whats the diff to task content and decide if feature
    text should be updated or a new distinct feature be introduced with the task content
  - If feature is not matched, introduce a new distinct feature with tasks content.
    If needed check if the new feature belongs to a new section/heading within the documents
    to group features by topics.
  - if in doubt if a task matches a feature, ask the user how to proceed, provide proposal
    what you think would fit best.
- Don't add explicit features that would stem from task that have subject of
  tests / refactoring / housekeeping or fixing bugs
- Remove finished task from todo when merged with feature document
- Don't remove empty todo sections - we might add new tasks to it,
  add a placeholder "- [ ]" task if neccessary.

For updated features document, consult the sources/implementation to check if features
are actually implemented the way they are currently stated in the feature description.
Update the feature descriptions to match the current implementation.

Check `README.md` too and align it to `features.md`.

## Contributor Notes

### Project layout

```text
src/
  main.rs       — event loop, key handling, ops channel dispatch
  app.rs        — application state (AppState, Focus, AppMode, all fields)
  ui.rs         — ratatui rendering (all draw_* functions)
  git.rs        — git status parsing (RepoStatus, FileEntry, FileStatusKind)
  ops.rs        — background git operations (OpRequest, spawn_op, run_op)
  watcher.rs    — file-system watcher (notify crate, git-aware filter)
  config.rs     — config file loading (~/.config/gitover/config.yaml)
  theme.rs      — UI color theme definitions
  state.rs      — persistent state (repo list, recents, ~/.config/gitover/state.yaml)
  lib.rs        — re-exports config/git/state for integration tests
tests/
  git_tests.rs    — unit + integration tests for git.rs
  config_tests.rs — unit tests for config.rs and state.rs
docs/
  features.md   — implemented feature reference (keep in sync with code)
  todo.md       — living task list (never delete sections, use placeholder)
Makefile        — build, lint, format, test, release, install, tag-version targets
```

### Guidelines for Implementation, Testing and Bugfixing

- Avoid Code Smells
- KISS Principle
- SOLID Principles
- SSOT (Single Source of Truth)
- YAGNI (You Aren’t Gonna Need It)
- Clean Architecture
- Minimum Viable Product (MVP)
- Five Whys (Ohno)
- Chain of Thought (CoT)
- Occam’s Razor
- TDD, Chicago School
- Test Double: Mock (Meszaros)
- Test Double: Spy (Meszaros)
- Test Double: Stub (Meszaros)

### Development workflow

After implementing new functionality or fixing bugs, run the test suite to verify
all tests still succeed.
For new features come up with appropriate test case(s) to verify them.
For bugfixes check whether there is an existing test case that could be improved to
verify the fix or introduce new test case(s) to verify the fix.
If unclear whether to introduce new test case(s), ask user for clarification.

```shell
make lint          # cargo check + cargo clippy — fix all warnings before committing
make format        # cargo fmt
make test          # cargo test — all tests must pass
make build         # cargo build (debug)
make build-and-run # cargo run — quick manual test
make release       # cargo build --release
make install       # cargo install --path . → ~/.cargo/bin/gitover
make tag-version   # tag HEAD with version from Cargo.toml
```
