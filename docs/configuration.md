# Configuration

Reference for gitover's CLI flags, config file, and persisted state file.

## CLI options/flags

| Flag | Description |
|------|-------------|
| `--version` | Show version & built info and exit |
| `--config <path>` | Override the config file location (skips CWD-walk and global fallback) |
| `--state <path>` | Override the state file location (skips CWD-walk and global fallback); file is created on first save if absent |
| `--debug-log <path>` | Enable debug logging to a file; path supports `~` and `${VAR}` expansion. Appended to if it already exists, created otherwise. Overrides `general.debug_log` from config. App terminates if any variable cannot be resolved |

## Config file

Full JSON Schema is in [`docs/config.schema.json`](./config.schema.json).

- Config file lookup: searches for `gitover.config.yaml` starting from the
  current working directory and walking up to the filesystem root; falls back
  to `~/.config/gitover/config.yaml` if not found.
  Missing file is valid — default config is used.

### `general`

- `general.git`: override the path to the git executable
- `general.auto_fetch_interval`: interval in seconds for automatic background
  fetch of all repos (default: 600 = 10 minutes; set to 0 to disable automatic fetch)
- `general.debug_log`: path to the debug log file; supports `~` and `${VAR}`
  expansion. Appended to if it already exists, created otherwise. Can be overridden
  by `--debug-log` CLI flag. App terminates if any variable cannot be resolved
- `general.case_sensitive_path_sorting`: when `true`, paths are sorted case-sensitively
  in the Status Details pane (file list) and in the Git History pane
  (per-commit file sub-rows).
  Also controls the sort order of repo paths **within each section** of the
  Repositories pane.
  Defaults to `false` (case-insensitive). Does not affect section-name ordering.
  Named sections are always sorted case-insensitively, regardless of this setting.
- `general.release_check_interval`: interval in seconds for checking the
  GitHub Releases API for a new app version (default: 86400 = 24 hours; set to 0 to disable)

### `repo_commands`

List of commands that can be run for the current repository. Each command is
appended to the per-repo action menu (see [Features](./features.md#custom-repo-commands)).

- `repo_commands[].name`: Description of the command, will be shown in action menu
- `repo_commands[].cmd`: The command line to be executed; supports `${VAR}`
  substitution in two steps:
  1. Repo-dependent variables: `${ROOT}` (git root path), `${BRANCH}` (current branch name)
  2. Environment variables: any `${VAR}` still unresolved is looked up in the process environment
  The command is not executed if any variable cannot be resolved
- `repo_commands[].background`: Boolean flag whether the `cmd` should be executed
  in background

## State file

Full JSON Schema is in [`docs/state.schema.json`](./state.schema.json).

Persisted app state (repo list, pane visibility):

- State file lookup: searches for `gitover.state.yaml` starting from CWD and
  walking up to root; falls back to `~/.config/gitover/state.yaml` if not found.
- Relative paths in the state file are resolved against the directory containing
  the state file.
- When saving, repo paths that are under the state file's directory are stored
  as relative paths, keeping per-project state files portable.
- Repositories are organised into named sections; a default (unnamed) section
  always exists, is always shown first, and cannot be renamed, removed, or collapsed
- Section collapse state is stored per named section; the cursor position
  within the repository list is not persisted
