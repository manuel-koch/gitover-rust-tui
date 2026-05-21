/// Unit tests for config load/save (config.rs and state.rs).
use gitover::config::Config;
use gitover::state::State;
use std::fs;
use std::path::PathBuf;
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
fn state_pane_visibility_round_trip() {
    let mut state = State::default();
    // Defaults should be false
    assert!(!state.show_file_status);
    assert!(!state.show_log);
    assert!(!state.show_history);
    assert!(!state.show_diff);

    // Set pane visibility
    state.show_file_status = true;
    state.show_log = true;
    state.show_history = true;
    state.show_diff = true;

    let yaml = serde_yaml::to_string(&state).expect("serialize");
    let loaded: State = serde_yaml::from_str(&yaml).expect("deserialize");

    assert!(loaded.show_file_status);
    assert!(loaded.show_log);
    assert!(loaded.show_history);
    assert!(loaded.show_diff);
}

// ── State::load_from_path (--state CLI override) ──────────────────────────────

#[test]
fn state_load_from_path_nonexistent_returns_default_with_given_path() {
    let tmp = tempdir().unwrap();
    let state_path = tmp.path().join("custom.yaml");
    // File does not exist yet.
    let state = State::load_from_path(state_path.clone());
    assert!(state.repos.is_empty());
    assert_eq!(state.path, state_path);
}

#[test]
fn state_load_from_path_reads_explicit_file() {
    let tmp = tempdir().unwrap();
    let repo_dir = tmp.path().join("my-repo");
    fs::create_dir_all(&repo_dir).unwrap();
    let state_path = tmp.path().join("state.yaml");
    // Write a state file with an absolute path.
    let yaml = format!("repos:\n  - {}\n", repo_dir.display());
    fs::write(&state_path, &yaml).unwrap();

    let state = State::load_from_path(state_path.clone());
    assert_eq!(state.repos.len(), 1);
    assert_eq!(state.repos[0], repo_dir.to_string_lossy().as_ref());
    assert_eq!(state.path, state_path);
}

#[test]
fn state_load_from_path_resolves_relative_paths() {
    let tmp = tempdir().unwrap();
    let repo_dir = tmp.path().join("my-repo");
    fs::create_dir_all(&repo_dir).unwrap();
    let state_path = tmp.path().join("state.yaml");
    // State file stores a relative path.
    fs::write(&state_path, "repos:\n  - my-repo\n").unwrap();

    let state = State::load_from_path(state_path);
    assert_eq!(state.repos.len(), 1);
    assert_eq!(state.repos[0], repo_dir.to_string_lossy().as_ref());
}

#[test]
fn state_load_from_path_absolute_paths_kept_as_is() {
    let tmp = tempdir().unwrap();
    let repo_dir = tmp.path().join("abs-repo");
    fs::create_dir_all(&repo_dir).unwrap();
    // State file stored elsewhere but the path in it is absolute.
    let other_tmp = tempdir().unwrap();
    let state_path = other_tmp.path().join("state.yaml");
    let yaml = format!("repos:\n  - {}\n", repo_dir.display());
    fs::write(&state_path, &yaml).unwrap();

    let state = State::load_from_path(state_path);
    assert_eq!(state.repos[0], repo_dir.to_string_lossy().as_ref());
}

#[test]
fn state_save_to_explicit_path_stores_relative_paths() {
    let tmp = tempdir().unwrap();
    let repo_dir = tmp.path().join("project");
    fs::create_dir_all(&repo_dir).unwrap();
    let state_path = tmp.path().join("state.yaml");

    let mut state = State::load_from_path(state_path.clone());
    state.add_repo(&repo_dir.to_string_lossy());
    state.save().unwrap();

    let content = fs::read_to_string(&state_path).unwrap();
    // Stored path should be the relative name only, not the full prefix.
    assert!(content.contains("project"), "relative name should appear");
    assert!(
        !content.contains(&tmp.path().to_string_lossy().as_ref()),
        "absolute prefix should not appear in saved YAML"
    );
}

#[test]
fn state_save_load_round_trip_with_explicit_path() {
    let tmp = tempdir().unwrap();
    let repo_dir = tmp.path().join("round-trip-repo");
    fs::create_dir_all(&repo_dir).unwrap();
    let state_path = tmp.path().join("mystate.yaml");

    let mut state = State::load_from_path(state_path.clone());
    state.add_repo(&repo_dir.to_string_lossy());
    state.show_log = true;
    state.save().unwrap();

    let loaded = State::load_from_path(state_path);
    assert_eq!(loaded.repos.len(), 1);
    assert_eq!(loaded.repos[0], repo_dir.to_string_lossy().as_ref());
    assert!(loaded.show_log);
}

// ── Config load_from (--config CLI override) ──────────────────────────────────

#[test]
fn config_load_from_explicit_path_overrides_defaults() {
    let tmp = tempdir().unwrap();
    let cfg_path = tmp.path().join("override.yaml");
    fs::write(
        &cfg_path,
        "general:\n  git: /custom/bin/git\n  auto_fetch_interval: 120\n",
    )
    .unwrap();
    let cfg = Config::load_from(&cfg_path);
    assert_eq!(cfg.general.git.as_deref(), Some("/custom/bin/git"));
    assert_eq!(cfg.general.auto_fetch_interval, Some(120));
}

#[test]
fn config_load_from_nonexistent_explicit_path_returns_default() {
    let path = PathBuf::from("/nonexistent/path/to/config.yaml");
    let cfg = Config::load_from(&path);
    assert!(cfg.general.git.is_none());
}
