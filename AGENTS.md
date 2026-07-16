# Agent Guidelines for Gitover Rust TUI

A Rust-based terminal UI to track git status of multiple repositories.

## How to read these docs

The full document set is enumerated in
[project-layout](./docs/project-layout.md) under the `docs/` entry
(this is the SSOT for "what doc exists, what it covers, when to update it").
The reading order below is the common path through that set:

1. **Before editing anything:** read
  - [project-layout](./docs/project-layout.md) — file tree, source/test
   layout, and the canonical list of docs.
2. **Before investigating behavior:** skim
   [README](./README.md) (project intro and quickstart),
   [features](./docs/features.md) (implemented feature reference), and
   [todo](./docs/todo.md) (current tasks and known bugs). For configuration
   fields and the keybinding tables consult
   [configuration](./docs/configuration.md) and
   [keybindings](./docs/keybindings.md).
3. **Testing:** While implementing code changes (consent given), write tests according to
   [development-workflow](./docs/development-workflow.md)
4. **Before reporting done**, run the verification commands in
   [development-workflow](./docs/development-workflow.md) and fix any findings.
5. **Todo task lifecycle:** Checkmark done todo tasks and merge to
   documentation (consent given) as noted in [todo-workflow](./docs/todo-workflow.md)
7. **For release-note generation:** read
   [release-notes-workflow](./docs/release-notes-workflow.md).

## Communication

* Be concise and direct
* No preamble
* No motivational filler
* No repetition
* Prefer bullets over prose
* Keep explanations to the minimum needed

## Execution

* Admit uncertainty when appropriate
* Break large tasks into independently-verifiable units
* Parallelize independent subtasks where possible
* Execute actions via your tools
* No file edits without explicit user consent
  * Chat triggers — do NOT edit: "analyze", "discuss", "evaluate", "explore", "focus",
    "highlight", "propose", "review", "re-think", "suggest", "X doesn't work",
    "can we refactor Y", "what if Z" (and equivalent-intent phrases).
  * Imperatives — DO edit: "apply it", "do it", "go ahead", "fix it", "fix the bug",
    "implement it" (and equivalent-intent phrases)

When consent applies to a multi-step task, complete all the steps the user asked
for before reporting back; partial completion is not the same as completion.

## Contributor Notes

**Docs are part of the change set.** A commit must update every doc affected
by the change in the same commit. If a doc doesn't exist yet,
ask the user before creating a new doc file.
Out-of-date docs are treated as bugs.

If you notice a stale doc while working on something else, report it; do not fix it unless
asked (consent rule applies).

Maintain configured test coverage threshold as noted in [development-workflow](./docs/development-workflow.md).

## Coding Conventions

Use full, descriptive names for variables, functions, methods, and types — never
abbreviations (e.g. `case_sensitive_sort` not `cs`, `repo_path` not `rp`,
`format_timestamp` not `fmt_ts`).

**Binary + library split.** The project ships both a `[[bin]]` (`src/main.rs`) and a
`[lib]` (`src/lib.rs`) with the same crate name `gitover` (see `Cargo.toml`). Any
type, function, or module that integration tests in `tests/` need to reach must be
re-exported through `src/lib.rs` — `tests/*.rs` cannot `use` items from `src/main.rs`.

Keep `main.rs` thin (event loop, key handling, ops-channel dispatch).

## Guidelines for Implementation, Testing and Bugfixing

These are well-known concepts (**Semantic Anchors**).
Look up any unfamiliar principle via web search before applying it.
Structure your work and thinking to follow those principles:

- Code Smells by Fowler
- Resist scope creep
- KISS (Keep It Simple)
- SSOT (Single Source of Truth)
- YAGNI (You Aren't Gonna Need It)
- Clean Code by Uncle Bob
- SOLID Principles by Martin
- Clean Architecture by Martin
- Five Whys by Ohno — find root cause before fixing
- CoT (Chain of Thought)
- Occam's Razor
- TDD (Test Driven Development)
- Test Double: Mock / Spy / Stub by Meszaros
- Testing Pyramid by Cohn, Testing Trophy by Dodds
