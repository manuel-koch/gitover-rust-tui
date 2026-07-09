# Implementation ToDo for Gitover Rust TUI

This is a living document.
New tasks are added as needed.
Done tasks are check-marked when implemented.
Checkmarked tasks are removed on demand to merge them into `features.md`.

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

- [ ] In the file-picker popup, apply the sorting-flag for paths.
      ( blocked: ratatui-explorer sorts internally, no API to override,
      see https://github.com/tatounee/ratatui-explorer/issues/22 )

## Git Rebase Operation

- [ ] Rebase onto trunk branch: run `git rebase <trunkbranch>`
      Auto-stash before rebase, pop stash after rebase completes
- [ ] Rebase controls when rebase is in progress: continue / skip / abort

## Check for new app release

- [ ] **HTTP dependency**: Add `ureq` (sync, pure Rust HTTP client) for querying GitHub Releases API
      - `GET https://api.github.com/repos/manuelkoch/gitover-rust-tui/releases/latest`
      - No async runtime needed — matches existing `std::thread::spawn` pattern in ops.rs
- [ ] **Config**: Add `general.release_check_interval` (Option<u64>, default 86400s = 24h, 0 = disabled) to GeneralConfig
      - Use "0" to disable the check entirely
- [ ] **OpRequest/OpResult**: Add `CheckRelease` variant; background thread fetches & parses `tag_name` from JSON response
      - Silent retry on network/rate-limit errors (no error log spam)
- [ ] **Timer**: Add `next_release_check: Option<Instant>`, `is_release_check_due()`, `reset_release_check_timer()` to App
      - Check fires in event loop alongside auto-fetch timer
- [ ] **Popup notification**: On first detection of newer version, show auto-dismissing PopupMessage
      - `"New release v0.9.0 available! → github.com/manuelkoch/gitover-rust-tui/releases"`
- [ ] **Help dialog entry**: Add "new version available" hint to the help overlay, shown unconditionally whenever user opens it
      - Persistent visual indicator (not auto-dismissing) so user sees it whenever they check shortcuts
- [ ] **App title indicator**: Show a visual indicator (e.g. `◆` or `✦`) after the version in the header title when a new release is available
- [ ] **Per-version dismissal**: Track dismissed release version so popup doesn't repeat for same version
      - Header hint remains visible; popup only on first detection
- [ ] **State persistence (optional)**: Save last-seen-latest-version to state file to survive restarts
- [ ] **Update config schema**: Add `general.release_check_interval` to `docs/config.schema.json`
- [ ] **Update state schema** (if state persistence implemented): Add `last_seen_release_version` to `docs/state.schema.json`
- [ ] **Documentation**: Add feature entry to `docs/features.md` under new "Release Notifications" section
- [ ] **Update README.md**: Mention release-check feature if appropriate
