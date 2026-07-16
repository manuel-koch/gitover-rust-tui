# Recent / Remaining Implementation Todo Tasks and Bugs

This is a living document, see [todo-workflow](./todo-workflow.md).

## Bugs

- [x] Activity indicator of a repository ( e.g. for fetching, pulling )
      vanish after short delay although the action is not done yet
      ( long running pull ).
      Activity indicator should be visible as long as an activity in in progress.

- [x] j/k keybinding doesn't seem to work in file-picker dialog to
      add a repository. The help hint is "↑↓/jk" but should be
      "↑↓/kj".

## UX Polish

- [x] Include the currently selected repository info in the repositories-pane
      title
  - "Repositories ( <repo-group> - <repo-name> )" when there is a
    repo selected
  - "Repositories ( <repo-group> )" when there is no repo selected

- [x] Action menu for selected repository doesn't include an action to
      remove the repo from the app.

## Git Rebase Operation

> **Design needs rework.** The action-menu entry, conflict-resolution flow,
> rebase-in-progress state tracking, and abort/skip UX are not specified yet;
> the items below are placeholders, not an implementation plan. Rework the
> design before starting on these tasks.

- [ ] Rebase onto trunk branch: run `git rebase <trunkbranch>`
      Auto-stash before rebase, pop stash after rebase completes
- [ ] Rebase controls when rebase is in progress: continue / skip / abort

## Check for new app release

- [x] **HTTP dependency**: Add `ureq` (sync, pure Rust HTTP client) for querying GitHub Releases API
      - `GET https://api.github.com/repos/manuelkoch/gitover-rust-tui/releases/latest`
      - No async runtime needed — matches existing `std::thread::spawn` pattern in ops.rs
- [x] **Config**: Add `general.release_check_interval` (Option<u64>, default 86400s = 24h, 0 = disabled) to GeneralConfig
      - Use "0" to disable the check entirely
- [x] **OpRequest/OpResult**: Add `CheckRelease` variant; background thread fetches & parses `tag_name` from JSON response
      - Silent retry on network/rate-limit errors (no error log spam)
- [x] **Timer**: Add `next_release_check: Option<Instant>`, `is_release_check_due()`, `reset_release_check_timer()` to App
      - Check fires in event loop alongside auto-fetch timer
- [x] **Popup notification**: On first detection of newer version, show auto-dismissing PopupMessage
      - `"New release v0.9.0 available! → github.com/manuelkoch/gitover-rust-tui/releases"`
- [x] **Help dialog entry**: Add "new version available" hint to the help overlay, shown unconditionally whenever user opens it
      - Persistent visual indicator (not auto-dismissing) so user sees it whenever they check shortcuts
- [x] **App title indicator**: Show a visual indicator (`✦`) after the version in the header title when a new release is available
- [x] **Per-version dismissal**: Track dismissed release version so popup doesn't repeat for same version
      - Header hint remains visible; popup only on first detection
- [x] **State persistence (optional)**: Save last-seen-latest-version to state file to survive restarts
- [x] **Update config schema**: Add `general.release_check_interval` to `docs/config.schema.json`
- [x] **Update state schema** (if state persistence implemented): Add `last_notified_release_version` to `docs/state.schema.json`
- [x] **Documentation**: Add feature entry to `docs/features.md` under new "Release Notifications" section
- [x] **Update README.md**: Mention release-check feature if appropriate

## Blocked

Tasks blocked on an external dependency stay here until unblocked.
Do NOT implement workarounds without explicit user direction.

- [ ] In the file-picker popup, apply the sorting-flag for paths.
      ( blocked: ratatui-explorer sorts internally, no API to override,
      see https://github.com/tatounee/ratatui-explorer/issues/22 )
