# Implementation ToDo for Gitover Rust TUI

This is a living document.
New tasks are added as needed.
Done tasks are check-marked when implemented.
Checkmarked tasks are removed on demand to merge them into `features.md`.

## Bugs

- [x] Commit-dialog: Can't move the cursor left/right within commit-message,
      only able to append at the end of line or remove from end of the line.
- [x] Commit-dialog: Can't enter newline in commit-message,
      shift-enter just inserts a "j" and alt-enter does nothing.
- [x] Status-pane: stage or unstage a file, afterwards a different file is selected.
      Would expect that for those actions where the file still exists after the action
      ( only with a different status ) that the same file stays selected.

## Git Status Columns

- [ ]

## Git Status Pane

- [ ] 

## Git Branches Pane

- [ ]

## UX Polish

- [ ] In the file-picker popup, apply the sorting-flag for paths.
      ( blocked: ratatui-explorer sorts internally, no API to override,
      see https://github.com/tatounee/ratatui-explorer/issues/22 )

## UX Mouse Interaction

- [ ]

## Git Commit History Pane

- [ ]

## Git Details Pane

- [ ]

## Git Basic Operations

- [ ]

## Git Rebase Operation

- [ ] Rebase onto trunk branch: run `git rebase <trunkbranch>`
      Auto-stash before rebase, pop stash after rebase completes
- [ ] Rebase controls when rebase is in progress: continue / skip / abort

## Per-file Actions

- [ ]

## Status-File / Commit-History-File Diff

- [ ]

## Testing & Quality

- [ ]

## Configuration

- [x] Enhance "state" configuration to store repositories in named sections instead of
      flat list of paths.
  - [x] A section can have a name and arbitrary repository paths
  - [x] There is a default / unnamed section
    - [x] the default section doesn't use a section-title-row in repositories-pane
    - [x] the default section can't be renamed or removed
    - [x] if the default section has associated repositories, it will be the first
          section to be shown in repositories-pane
  - [x] Repositories pane shows case-insensitive alphabetically name-sorted sections,
    - sections repositories are shown as 2-spaces indented sub-rows
    - sections repositories are sorted by their paths ( using the setting
      `case_sensitive_path_sorting` )
  - [x] column headers in repository-pane are unchanged, don't introduce a new column for
        the section-name  
  - [x] When adding a repository it is added to current section ( the section that is
        selected or from the repo that is currently selected )
    - [x] If there are only named-sections, adding a repo to the default-section is
          only possible by first adding to current section and then moving the repo
          to the default section afterwards manually via action menu.
    - [x] Add a hint to the add-dialog when there are only named-sections,
          to give the user a hint how he can move the repo into the default-section
          afterwards.
  - [x] Repositiories-pane action-menu shows different actions, depending on whether
        current row is a section title or a repository title
  - [x] User can create a new section in the repositories-pane via action menu
    - [x] action checks that no duplicate section name ( case-insensitive ) can be entered
    - [x] Repository-pane selects new added section after the create-action
  - [x] User can rename current section in the repositories-pane via action menu
    - [x] action checks that no duplicate section name ( case-insensitive ) can be entered
    - [x] this action is only available when section-title-row is selected and
          current section is not default section
    - [x] After rename select the renamed section title row
  - [x] User can remove current section in the repositories-pane via action menu
    - [x] this action is only available when section-title-row is selected and
          current section is not default section
    - [x] Show confirmation dialog to user ("Remove section X? Its N repos will move
          to default." or "Remove empty section X?")
    - [x] All repos under the removed section will be moved to the default section
    - [x] After removal make the first repo in default section the current repo,
          or select nothing if default section has no repos
  - [x] User can move the currently selected repository to an existing section 
        via action menu
    - [x] this action is only available when repository-row is selected
    - [x] The action will present the user all named sections plus the default section,
          except the current section
      - [x] display default-section first ( if it is not the current section )
      - [x] display other section-names sorted case-insensitive alphabetically )
    - [x] Don't show the "move" action if there is only a default section
    - [x] Keep the section when all of its repos are moved to other sections
    - [x] Repository-pane keeps current/moved repo as selected after the move action
          Expand the target section to show the repository.
  - [x] Migrate existing repo-path-only-list state config and save it in the new format
    - [x] since there is only a default-section at start of migration, the state of the
          default-section is expanded.
  - [x] Sections cursor is not persistet in state config
  - [x] Repos cursor is not persistet in state config
  - [x] Repository-pane automatically selects first visible row overall
        (first default-section repo if any, otherwise first named-section title)
        after startup
  - [x] Repository named sections can be collapsed by "left" key-binding,
        expanded by "right" key-binding.
        Collapse-state is stored per section in the state config.
        Default section can't be collapsed, stays expanded all the time.
    - [x] Add keybindings for expand/collapse of section to help dialog
    - [x] Use ▶/▼ prefix on section title to show collapse-state of section
    - [x] A collapsed section can be removed/renamed via action menu
    - [x] Default section is always expanded, can never be collapsed
      - [x] expand/collapse key handler ignores key presses for default section
  - [x] Repository-pane title shows the current section title
    - [x] Use "Repositories ( <section-name> )" when current section is a named section.
          Adjust the pane title when current row is a section-title or a repo-title.
    - [x] Use "Repositories" when current row is a repo within the default-section.
  - [x] Section-title row shows an aggregated change summary when the section is collapsed.
    - [x] Status column: "N dirty" (dirty color) when any repo has local changes, else "clean" (clean color)
    - [x] Upstream column: "N ↑↓" (warning color) when any repo has upstream divergence, else "-"
    - [x] Trunk column: "N ↑↓" (trunk-behind or warning color) when any repo diverges from trunk, else "-"
    - [x] Activity column: spinner + op label (or "N active") when any repos are being operated on
    - [x] Summary is hidden when the section is expanded (per-repo rows show full details)
  - [x] When a section-title row is selected, dependent panes show appropriate empty state
    - [x] File Status pane shows "no repository selected" placeholder and a generic title
    - [x] History pane clears its commit list (no stale commits from the previously selected repo)
  - [x] Action menus (section-title and repo-row) have a minimum width of 40 % of the terminal
        so labels are not truncated on wide terminals
  - [x] Pressing "f" on a section-title row fetches all repos in that section
  - [x] Pressing "r" on a section-title row refreshes all repos in that section
  - [x] Repo info refresh ("r" key) shows activity indicator per repo during async scan
    - [x] Each refreshed repo shows "scan" spinner in the Activity column while in progress
    - [x] Applies both when pressing "r" on a repo row (full refresh) and on a section title

## Documentation & Release

- [ ]
