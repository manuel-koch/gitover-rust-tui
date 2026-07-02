// Copyright © 2026 Manuel Koch
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::{
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver, Sender},
    thread,
    time::{Duration, Instant, SystemTime},
};

use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

/// Returned to the caller: which repo root became dirty.
pub type DirtyRx = Receiver<String>;

/// Debounce window — we wait this long after the last event before reporting dirty.
const DEBOUNCE: Duration = Duration::from_millis(500);

/// How often to stat git state files (`HEAD`, `index`, etc.) as a fallback
/// when the filesystem watcher produces no events (e.g. macOS FSEvents
/// coalescing or missed events from external tools).
const FALLBACK_POLL: Duration = Duration::from_secs(2);

/// Start one background thread per repo path.
/// Returns a channel receiver that yields repo root paths when they need refreshing.
pub fn start(repo_paths: Vec<String>) -> DirtyRx {
    let (dirty_tx, dirty_rx) = mpsc::channel::<String>();

    for path in repo_paths {
        let tx = dirty_tx.clone();
        thread::spawn(move || watch_repo(path, tx));
    }

    dirty_rx
}

fn watch_repo(root: String, tx: Sender<String>) {
    let (ev_tx, ev_rx) = mpsc::channel();

    let mut watcher = match RecommendedWatcher::new(ev_tx, Config::default()) {
        Ok(w) => w,
        Err(e) => {
            eprintln!("watcher: failed to create watcher for {root}: {e}");
            return;
        }
    };

    let root_path = PathBuf::from(&root);

    let repo = git2::Repository::open(&root_path).ok();

    // Resolve the actual git directory via git2 so that worktrees (where .git
    // is a file pointing elsewhere) are handled correctly.  Fall back to the
    // naive join if git2 can't open the repo.
    let git_dir: PathBuf = repo
        .as_ref()
        .map(|r| r.path().to_path_buf())
        .unwrap_or_else(|| root_path.join(".git"));

    // Always watch the root (worktree). For normal repos the git_dir is inside
    // root_path so this covers everything. For worktrees and submodules the
    // git_dir is OUTSIDE root_path, so we must watch it separately.
    if let Err(e) = watcher.watch(&root_path, RecursiveMode::Recursive) {
        eprintln!("watcher: failed to watch {root}: {e}");
        return;
    }

    // For worktrees and submodules, also watch the git_dir directly since it
    // lives outside the worktree (the .git is a file pointing there).
    // Skip this if git_dir is inside root_path (normal repo) or if git_dir
    // equals root_path/.git (meaning we already cover it via root_path watch).
    let git_dir_outside_worktree = !git_dir.starts_with(&root_path);
    let git_dir_is_file = root_path.join(".git") != git_dir;
    if git_dir_outside_worktree && git_dir_is_file {
        // git_dir is outside root_path (worktree/submodule case) — watch it too
        if let Err(e) = watcher.watch(&git_dir, RecursiveMode::Recursive) {
            eprintln!("watcher: failed to watch git_dir {git_dir:?}: {e}");
            // Continue anyway — we still have root_path watched
        }
    }

    let mut last_relevant: Option<Instant> = None;
    let mut head_mtime: Option<SystemTime> = None;
    let mut index_mtime: Option<SystemTime> = None;

    loop {
        let timeout = last_relevant
            .map(|t| DEBOUNCE.checked_sub(t.elapsed()).unwrap_or(Duration::ZERO))
            .unwrap_or(FALLBACK_POLL);

        match ev_rx.recv_timeout(timeout) {
            Ok(Ok(event)) => {
                if is_relevant(&event, &root_path, &git_dir, &repo) {
                    last_relevant = Some(Instant::now());
                }
            }
            Ok(Err(e)) => {
                eprintln!("watcher: notify error in {root}: {e}");
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                if let Some(t) = last_relevant {
                    if t.elapsed() >= DEBOUNCE {
                        // If the receiver is gone (app replaced dirty_rx) exit the thread.
                        if tx.send(root.clone()).is_err() {
                            return;
                        }
                        last_relevant = None;
                    }
                } else {
                    // No filesystem events pending — poll HEAD and index as a
                    // fallback in case the fs watcher missed the change (macOS
                    // FSEvents coalescing, etc.).  Uses a simple mtime check:
                    // stat is orders of magnitude cheaper than a full git status.
                    match poll_mtime(&git_dir.join("HEAD"), &mut head_mtime, &tx, &root) {
                        MtimeResult::DeadChannel => return,
                        MtimeResult::Changed => continue,
                        MtimeResult::Unchanged => {}
                    }
                    match poll_mtime(&git_dir.join("index"), &mut index_mtime, &tx, &root) {
                        MtimeResult::DeadChannel => return,
                        MtimeResult::Changed => {}
                        MtimeResult::Unchanged => {}
                    }
                }
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => return,
        }
    }
}

/// Result of polling a git state file for modification-time changes.
enum MtimeResult {
    /// File has not changed since the last poll (or stat/modified failed).
    Unchanged,
    /// File changed; a refresh signal was sent to the dirty channel.
    Changed,
    /// The receiver end of the dirty channel was dropped — the polling
    /// thread should exit.
    DeadChannel,
}

/// Stat a git state file and compare its mtime to the last-known value.
/// On first call the mtime is recorded without triggering a refresh (baseline).
/// On subsequent calls, if the mtime differs, a dirty signal is sent via `tx`.
///
/// Returns `MtimeResult::Changed` when a refresh was signalled,
/// `MtimeResult::DeadChannel` when the receiver is gone (caller should exit),
/// or `MtimeResult::Unchanged` otherwise.
fn poll_mtime(
    path: &Path,
    mtime: &mut Option<SystemTime>,
    tx: &Sender<String>,
    root: &str,
) -> MtimeResult {
    let Ok(meta) = std::fs::metadata(path) else {
        return MtimeResult::Unchanged;
    };
    let Ok(new_mtime) = meta.modified() else {
        return MtimeResult::Unchanged;
    };

    let changed = match *mtime {
        Some(prev) => prev != new_mtime,
        None => {
            // First poll — record baseline without triggering a refresh.
            *mtime = Some(new_mtime);
            return MtimeResult::Unchanged;
        }
    };

    if changed {
        *mtime = Some(new_mtime);
        if tx.send(root.to_string()).is_err() {
            return MtimeResult::DeadChannel;
        }
        MtimeResult::Changed
    } else {
        MtimeResult::Unchanged
    }
}

/// Returns true if this filesystem event should trigger a git status refresh.
//////
/// For paths OUTSIDE .git:
///   - Skip noisy filenames: .DS_Store, __pycache__
///   - Skip directory-level events (only file changes matter)
///   - Skip git-ignored files
///
/// For paths INSIDE .git:
///   - Skip objects/ subtree (git object database, always noisy)
///   - Skip *.lock and *.cache files (git internal locks)
///   - Skip hooks/ subtree
///   - Skip modules/ subtree
///   - Skip packed-refs (remote ref packing)
///   - Skip sourcetreeconfig (third-party tool)
///   - Skip commit-message drafts: COMMIT_EDITMSG, PREPARE_COMMIT_MSG, GIT_COLA_MSG
///   - Skip LFS tmp paths (lfs/.../tmp/...)
///   - Everything else IS relevant: HEAD, index, refs/, rebase-merge/, ORIG_HEAD, etc.
fn is_relevant(
    event: &notify::Event,
    repo_root: &Path,
    git_dir: &Path,
    repo: &Option<git2::Repository>,
) -> bool {
    match &event.kind {
        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) => {}
        _ => return false,
    }

    for path in &event.paths {
        if is_relevant_path(path, repo_root, git_dir, repo) {
            return true;
        }
    }
    false
}

