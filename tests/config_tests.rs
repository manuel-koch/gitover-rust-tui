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

#[test]
fn state_add_and_remove_repo() {
    let mut state = State::default();
    assert!(!state.has_any_repos());

    let added = state.add_repo("/tmp/repo-a");
    assert!(added, "first add should return true");
    assert_eq!(state.all_repos_flat().len(), 1);
    assert_eq!(state.sections[0].repos[0], "/tmp/repo-a");

    let duplicate = state.add_repo("/tmp/repo-a");
    assert!(!duplicate, "duplicate add should return false");
    assert_eq!(state.all_repos_flat().len(), 1);

    state.remove_repo("/tmp/repo-a");
    assert!(!state.has_any_repos());
}

#[test]
fn state_repos_sorted_on_add() {
    let mut state = State::default();
    state.add_repo("/tmp/z-repo");
    state.add_repo("/tmp/a-repo");
    state.add_repo("/tmp/m-repo");

    // Should be sorted alphabetically (case-insensitive) within the default section.
    let repos = &state.sections[0].repos;
    assert_eq!(repos[0], "/tmp/a-repo");
    assert_eq!(repos[1], "/tmp/m-repo");
    assert_eq!(repos[2], "/tmp/z-repo");
}

#[test]
fn state_save_and_load_round_trip() {
    let tmp = tempdir().unwrap();
    let mut state = State::default();
    state.add_repo("/tmp/repo-x");
    state.add_repo("/tmp/repo-y");

    let yaml = serde_yaml::to_string(&state).expect("serialize");
    let loaded: State = serde_yaml::from_str(&yaml).expect("deserialize");

    assert_eq!(loaded.all_repos_flat().len(), 2);
    assert!(loaded.all_repos_flat().contains(&"/tmp/repo-x".to_string()));
    assert!(loaded.all_repos_flat().contains(&"/tmp/repo-y".to_string()));
    drop(tmp);
}

#[test]
fn state_remove_nonexistent_is_noop() {
    let mut state = State::default();
    state.add_repo("/tmp/real-repo");
    state.remove_repo("/tmp/does-not-exist");
    assert_eq!(state.all_repos_flat().len(), 1);
}

#[test]
fn state_pane_visibility_round_trip() {
    let mut state = State::default();
    // Defaults should be false
    assert!(!state.show_file_status);
    assert!(!state.show_log);
    assert!(!state.show_history);
    assert!(!state.show_details);

    // Set pane visibility
    state.show_file_status = true;
    state.show_log = true;
    state.show_history = true;
    state.show_details = true;

    let yaml = serde_yaml::to_string(&state).expect("serialize");
    let loaded: State = serde_yaml::from_str(&yaml).expect("deserialize");

    assert!(loaded.show_file_status);
    assert!(loaded.show_log);
    assert!(loaded.show_history);
    assert!(loaded.show_details);
}

// ── State::load_from_path (--state CLI override) ──────────────────────────────

#[test]
fn state_load_from_path_nonexistent_returns_default_with_given_path() {
    let tmp = tempdir().unwrap();
    let state_path = tmp.path().join("custom.yaml");
    // File does not exist yet.
    let state = State::load_from_path(state_path.clone());
    assert!(!state.has_any_repos());
    assert_eq!(state.path, state_path);
}

#[test]
fn state_load_from_path_reads_explicit_file() {
    let tmp = tempdir().unwrap();
    let repo_dir = tmp.path().join("my-repo");
    fs::create_dir_all(&repo_dir).unwrap();
    let state_path = tmp.path().join("state.yaml");
    // Write a legacy flat-repos state file.
    let yaml = format!("repos:\n  - {}\n", repo_dir.display());
    fs::write(&state_path, &yaml).unwrap();

    let state = State::load_from_path(state_path.clone());
    let flat = state.all_repos_flat();
    assert_eq!(flat.len(), 1);
    assert_eq!(flat[0], repo_dir.to_string_lossy().as_ref());
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
    let flat = state.all_repos_flat();
    assert_eq!(flat.len(), 1);
    assert_eq!(flat[0], repo_dir.to_string_lossy().as_ref());
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
    assert_eq!(
        state.all_repos_flat()[0],
        repo_dir.to_string_lossy().as_ref()
    );
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
    let flat = loaded.all_repos_flat();
    assert_eq!(flat.len(), 1);
    assert_eq!(flat[0], repo_dir.to_string_lossy().as_ref());
    assert!(loaded.show_log);
}

