/// Unit and integration tests for git.rs.
///
/// The unit tests cover pure logic (FileStatusKind methods, RepoStatus helpers,
/// file sort ordering) that require no filesystem access.
///
/// The integration tests create a real temporary git repository via git2 and
/// verify that `get_repo_status` returns sensible results for several scenarios.
use gitover::git::{
    get_branches_with_ahead_behind, get_commit_file_diff, get_commit_history, get_file_diff,
    get_head_commit_file_count, get_head_commit_message, get_repo_status,
    get_untracked_file_content, DeltaKind, FileEntry, FileStatusKind, HistoryFilter, RepoStatus,
};
use std::fs;
use std::path::PathBuf;

// ── Unit tests ───────────────────────────────────────────────────────────────

#[test]
fn file_status_kind_codes() {
    assert_eq!(FileStatusKind::Staged.code(), "S");
    assert_eq!(FileStatusKind::Modified.code(), "M");
    assert_eq!(FileStatusKind::Deleted.code(), "D");
    assert_eq!(FileStatusKind::Conflict.code(), "C");
    assert_eq!(FileStatusKind::Untracked.code(), "U");
}

#[test]
fn file_status_kind_labels() {
    assert_eq!(FileStatusKind::Staged.label(), "staged");
    assert_eq!(FileStatusKind::Modified.label(), "modified");
    assert_eq!(FileStatusKind::Deleted.label(), "deleted");
    assert_eq!(FileStatusKind::Conflict.label(), "conflict");
    assert_eq!(FileStatusKind::Untracked.label(), "untracked");
}

#[test]
fn file_status_sort_priority_order() {
    // Conflict < Staged < Modified < Deleted < Untracked
    assert!(FileStatusKind::Conflict.sort_priority() < FileStatusKind::Staged.sort_priority());
    assert!(FileStatusKind::Staged.sort_priority() < FileStatusKind::Modified.sort_priority());
    assert!(FileStatusKind::Modified.sort_priority() < FileStatusKind::Deleted.sort_priority());
    assert!(FileStatusKind::Deleted.sort_priority() < FileStatusKind::Untracked.sort_priority());
}

#[test]
fn delta_kind_codes() {
    assert_eq!(DeltaKind::Added.code(), "A");
    assert_eq!(DeltaKind::Modified.code(), "M");
    assert_eq!(DeltaKind::Deleted.code(), "D");
    assert_eq!(DeltaKind::Renamed.code(), "R");
    assert_eq!(DeltaKind::Other.code(), "?");
}

#[test]
fn history_filter_label_full_is_empty() {
    assert_eq!(HistoryFilter::Full.label(), "");
}

#[test]
fn history_filter_label_ahead_of() {
    assert_eq!(
        HistoryFilter::AheadOf("origin/main".to_string()).label(),
        "ahead of origin/main"
    );
}

#[test]
fn history_filter_label_behind_of() {
    assert_eq!(
        HistoryFilter::BehindOf("origin/main".to_string()).label(),
        "behind origin/main"
    );
}

#[test]
fn history_filter_label_branch_full() {
    assert_eq!(
        HistoryFilter::BranchFull("feature-x".to_string()).label(),
        "feature-x"
    );
}

#[test]
fn history_filter_label_branch_ahead_of() {
    let filter = HistoryFilter::BranchAheadOf {
        branch: "feat".to_string(),
        of: "origin/main".to_string(),
    };
    assert_eq!(filter.label(), "feat ahead of origin/main");
}

#[test]
fn history_filter_label_branch_behind_of() {
    let filter = HistoryFilter::BranchBehindOf {
        branch: "feat".to_string(),
        of: "origin/main".to_string(),
    };
    assert_eq!(filter.label(), "feat behind origin/main");
}

#[test]
fn repo_status_is_clean_with_no_changes() {
    let s = RepoStatus {
        path: "/tmp/repo".into(),
        branch: "main".into(),
        added: 0,
        modified: 0,
        staged: 0,
        deleted: 0,
        conflict: 0,
        upstream: None,
        trunk: None,
        local_branches: vec![],
        remote_only_branches: vec![],
        merged_branches: vec![],
        files: vec![],
        error: None,
    };
    assert!(s.is_clean());
}

#[test]
fn repo_status_is_not_clean_with_modified() {
    let s = RepoStatus {
        path: "/tmp/repo".into(),
        branch: "main".into(),
        added: 0,
        modified: 1,
        staged: 0,
        deleted: 0,
        conflict: 0,
        upstream: None,
        trunk: None,
        local_branches: vec![],
        remote_only_branches: vec![],
        merged_branches: vec![],
        files: vec![],
        error: None,
    };
    assert!(!s.is_clean());
}

