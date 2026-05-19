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
    time::{Duration, Instant},
};

use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

/// Returned to the caller: which repo root became dirty.
pub type DirtyRx = Receiver<String>;

/// Debounce window — we wait this long after the last event before reporting dirty.
const DEBOUNCE: Duration = Duration::from_millis(500);

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

    loop {
        let timeout = last_relevant
            .map(|t| DEBOUNCE.checked_sub(t.elapsed()).unwrap_or(Duration::ZERO))
            .unwrap_or(Duration::from_secs(60));

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
                }
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => return,
        }
    }
}

/// Returns true if this filesystem event should trigger a git status refresh.
///
/// Logic mirrors the Python fswatcher.py RepoTracker.ignored() / discarded():
///
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

    // Skip directory-level events: only file content changes matter.
    // (notify may emit events for the parent dir when a file inside changes.)
    if path.is_dir() {
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
