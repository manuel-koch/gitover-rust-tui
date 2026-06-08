/// Unit and integration tests for git.rs.
///
/// The unit tests cover pure logic (FileStatusKind methods, RepoStatus helpers,
/// file sort ordering) that require no filesystem access.
///
/// The integration tests create a real temporary git repository via git2 and
/// verify that `get_repo_status` returns sensible results for several scenarios.
use gitover::git::{get_repo_status, FileEntry, FileStatusKind, RepoStatus};
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