#[test]
fn repo_status_is_not_clean_with_error() {
    let s = RepoStatus::error_entry("/tmp/bad", "not a repo");
    assert!(!s.is_clean());
    assert_eq!(s.error.as_deref(), Some("not a repo"));
    assert_eq!(s.path, "/tmp/bad");
}

#[test]
fn file_sort_order_conflict_first() {
    let mut files = vec![
        FileEntry {
            path: "c.txt".into(),
            status: FileStatusKind::Untracked,
        },
        FileEntry {
            path: "b.txt".into(),
            status: FileStatusKind::Modified,
        },
        FileEntry {
            path: "a.txt".into(),
            status: FileStatusKind::Conflict,
        },
        FileEntry {
            path: "d.txt".into(),
            status: FileStatusKind::Staged,
        },
        FileEntry {
            path: "e.txt".into(),
            status: FileStatusKind::Deleted,
        },
    ];

    files.sort_by(|a, b| {
        a.status
            .sort_priority()
            .cmp(&b.status.sort_priority())
            .then_with(|| a.path.cmp(&b.path))
    });

    assert_eq!(files[0].status, FileStatusKind::Conflict);
    assert_eq!(files[1].status, FileStatusKind::Staged);
    assert_eq!(files[2].status, FileStatusKind::Modified);
    assert_eq!(files[3].status, FileStatusKind::Deleted);
    assert_eq!(files[4].status, FileStatusKind::Untracked);
}

#[test]
fn file_sort_order_alphabetical_within_group() {
    let mut files = vec![
        FileEntry {
            path: "z.txt".into(),
            status: FileStatusKind::Untracked,
        },
        FileEntry {
            path: "a.txt".into(),
            status: FileStatusKind::Untracked,
        },
        FileEntry {
            path: "m.txt".into(),
            status: FileStatusKind::Untracked,
        },
    ];

    files.sort_by(|a, b| {
        a.status
            .sort_priority()
            .cmp(&b.status.sort_priority())
            .then_with(|| a.path.cmp(&b.path))
    });

    assert_eq!(files[0].path, "a.txt");
    assert_eq!(files[1].path, "m.txt");
    assert_eq!(files[2].path, "z.txt");
}

// ── Integration tests — real temp git repos ──────────────────────────────────

/// Create a minimal git repository at `dir` with one commit on branch `branch`.
/// Returns the path as a String.
fn make_repo(dir: &PathBuf, branch: &str) -> String {
    let repo = git2::Repository::init(dir).expect("init repo");

    // Configure identity so commits work without global git config
    let mut cfg = repo.config().unwrap();
    cfg.set_str("user.name", "Test User").unwrap();
    cfg.set_str("user.email", "test@example.com").unwrap();
    drop(cfg);

    // Create a file and commit it
    let file = dir.join("README.md");
    fs::write(&file, "hello").unwrap();

    let mut index = repo.index().unwrap();
    index.add_path(std::path::Path::new("README.md")).unwrap();
    index.write().unwrap();

    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let sig = repo.signature().unwrap();
    repo.commit(
        Some(&format!("refs/heads/{branch}")),
        &sig,
        &sig,
        "initial commit",
        &tree,
        &[],
    )
    .unwrap();

    // Point HEAD at the branch
    repo.set_head(&format!("refs/heads/{branch}")).unwrap();

    dir.to_string_lossy().to_string()
}

#[test]
fn integration_clean_repo() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let path = make_repo(&tmp.path().to_path_buf(), "main");

    let status = get_repo_status(&path, false).expect("get_repo_status");

    assert_eq!(status.branch, "main");
    assert!(status.is_clean(), "fresh repo should be clean");
    assert!(status.error.is_none());
    assert_eq!(status.added, 0);
    assert_eq!(status.modified, 0);
    assert_eq!(status.staged, 0);
    assert_eq!(status.deleted, 0);
    assert_eq!(status.conflict, 0);
    assert!(status.files.is_empty());
}

#[test]
fn integration_untracked_file() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let path = make_repo(&tmp.path().to_path_buf(), "main");

    // Add an untracked file (not staged)
    fs::write(tmp.path().join("new.txt"), "untracked").unwrap();

    let status = get_repo_status(&path, false).expect("get_repo_status");

    assert!(!status.is_clean());
    assert_eq!(status.added, 1, "should have 1 untracked file");
    assert_eq!(status.files.len(), 1);
    assert_eq!(status.files[0].status, FileStatusKind::Untracked);
    assert_eq!(status.files[0].path, "new.txt");
}

