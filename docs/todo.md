# Implementation ToDo for Gitover Rust TUI

This is a living document.
New tasks are added as needed.
Done tasks are check-marked when implemented.
Checkmarked tasks are removed on demand to merge them into `features.md`.

## Bugs

- [ ]

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

- [ ] Enhance "state" configuration to store repositories in named sections instead of
      flat list of paths.
  - [ ] A section can have a name and arbitrary repository paths
  - [ ] There is a default / unnamed section
    - [ ] the default section doesn't use a section-title-row in repositories-pane
    - [ ] the default section can't be renamed or removed
    - [ ] if the default section has associated repositories, it will be the first
          section to be shown in repositories-pane
  - [ ] Repositories pane shows case-insensitive alphabetically name-sorted sections,
    - sections repositories are shown as 2-spaces indented sub-rows
    - sections repositories are sorted by their paths ( using the setting
      `case_sensitive_path_sorting` )
  - [ ] column headers in repository-pane are unchanged, don't introduce a new column for
        the section-name  
  - [ ] When adding a repository it is added to current section ( the section that is
        selected or from the repo that is currently selected )
    - If there are only named-sections, then adding repo to default-section is
      only possible by first adding to current section and then moving the repo to
      the default section afterwards manually.
    - add a hint to the add-dialog when there are only named-sections,
      to give the user a hint how he can move the repo into the defaut-section
      aftwards.
  - [ ] Repositiories-pane action-menu shows different actions, depending on whether
        current row is a section title or a repository title
  - [ ] User can create a new section in the repositories-pane via action menu
    - [ ] action checks that no duplicate section name ( case-insensitive ) can be entered
    - [ ] Repository-pane selects new added section after the create-action
  - [ ] User can rename current section in the repositories-pane via action menu
    - [ ] action checks that no duplicate section name ( case-insensitive ) can be entered
    - [ ] this action is only available when section-title-row is selected and
          current section is not default section
    - [ ] After rename select the renamed section title row
  - [ ] User can remove current section in the repositories-pane via action menu
    - [ ] this action is only available when section-title-row is selected and
          current section is not default section
    - [ ] Show confirmation dialog to user ("Remove section X? Its N repos will move
          to default." or "Remove empty section X?")
    - [ ] All repos under the removed section will be moved to the default section
    - [ ] After removal make the first repo in default section the current repo,
          or select nothing if default section has no repos
  - [ ] User can move the currently selected repository to an existing section 
        via action menu
    - [ ] this action is only available when repository-row is selected
    - [ ] The action will present the user all named sections plus the default section,
          except the current section
      - [ ] display default-section first ( if it is not the current section )
      - [ ] display other section-names sorted case-insensitive alphabetically )
    - [ ] Don't show the "move" action if there is only a default section
    - [ ] Keep the section when all of its repos are moved to other sections
    - [ ] Repository-pane keeps current/moved repo as selected after the move action
          Expand the target section to show the repository.
  - [ ] Migrate existing repo-path-only-list state config and save it in the new format
    - [ ] since there is only a default-section at start of migration, the state of the
          default-section is expanded.
  - [ ] Sections cursor is not persistet in state config
  - [ ] Repos cursor is not persistet in state config
  - [ ] Repository-pane automatically selects first visible row overall
        (first default-section repo if any, otherwise first named-section title)
        after startup
  - [ ] Repository named sections can be collapsed by "left" key-binding,
        expanded by "right" key-binding.
        Collapse-state is stored per section in the state config.
        Default section can't be collapsed, stays expanded all the time.
    - [ ] Add keybindings for expand/collapse of section to help dialog
    - [ ] Use ▶/▼ prefix on section title to show collapse-state of section
    - [ ] A collapsed section can be removed/renamed via action menu
    - [ ] Default section is always expanded, can never be collapsed
      - [ ] expand/collapse key handler ignores key presses for default section
  - [ ] Repository-pane title shows the current section title
    - [ ] Use "Repositories ( <section-name> )" when current section is a named section.
          Adjust the pane title when current row is a section-title or a repo-title.
    - [ ] Use "Repositories" when current row is a repo within the default-section.
  
## Documentation & Release

- [ ]
