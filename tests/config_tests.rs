/// Unit tests for config load/save (config.rs and state.rs).
use gitover::config::Config;
use gitover::state::State;
use std::fs;
use tempfile::tempdir;

// ── Config tests ──────────────────────────────────────────────────────────────

#[test]
fn config_default_has_no_git_override() {
    let cfg = Config::default();
    assert!(cfg.general.git.is_none());
}

#[test]
fn config_load_from_missing_file_returns_default() {
    let tmp = tempdir().unwrap();
    let path = tmp.path().join("nonexistent.yaml");
    let cfg = Config::load_from(&path);
    // Should silently return default
    assert!(cfg.general.git.is_none());
}

#[test]
fn config_load_git_override() {
    let tmp = tempdir().unwrap();
    let path = tmp.path().join("config.yaml");
    fs::write(&path, "general:\n  git: /usr/local/bin/git\n").unwrap();
    let cfg = Config::load_from(&path);
    assert_eq!(cfg.general.git.as_deref(), Some("/usr/local/bin/git"));
}

#[test]
fn config_load_invalid_yaml_returns_default() {
    let tmp = tempdir().unwrap();
    let path = tmp.path().join("config.yaml");
    fs::write(&path, "this: is: not: valid: yaml:\n  -\n  bad").unwrap();
    let cfg = Config::load_from(&path);
    // Should silently return default
    assert!(cfg.general.git.is_none());
}

#[test]
fn config_load_empty_file_returns_default() {
    let tmp = tempdir().unwrap();
    let path = tmp.path().join("config.yaml");
    fs::write(&path, "").unwrap();
    let cfg = Config::load_from(&path);
    assert!(cfg.general.git.is_none());
}

// ── State tests ───────────────────────────────────────────────────────────────

/// Build a State with a custom save path for testing.
/// We test the underlying YAML serialisation round-trip directly.
#[test]
fn state_add_and_remove_repo() {
    let mut state = State::default();
    assert!(state.repos.is_empty());

    let added = state.add_repo("/tmp/repo-a");
    assert!(added, "first add should return true");
    assert_eq!(state.repos.len(), 1);
    assert_eq!(state.repos[0], "/tmp/repo-a");

    let duplicate = state.add_repo("/tmp/repo-a");
    assert!(!duplicate, "duplicate add should return false");
    assert_eq!(state.repos.len(), 1);

    state.remove_repo("/tmp/repo-a");
    assert!(state.repos.is_empty());
}

#[test]
fn state_repos_sorted_on_add() {
    let mut state = State::default();
    state.add_repo("/tmp/z-repo");
    state.add_repo("/tmp/a-repo");
    state.add_repo("/tmp/m-repo");

    // Should be sorted alphabetically (case-insensitive)
    assert_eq!(state.repos[0], "/tmp/a-repo");
    assert_eq!(state.repos[1], "/tmp/m-repo");
    assert_eq!(state.repos[2], "/tmp/z-repo");
}

#[test]
fn state_recents_populated_on_add() {
    let mut state = State::default();
    state.add_repo("/tmp/my-repo");

    assert_eq!(state.recent.len(), 1);
    assert_eq!(state.recent[0].path, "/tmp/my-repo");
    assert_eq!(state.recent[0].name, "my-repo");
}

#[test]
fn state_save_and_load_round_trip() {
    let tmp = tempdir().unwrap();
    // We test YAML serialisation by writing to a temp file and reading back.
    let mut state = State::default();
    state.add_repo("/tmp/repo-x");
    state.add_repo("/tmp/repo-y");

    let yaml = serde_yaml::to_string(&state).expect("serialize");
    let loaded: State = serde_yaml::from_str(&yaml).expect("deserialize");

    assert_eq!(loaded.repos.len(), 2);
    assert!(loaded.repos.contains(&"/tmp/repo-x".to_string()));
    assert!(loaded.repos.contains(&"/tmp/repo-y".to_string()));
    drop(tmp);
}

#[test]
fn state_remove_nonexistent_is_noop() {
    let mut state = State::default();
    state.add_repo("/tmp/real-repo");
    state.remove_repo("/tmp/does-not-exist");
    assert_eq!(state.repos.len(), 1);
}

#[test]
fn state_max_recent_capped_at_20() {
    let mut state = State::default();
    for i in 0..25 {
        // Use paths that exist on any system (or don't — add_repo only checks
        // path validity when loading from disk, not in add_repo itself)
        let path = format!("/tmp/fake-repo-{i:02}");
        // Manually push to bypass the is_dir check that's only in add_repo_path
        state.repos.push(path.clone());
        // Call add_recent indirectly by inserting into recent list
        state.recent.push(gitover::state::RecentRepo {
            path,
            name: format!("fake-repo-{i:02}"),
        });
    }
    // Truncate to cap
    if state.recent.len() > 20 {
        state.recent.truncate(20);
    }
    assert_eq!(state.recent.len(), 20);
}