#[test]
fn integration_staged_file() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let path = make_repo(&tmp.path().to_path_buf(), "main");

    // Stage a new file
    fs::write(tmp.path().join("staged.txt"), "staged content").unwrap();
    let repo = git2::Repository::open(&path).unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(std::path::Path::new("staged.txt")).unwrap();
    index.write().unwrap();

    let status = get_repo_status(&path, false).expect("get_repo_status");

    assert!(!status.is_clean());
    assert_eq!(status.staged, 1);
    assert!(status
        .files
        .iter()
        .any(|f| f.status == FileStatusKind::Staged));
}

#[test]
fn integration_modified_file() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let path = make_repo(&tmp.path().to_path_buf(), "main");

    // Modify the committed file (don't stage)
    fs::write(tmp.path().join("README.md"), "modified content").unwrap();

    let status = get_repo_status(&path, false).expect("get_repo_status");

    assert!(!status.is_clean());
    assert_eq!(status.modified, 1);
    assert!(status
        .files
        .iter()
        .any(|f| f.status == FileStatusKind::Modified));
}

#[test]
fn integration_unborn_branch() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let dir = tmp.path().to_path_buf();

    // Init a repo but make no commits — HEAD points to unborn branch
    git2::Repository::init(&dir).expect("init repo");

    let path = dir.to_string_lossy().to_string();
    let status = get_repo_status(&path, false).expect("get_repo_status on unborn branch");

    // Should show the branch name (e.g. "master" or "main"), not "detached"
    assert!(!status.branch.is_empty());
    assert_ne!(status.branch, "detached");
    assert!(status.error.is_none());
}

#[test]
fn integration_invalid_path_returns_error_entry() {
    let path = "/nonexistent/path/to/repo";
    // get_repo_status should return Err for invalid path
    assert!(get_repo_status(path, false).is_err());

    // error_entry helper should produce a displayable placeholder
    let entry = RepoStatus::error_entry(path, "test error");
    assert_eq!(entry.path, path);
    assert!(entry.error.is_some());
    assert!(!entry.is_clean());
}

#[test]
fn integration_local_branches_listed() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let path = make_repo(&tmp.path().to_path_buf(), "main");

    // Create a second branch
    let repo = git2::Repository::open(&path).unwrap();
    let head = repo.head().unwrap();
    let commit = repo.find_commit(head.target().unwrap()).unwrap();
    repo.branch("feature-x", &commit, false).unwrap();

    let status = get_repo_status(&path, false).expect("get_repo_status");

    assert!(
        status.local_branches.contains(&"main".to_string()),
        "should list main branch"
    );
    assert!(
        status.local_branches.contains(&"feature-x".to_string()),
        "should list feature-x branch"
    );
    // Local branches should be sorted
    let sorted = {
        let mut v = status.local_branches.clone();
        v.sort();
        v
    };
    assert_eq!(status.local_branches, sorted);
}

// ── Commit history integration tests ─────────────────────────────────────────

fn make_repo_with_commits(dir: &PathBuf) -> String {
    let repo = git2::Repository::init(dir).expect("git init");
    let mut cfg = repo.config().unwrap();
    cfg.set_str("user.name", "Test User").unwrap();
    cfg.set_str("user.email", "test@example.com").unwrap();
    drop(cfg);

    // First commit: add README
    let readme = dir.join("README.md");
    fs::write(&readme, "hello").unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(std::path::Path::new("README.md")).unwrap();
    index.write().unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let sig = repo.signature().unwrap();
    let first_oid = repo
        .commit(
            Some("refs/heads/main"),
            &sig,
            &sig,
            "initial commit",
            &tree,
            &[],
        )
        .unwrap();

    // Second commit: add another file
    let second = dir.join("second.txt");
    fs::write(&second, "content").unwrap();
    index.add_path(std::path::Path::new("second.txt")).unwrap();
    index.write().unwrap();
    let tree_id2 = index.write_tree().unwrap();
    let tree2 = repo.find_tree(tree_id2).unwrap();
    let parent = repo.find_commit(first_oid).unwrap();
    repo.commit(
        Some("refs/heads/main"),
        &sig,
        &sig,
        "add second file",
        &tree2,
        &[&parent],
    )
    .unwrap();

    repo.set_head("refs/heads/main").unwrap();
    dir.to_string_lossy().to_string()
}