fn is_relevant_path(
    path: &Path,
    repo_root: &Path,
    git_dir: &Path,
    repo: &Option<git2::Repository>,
) -> bool {
    let inside_git = path.starts_with(git_dir);

    if inside_git {
        is_relevant_git_path(path, git_dir)
    } else {
        is_relevant_worktree_path(path, repo_root, repo)
    }
}

/// Decide whether a path inside `.git/` is relevant.
fn is_relevant_git_path(path: &Path, git_dir: &Path) -> bool {
    // A directory-level event on git_dir itself (common on macOS FSEvents when
    // git rewrites HEAD or updates refs) should trigger a refresh — it means
    // something changed inside the git dir even if the exact path was coalesced.
    if path == git_dir {
        return true;
    }

    let rel = match path.strip_prefix(git_dir) {
        Ok(r) => r,
        Err(_) => return false,
    };

    let parts: Vec<&str> = rel
        .components()
        .filter_map(|c| c.as_os_str().to_str())
        .collect();

    if parts.is_empty() {
        return false;
    }

    let top = parts[0];
    let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

    // Skip git object database
    if top == "objects" {
        return false;
    }

    // Skip lock/cache files (index.lock, config.lock, etc.)
    if ext == "lock" || ext == "cache" {
        return false;
    }

    // Skip hooks directory
    if top == "hooks" {
        return false;
    }

    // Skip git submodule metadata
    if top == "modules" {
        return false;
    }

    // Skip remote ref packing
    if rel == Path::new("packed-refs") {
        return false;
    }

    // Skip SourceTree config
    if filename == "sourcetreeconfig" {
        return false;
    }

    // Skip commit-message drafts (noisy during editor sessions)
    if matches!(
        filename,
        "COMMIT_EDITMSG" | "PREPARE_COMMIT_MSG" | "GIT_COLA_MSG"
    ) {
        return false;
    }

    // Skip git-lfs tmp paths: lfs/<something>/tmp/...
    if parts.contains(&"lfs") && parts.contains(&"tmp") {
        return false;
    }

    // The index file is the primary signal for stage/unstage operations.
    // git writes a new index via rename(index.lock → index), so we must
    // treat it as always relevant to ensure the debounce fires.
    let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    if filename == "index" {
        return true;
    }

    // Everything else inside .git IS relevant:
    // HEAD, ORIG_HEAD, MERGE_HEAD, CHERRY_PICK_HEAD, index,
    // refs/, rebase-merge/, rebase-apply/, config, etc.
    true
}

