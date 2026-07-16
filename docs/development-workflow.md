# Development workflow

After implementing new functionality or fixing bugs, run the test suite and the linter.

For new features come up with appropriate test case(s) to verify them.

Bugfixes MUST add or strengthen a test that fails before the fix and passes after.

Coverage threshold: **≥80% line coverage** is enforced on testable files
by `make test-coverage` (below the threshold the command fails).
`ui.rs` and `main.rs` are excluded because they require a live terminal.

The following `make` targets are available:

```shell
make format                          # cargo fmt — enforce consistent formatting
make lint                            # cargo check + cargo clippy — fix all warnings before committing
make test-coverage                   # run tests + print per-file coverage summary; fails if <80%
make test-coverage-missing           # same as test-coverage but also prints uncovered line numbers
make test                            # cargo test — all tests must pass
make build-debug                     # cargo build (debug)
make build-debug-and-run             # build debug binary and launch it
make build-debug-and-run-with-sandbox-repos
                                     # build debug binary and launch it with scaffold sandbox repos under ~/tmp/gitover-sandbox
make build-release                   # cargo build (release)
make build-release-and-run           # build release binary and launch it
make install                         # cargo install --path . → ~/.cargo/bin/gitover
make tag-version                     # tag HEAD with version from Cargo.toml
make outdated-dependencies           # cargo update --dry-run — show available upgrades
make upgrade-dependencies            # cargo update — apply dependency upgrades to Cargo.lock
```

The `Makefile` at the project root is the source of truth for `make` targets.
This section is a documentation mirror — when a target is added, removed, renamed,
or its command changes, update this section in the same commit (see
[AGENTS.md](../AGENTS.md) → *Docs are part of the change set*).
Other documents that reference `make` targets must delegate here rather than
re-list targets, otherwise the lists drift the next time the `Makefile` changes.

When the visible UI changes materially, refresh `docs/screenshot.jpg` so it
stays representative — see [project-layout.md](./project-layout.md) under
`docs/` for the full trigger list.

## Sandbox repositories

`create-sandbox-repos.sh` creates a set of demo git repositories under
`<base-dir>` that demonstrate a selection of git states visible in gitover:

| Repo | What it demonstrates |
|------|----------------------|
| `repo-01` | Clean repo, fully in sync with upstream (↑0 ↓0) |
| `repo-02` | Staged + modified + deleted + untracked files (S/M/D/U counts); includes a binary file (modified) and a text file with a removed line |
| `repo-03` | 3 commits ahead of upstream (↑3 ↓0) |
| `repo-04` | 2 commits behind upstream (↑0 ↓2) |
| `repo-05` | Feature branch, 2 commits ahead of trunk (↑2 ↓0 trunk) |
| `repo-06` | Detached HEAD (`detached <sha8>` in Branch column) |
| `repo-07` | Active merge conflict (C count in Status column) |
| `repo-08` | Two branches: `merged-feature` (fully merged to trunk, shown with ✓) and `active-feature` (1 commit ahead of trunk) |
| `repo-09` | Two local branches never pushed to origin: `feature/wip` (current, no upstream) and `draft-notes` (non-current, no upstream) |

Usage: `./create-sandbox-repos.sh [<base-dir>]`

- `<base-dir>` is the root directory under which all sandbox repos are created.
  - If omitted, a new temporary directory is created via `mktemp -d -t gitover-sandbox`
    (e.g. `/tmp/gitover-sandbox.XXXXXX`), keeping repos outside the project tree and
    avoiding editor "dubious ownership" warnings for nested git repos.
  - If provided, the directory is created with `mkdir -p` if it does not yet exist, and
    the path is canonicalized to an absolute path before use.
- Re-running the script wipes and recreates all repos cleanly under the same `<base-dir>`.
- The script prints the absolute path of each created repo at the end — add them to
  gitover with the `A` key.
