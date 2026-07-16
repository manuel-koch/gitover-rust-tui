# Project layout

Scope of files and directories in the project:

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
  utils.rs      — variable expansion, path expansion, text truncation
  lib.rs        — re-exports config/git/state for integration tests
build.rs        — emits `BUILD_TIMESTAMP` and `GIT_SHORT_HASH` env vars at compile time
                  (also exposed via `--version`; see [docs/features](./features.md#release-info) )
tests/
  git_tests.rs    — unit + integration tests for git.rs
  config_tests.rs — unit tests for config.rs and state.rs
docs/
  features.md              — implemented feature reference (user-visible behavior, panes,
                              keybindings, actions, configuration surface). Update when
                              user-visible behavior, a pane/column/keybinding, or a
                              config field changes.
  configuration.md         — CLI flags, `general.*` config fields, `repo_commands`,
                              persisted state-file fields, and lookup paths. Update
                              when a CLI flag, config field, or state field is
                              added, removed, renamed, or has its default/type/description
                              changed.
  keybindings.md           — global, navigation, per-pane, and file-picker key tables.
                              Update when a keybinding is added, removed, renamed,
                              or its action changes.
  todo.md                  — living task list (never delete sections, use placeholder).
                              Update when a task is added or completed.
  todo-workflow.md         — how to add, complete, and merge todo tasks. Update when
                              the task lifecycle or cleanup rules change.
  development-workflow.md  — `Makefile` targets, test/lint/format conventions, sandbox
                              repo script. Update when a Makefile target is added,
                              removed, renamed, or its command changes; or when
                              conventions change.
  release-notes-workflow.md — release-notes format and generation rules. Update when
                              the output filename pattern, content rules, or entry
                              format change.
  project-layout.md        — this file.
  screenshot.jpg           — application screenshot. Refresh when the default view
                              changes materially (new pane, new column, new keybinding
                              discoverable from the default view, theme change,
                              layout restructure). The screenshot is embedded in
                              `README.md` and at the top of `docs/features.md`;
                              an outdated image is a doc bug.
  config.schema.json       — JSON Schema for gitover.config.yaml. Update when a
                              config field is added, removed, or has its
                              default/type/description changed.
  state.schema.json        — JSON Schema for gitover.state.yaml. Update when a
                              persisted-state field is added, removed, or has its
                              default/type/description changed.
AGENTS.md                  — Hints and guidelines for AI agents. Update when the agent
                             workflow, reading order, consent triggers, or coding
                             conventions change.
Cargo.toml                 — Rust crate manifest (name, version, dependencies, binary/library
                             targets). Update for version bump on release or to adapt
                             Rust dependencies.
create-sandbox-repos.sh    — Creates a set of sandbox git repositories to showcase
                             gitover's features. Update when special git scenarios should
                             be instantiated.
README.md                  — landing-page project introduction and quickstart. Update
                              when the install/usage flow, top-level feature list,
                              or any CLI flag (advertised in the brief features list)
                              changes.
Makefile        — build/test/lint/format/coverage/install targets (SSOT: `Makefile`; canonical user-facing list: [development-workflow](./development-workflow.md))
```