#[test]
fn commit_history_full_filter_returns_all_commits() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let path = make_repo_with_commits(&tmp.path().to_path_buf());

    let commits =
        get_commit_history(&path, &HistoryFilter::Full, 100, false).expect("get_commit_history");

    assert_eq!(commits.len(), 2, "should return both commits");
    // Most recent first (TIME sort)
    assert_eq!(commits[0].summary, "add second file");
    assert_eq!(commits[1].summary, "initial commit");
    assert_eq!(commits[0].files.len(), 1, "second commit added one file");
}

#[test]
fn commit_history_full_respects_limit() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let path = make_repo_with_commits(&tmp.path().to_path_buf());

    let commits =
        get_commit_history(&path, &HistoryFilter::Full, 1, false).expect("get_commit_history");

    assert_eq!(
        commits.len(),
        1,
        "limit=1 should return only the newest commit"
    );
    assert_eq!(commits[0].summary, "add second file");
}

#[test]
fn commit_history_invalid_path_returns_error() {
    let result = get_commit_history("/nonexistent/path", &HistoryFilter::Full, 100, false);
    assert!(result.is_err());
}

#[test]
fn get_head_commit_message_returns_latest_commit_summary() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let path = make_repo_with_commits(&tmp.path().to_path_buf());

    let msg = get_head_commit_message(&path).expect("head commit message");
    assert!(
        msg.starts_with("add second file"),
        "should return the HEAD (most recent) commit message, got: {msg}"
    );
}

#[test]
fn get_head_commit_message_returns_none_for_invalid_path() {
    let msg = get_head_commit_message("/nonexistent/path");
    assert!(msg.is_none());
}

#[test]
fn get_head_commit_file_count_returns_correct_count() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let path = make_repo_with_commits(&tmp.path().to_path_buf());

    let count = get_head_commit_file_count(&path);
    assert_eq!(count, 1, "HEAD commit (add second file) changed one file");
}

#[test]
fn get_head_commit_file_count_returns_zero_for_invalid_path() {
    let count = get_head_commit_file_count("/nonexistent/path");
    assert_eq!(count, 0);
}

#[test]
fn get_untracked_file_content_reads_text_file() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let path = make_repo(&tmp.path().to_path_buf(), "main");
    let file_path = tmp.path().join("notes.txt");
    fs::write(&file_path, "hello world\n").unwrap();

    let content = get_untracked_file_content(&path, "notes.txt").expect("read untracked file");
    assert_eq!(content, "hello world\n");
}

#[test]
fn get_untracked_file_content_returns_binary_marker_for_binary() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let path = make_repo(&tmp.path().to_path_buf(), "main");
    let file_path = tmp.path().join("data.bin");
    fs::write(&file_path, b"data\x00binary\x00here").unwrap();

    let content = get_untracked_file_content(&path, "data.bin").expect("read binary file");
    assert_eq!(content, "<binary file>");
}

#[test]
fn get_untracked_file_content_errors_for_missing_file() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let path = make_repo(&tmp.path().to_path_buf(), "main");
    let result = get_untracked_file_content(&path, "does_not_exist.txt");
    assert!(result.is_err());
}

// ── get_commit_history additional filter tests ────────────────────────────────

#[test]
fn commit_history_branch_full_filter_returns_commits() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let path = make_repo_with_commits(&tmp.path().to_path_buf());

    let commits = get_commit_history(
        &path,
        &HistoryFilter::BranchFull("main".to_string()),
        100,
        false,
    )
    .expect("get_commit_history with BranchFull");

    assert!(
        !commits.is_empty(),
        "BranchFull should return commits for 'main'"
    );
}

// ── get_file_diff integration ─────────────────────────────────────────────────

#[test]
fn get_file_diff_returns_diff_for_modified_file() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let path = make_repo(&tmp.path().to_path_buf(), "main");
    fs::write(tmp.path().join("README.md"), "modified content\n").unwrap();

    let diff = get_file_diff(&path, "README.md", "git").expect("get_file_diff");
    assert!(
        diff.contains("modified content") || diff.contains("-hello"),
        "diff must reference the changed content, got: {diff}"
    );
}

#[test]
fn get_file_diff_returns_empty_for_clean_file() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let path = make_repo(&tmp.path().to_path_buf(), "main");
    // File is unchanged — diff should be empty
    let diff = get_file_diff(&path, "README.md", "git").expect("get_file_diff");
    assert!(
        diff.is_empty(),
        "diff of clean file should be empty, got: {diff}"
    );
}

// ── get_commit_file_diff integration ─────────────────────────────────────────