/// Decide whether a path in the working tree is relevant.
fn is_relevant_worktree_path(
    path: &Path,
    repo_root: &Path,
    repo: &Option<git2::Repository>,
) -> bool {
    // Ignore the repo root directory event itself
    if path == repo_root {
        return false;
    }

    // Skip noisy OS/tool artifacts
    let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    if matches!(filename, ".DS_Store" | "__pycache__") {
        return false;
    }

    // Skip git-ignored files
    if let Some(r) = repo {
        if r.is_path_ignored(path).unwrap_or(false) {
            return false;
        }
    }

    true
}

/// Helper: rebuild the watcher channel for a new set of repo paths.
#[allow(dead_code)]
pub fn restart(repo_paths: Vec<String>) -> DirtyRx {
    start(repo_paths)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    // ── is_relevant_git_path ──────────────────────────────────────────────────

    #[test]
    fn git_path_git_dir_itself_is_relevant() {
        let git_dir = Path::new("/repo/.git");
        assert!(is_relevant_git_path(git_dir, git_dir));
    }

    #[test]
    fn git_path_objects_dir_is_skipped() {
        let git_dir = Path::new("/repo/.git");
        assert!(!is_relevant_git_path(
            &git_dir.join("objects/ab/cd1234"),
            git_dir
        ));
    }

    #[test]
    fn git_path_lock_file_is_skipped() {
        let git_dir = Path::new("/repo/.git");
        assert!(!is_relevant_git_path(&git_dir.join("index.lock"), git_dir));
    }

    #[test]
    fn git_path_cache_file_is_skipped() {
        let git_dir = Path::new("/repo/.git");
        assert!(!is_relevant_git_path(
            &git_dir.join("something.cache"),
            git_dir
        ));
    }

    #[test]
    fn git_path_hooks_dir_is_skipped() {
        let git_dir = Path::new("/repo/.git");
        assert!(!is_relevant_git_path(
            &git_dir.join("hooks/pre-commit"),
            git_dir
        ));
    }

    #[test]
    fn git_path_modules_dir_is_skipped() {
        let git_dir = Path::new("/repo/.git");
        assert!(!is_relevant_git_path(
            &git_dir.join("modules/sub/HEAD"),
            git_dir
        ));
    }

    #[test]
    fn git_path_packed_refs_is_skipped() {
        let git_dir = Path::new("/repo/.git");
        assert!(!is_relevant_git_path(&git_dir.join("packed-refs"), git_dir));
    }

    #[test]
    fn git_path_commit_editmsg_is_skipped() {
        let git_dir = Path::new("/repo/.git");
        assert!(!is_relevant_git_path(
            &git_dir.join("COMMIT_EDITMSG"),
            git_dir
        ));
        assert!(!is_relevant_git_path(
            &git_dir.join("PREPARE_COMMIT_MSG"),
            git_dir
        ));
        assert!(!is_relevant_git_path(
            &git_dir.join("GIT_COLA_MSG"),
            git_dir
        ));
    }

    #[test]
    fn git_path_lfs_tmp_is_skipped() {
        let git_dir = Path::new("/repo/.git");
        assert!(!is_relevant_git_path(
            &git_dir.join("lfs/objects/tmp/abc"),
            git_dir
        ));
    }

    #[test]
    fn git_path_head_is_relevant() {
        let git_dir = Path::new("/repo/.git");
        assert!(is_relevant_git_path(&git_dir.join("HEAD"), git_dir));
    }

    #[test]
    fn git_path_index_is_relevant() {
        let git_dir = Path::new("/repo/.git");
        assert!(is_relevant_git_path(&git_dir.join("index"), git_dir));
    }

    #[test]
    fn git_path_refs_is_relevant() {
        let git_dir = Path::new("/repo/.git");
        assert!(is_relevant_git_path(
            &git_dir.join("refs/heads/main"),
            git_dir
        ));
    }

    #[test]
    fn git_path_config_is_relevant() {
        let git_dir = Path::new("/repo/.git");
        assert!(is_relevant_git_path(&git_dir.join("config"), git_dir));
    }

    #[test]
    fn git_path_rebase_merge_is_relevant() {
        let git_dir = Path::new("/repo/.git");
        assert!(is_relevant_git_path(
            &git_dir.join("rebase-merge/head-name"),
            git_dir
        ));
    }

    // ── is_relevant_worktree_path ─────────────────────────────────────────────

    #[test]
    fn worktree_repo_root_itself_is_not_relevant() {
        let root = Path::new("/repo");
        assert!(!is_relevant_worktree_path(root, root, &None));
    }

    #[test]
    fn worktree_ds_store_is_skipped() {
        let root = Path::new("/repo");
        assert!(!is_relevant_worktree_path(
            &root.join(".DS_Store"),
            root,
            &None
        ));
    }

    #[test]
    fn worktree_pycache_is_skipped() {
        let root = Path::new("/repo");
        assert!(!is_relevant_worktree_path(
            &root.join("__pycache__"),
            root,
            &None
        ));
    }

    #[test]
    fn worktree_regular_source_file_is_relevant() {
        let root = Path::new("/repo");
        assert!(is_relevant_worktree_path(
            &root.join("src/main.rs"),
            root,
            &None
        ));
    }

    #[test]
    fn worktree_nested_file_is_relevant() {
        let root = Path::new("/repo");
        assert!(is_relevant_worktree_path(
            &root.join("a/b/c/file.txt"),
            root,
            &None
        ));
    }
}