// ── Section management tests ──────────────────────────────────────────────────

#[test]
fn state_add_section_and_repos() {
    let mut state = State::default();
    let section_idx = state.add_section("Work".to_string()).unwrap();
    assert_eq!(section_idx, 1);
    assert_eq!(state.sections.len(), 2);
    assert_eq!(state.sections[1].name.as_deref(), Some("Work"));

    // Add repo to the named section.
    let added = state.add_repo_to_section("/tmp/work-repo", 1);
    assert!(added);
    assert_eq!(state.sections[1].repos.len(), 1);

    // Flat view: default section (empty) + Work section.
    assert_eq!(state.all_repos_flat(), vec!["/tmp/work-repo".to_string()]);
}

#[test]
fn state_add_section_duplicate_name_is_rejected() {
    let mut state = State::default();
    state.add_section("Work".to_string()).unwrap();
    // Same name, different case.
    assert!(state.add_section("work".to_string()).is_none());
}

#[test]
fn state_named_sections_sorted_alphabetically() {
    let mut state = State::default();
    state.add_section("Zebra".to_string()).unwrap();
    state.add_section("Alpha".to_string()).unwrap();
    state.add_section("Middle".to_string()).unwrap();

    assert_eq!(state.sections[1].name.as_deref(), Some("Alpha"));
    assert_eq!(state.sections[2].name.as_deref(), Some("Middle"));
    assert_eq!(state.sections[3].name.as_deref(), Some("Zebra"));
}

#[test]
fn state_rename_section_updates_name_and_resorts() {
    let mut state = State::default();
    state.add_section("Aardvark".to_string()).unwrap();
    state.add_section("Zebra".to_string()).unwrap();

    // Rename Aardvark → Monkey (should move between Aardvark and Zebra).
    let new_idx = state.rename_section(1, "Monkey".to_string()).unwrap();
    assert_eq!(state.sections[new_idx].name.as_deref(), Some("Monkey"));
}

#[test]
fn state_remove_section_moves_repos_to_default() {
    let mut state = State::default();
    state.add_section("Work".to_string()).unwrap();
    state.add_repo_to_section("/tmp/repo-a", 1);
    state.add_repo_to_section("/tmp/repo-b", 1);

    state.remove_section(1);
    // Both repos should now be in the default section.
    assert_eq!(state.sections.len(), 1);
    assert_eq!(state.sections[0].repos.len(), 2);
    assert!(state.sections[0].repos.contains(&"/tmp/repo-a".to_string()));
    assert!(state.sections[0].repos.contains(&"/tmp/repo-b".to_string()));
}

#[test]
fn state_move_repo_to_section() {
    let mut state = State::default();
    state.add_repo("/tmp/repo-a");
    state.add_section("Work".to_string()).unwrap();

    state.move_repo_to_section("/tmp/repo-a", 1);
    assert!(state.sections[0].repos.is_empty());
    assert_eq!(state.sections[1].repos[0], "/tmp/repo-a");
}

#[test]
fn state_section_collapse_persists_in_yaml() {
    let mut state = State::default();
    state.add_section("Work".to_string()).unwrap();
    state.sections[1].collapsed = true;

    let yaml = serde_yaml::to_string(&state).expect("serialize");
    let loaded: State = serde_yaml::from_str(&yaml).expect("deserialize");

    assert!(loaded.sections[1].collapsed);
}

#[test]
fn state_migrate_legacy_flat_repos_to_default_section() {
    let tmp = tempdir().unwrap();
    let repo_dir = tmp.path().join("legacy-repo");
    fs::create_dir_all(&repo_dir).unwrap();
    let state_path = tmp.path().join("state.yaml");
    let yaml = format!("repos:\n  - {}\n", repo_dir.display());
    fs::write(&state_path, &yaml).unwrap();

    let state = State::load_from_path(state_path);
    // Migrated to default section.
    assert_eq!(state.sections.len(), 1);
    assert!(state.sections[0].name.is_none());
    assert_eq!(state.sections[0].repos.len(), 1);
    assert_eq!(state.all_repos_flat().len(), 1);
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