#[test]
fn get_commit_file_diff_shows_initial_file_addition() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let path = make_repo_with_commits(&tmp.path().to_path_buf());

    let commits = get_commit_history(&path, &HistoryFilter::Full, 100, false).unwrap();
    let initial = &commits[commits.len() - 1]; // oldest commit

    let diff = get_commit_file_diff(&path, &initial.short_hash, "README.md", "git")
        .expect("get_commit_file_diff");
    assert!(
        !diff.is_empty(),
        "diff of initial commit for README.md should not be empty"
    );
}

// ── Deleted file in working tree ─────────────────────────────────────────────

#[test]
fn integration_deleted_file_shows_in_status() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let path = make_repo(&tmp.path().to_path_buf(), "main");

    fs::remove_file(tmp.path().join("README.md")).unwrap();

    let status = get_repo_status(&path, false).expect("get_repo_status");
    assert_eq!(status.deleted, 1, "should report 1 deleted file");
    assert!(
        status
            .files
            .iter()
            .any(|f| f.status == FileStatusKind::Deleted),
        "should have a Deleted entry"
    );
}

// ── Modified/Deleted deltas and case-sensitive sort ───────────────────────────

fn make_repo_with_file_modification(dir: &PathBuf) -> String {
    let repo = git2::Repository::init(dir).expect("git init");
    let mut cfg = repo.config().unwrap();
    cfg.set_str("user.name", "Test User").unwrap();
    cfg.set_str("user.email", "test@example.com").unwrap();
    drop(cfg);

    let readme = dir.join("README.md");
    fs::write(&readme, "original").unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(std::path::Path::new("README.md")).unwrap();
    index.write().unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let sig = repo.signature().unwrap();
    let first_oid = repo
        .commit(
            Some("refs/heads/main"),
            &sig,
            &sig,
            "initial commit",
            &tree,
            &[],
        )
        .unwrap();

    fs::write(&readme, "modified").unwrap();
    index.add_path(std::path::Path::new("README.md")).unwrap();
    index.write().unwrap();
    let tree_id2 = index.write_tree().unwrap();
    let tree2 = repo.find_tree(tree_id2).unwrap();
    let parent = repo.find_commit(first_oid).unwrap();
    repo.commit(
        Some("refs/heads/main"),
        &sig,
        &sig,
        "modify README",
        &tree2,
        &[&parent],
    )
    .unwrap();

    repo.set_head("refs/heads/main").unwrap();
    dir.to_string_lossy().to_string()
}

#[test]
fn commit_history_includes_modified_delta() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let path = make_repo_with_file_modification(&tmp.path().to_path_buf());

    let history =
        get_commit_history(&path, &HistoryFilter::Full, 100, false).expect("get_commit_history");
    assert_eq!(history.len(), 2);

    let modify_commit = &history[0];
    assert!(
        modify_commit
            .files
            .iter()
            .any(|d| d.kind == DeltaKind::Modified),
        "modify commit should have a Modified delta"
    );
}

#[test]
fn commit_history_case_sensitive_sort_exercises_sort_closure() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let path = make_repo_with_file_modification(&tmp.path().to_path_buf());

    let history = get_commit_history(&path, &HistoryFilter::Full, 100, true)
        .expect("get_commit_history with case_sensitive_sort=true");
    assert!(!history.is_empty());
}

// ── get_branches_with_ahead_behind ────────────────────────────────────────────

#[test]
fn get_branches_with_ahead_behind_returns_current_branch() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let path = make_repo(&tmp.path().to_path_buf(), "main");

    let branches = get_branches_with_ahead_behind(&path).expect("get_branches_with_ahead_behind");

    assert!(!branches.is_empty(), "should have at least one branch");
    let current = branches.iter().find(|b| b.is_current);
    assert!(current.is_some(), "should have a current branch");
    assert_eq!(current.unwrap().name, "main");
}

#[test]
fn get_branches_with_ahead_behind_lists_all_local_branches() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let path = make_repo(&tmp.path().to_path_buf(), "main");

    {
        let repo = git2::Repository::open(&path).unwrap();
        let head = repo.head().unwrap();
        let commit = repo.find_commit(head.target().unwrap()).unwrap();
        repo.branch("feature-y", &commit, false).unwrap();
    } // repo dropped here so the path is no longer locked

    let branches = get_branches_with_ahead_behind(&path).expect("get_branches_with_ahead_behind");
    let names: Vec<_> = branches.iter().map(|b| b.name.as_str()).collect();
    assert!(names.contains(&"main"), "should list main");
    assert!(names.contains(&"feature-y"), "should list feature-y");
}
