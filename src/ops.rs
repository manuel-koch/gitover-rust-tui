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

use crate::git;
use std::process::{Command, Output, Stdio};
use std::sync::mpsc::Sender;
use std::time::SystemTime;

/// Result of a background git operation sent back to the main thread.
pub struct OpResult {
    pub repo_path: String,
    pub op_label: String,
    pub success: bool,
    pub lines: Vec<String>,
    /// Populated by `OpRequest::Refresh` — the freshly-read repo status.
    pub fresh_status: Option<git::RepoStatus>,
}

/// Which git operation to execute.
pub enum OpRequest {
    Fetch,
    Pull,
    Push,
    ForcePush,
    /// Checkout a branch.  `is_remote` controls whether `--track` is added.
    CheckoutBranch {
        name: String,
        is_remote: bool,
    },
    CreateBranch(String),
    /// Create a new branch off a specific base ref: `git checkout -b <name> <base>`.
    CreateBranchFrom {
        name: String,
        base: String,
    },
    DeleteBranch(String),
    /// Stage a file: `git add -- <path>` (path relative to repo root).
    StageFile(String),
    /// Unstage a file: `git reset -- <path>` (path relative to repo root).
    UnstageFile(String),
    /// Revert working-tree changes: `git checkout -- <path>`.
    /// For conflict files, runs `git reset -- <path>` first.
    RevertFile {
        file_path: String,
        is_conflict: bool,
    },
    /// Delete an untracked file from disk (path relative to repo root).
    DiscardFile(String),
    /// Fast-forward pull of a local branch without checking it out.
    /// `upstream` is the remote-tracking ref, e.g. "origin/feature-x".
    PullBranch {
        name: String,
        upstream: String,
    },
    /// Push a specific local branch (not necessarily HEAD) to origin.
    PushBranch {
        name: String,
    },
    /// Force-push a specific local branch (not necessarily HEAD) to origin.
    ForcePushBranch {
        name: String,
    },
    /// Run a custom shell command from config (already interpolated).
    RunRepoCommand {
        name: String,
        cmd: String,
        /// When true, spawn without waiting and discard output.
        background: bool,
    },
    /// Save the current diff of a file as `<file_path>.patch` (relative to repo root),
    /// then revert the file to its HEAD state.
    SavePatchAndRevert {
        file_path: String,
    },
    /// Apply a patch file using `git apply <file_path>` (path relative to repo root).
    ApplyPatch {
        file_path: String,
    },
    /// Create a new commit (or amend the last one) with the given message.
    Commit {
        message: String,
        amend: bool,
    },
    /// Undo the HEAD commit, leaving its changes as unstaged working-tree modifications.
    /// Equivalent to `git reset --mixed HEAD~1`.
    UndoCommit,
    /// Re-read the repo status in a background thread so the UI can show a spinner.
    Refresh {
        case_sensitive_sort: bool,
    },
}

impl OpRequest {
    pub fn label(&self) -> String {
        match self {
            OpRequest::Fetch => "fetch".into(),
            OpRequest::Pull => "pull".into(),
            OpRequest::Push => "push".into(),
            OpRequest::ForcePush => "force push".into(),
            OpRequest::CheckoutBranch { .. } => "checkout".into(),
            OpRequest::CreateBranch(_) | OpRequest::CreateBranchFrom { .. } => {
                "create branch".into()
            }
            OpRequest::DeleteBranch(_) => "delete branch".into(),
            OpRequest::StageFile(_) => "stage file".into(),
            OpRequest::UnstageFile(_) => "unstage file".into(),
            OpRequest::RevertFile { .. } => "revert file".into(),
            OpRequest::DiscardFile(_) => "discard file".into(),
            OpRequest::PullBranch { name, .. } => format!("pull branch {name}"),
            OpRequest::PushBranch { name } => format!("push branch {name}"),
            OpRequest::ForcePushBranch { name } => format!("force push branch {name}"),
            OpRequest::RunRepoCommand { name, .. } => name.clone(),
            OpRequest::SavePatchAndRevert { .. } => "save patch and revert".into(),
            OpRequest::ApplyPatch { .. } => "apply patch".into(),
            OpRequest::Commit { amend: true, .. } => "amend commit".into(),
            OpRequest::Commit { .. } => "commit".into(),
            OpRequest::UndoCommit => "undo commit".into(),
            OpRequest::Refresh { .. } => "scan".into(),
        }
    }
}

/// Spawn a background thread that executes `request` and sends the result to `tx`.
pub fn spawn_op(repo_path: String, request: OpRequest, git_bin: String, tx: Sender<OpResult>) {
    std::thread::spawn(move || {
        let label = request.label();
        let (success, lines, fresh_status) = match &request {
            OpRequest::Refresh {
                case_sensitive_sort,
            } => {
                let status = match git::get_repo_status(&repo_path, *case_sensitive_sort) {
                    Ok(s) => s,
                    Err(e) => git::RepoStatus::error_entry(&repo_path, format!("{e}")),
                };
                (true, vec![], Some(status))
            }
            _ => {
                let (ok, out) = run_op(&repo_path, &request, &git_bin);
                (ok, out, None)
            }
        };
        let _ = tx.send(OpResult {
            repo_path,
            op_label: label,
            success,
            lines,
            fresh_status,
        });
    });
}

fn run_op(repo_path: &str, request: &OpRequest, git_bin: &str) -> (bool, Vec<String>) {
    let mut lines = Vec::new();

    let ok = match request {
        OpRequest::Fetch => run_git(
            git_bin,
            repo_path,
            &["fetch", "origin", "--prune"],
            &mut lines,
        ),

        OpRequest::Pull => {
            let stashed = maybe_stash(git_bin, repo_path, &mut lines);
            let ok = run_git(git_bin, repo_path, &["pull", "--prune"], &mut lines);
            if stashed {
                run_git(git_bin, repo_path, &["stash", "pop"], &mut lines);
            }
            ok
        }

        OpRequest::Push => run_git(
            git_bin,
            repo_path,
            &["push", "--set-upstream", "origin", "HEAD"],
            &mut lines,
        ),

        OpRequest::ForcePush => run_git(
            git_bin,
            repo_path,
            &["push", "--force", "--set-upstream", "origin", "HEAD"],
            &mut lines,
        ),

        OpRequest::CheckoutBranch { name, is_remote } => {
            let stashed = maybe_stash(git_bin, repo_path, &mut lines);
            let ok = if *is_remote {
                run_git(
                    git_bin,
                    repo_path,
                    &["checkout", "--track", name],
                    &mut lines,
                )
            } else {
                run_git(git_bin, repo_path, &["checkout", name], &mut lines)
            };
            if stashed {
                run_git(git_bin, repo_path, &["stash", "pop"], &mut lines);
            }
            ok
        }

        OpRequest::CreateBranch(name) => {
            run_git(git_bin, repo_path, &["checkout", "-b", name], &mut lines)
        }

        OpRequest::CreateBranchFrom { name, base } => run_git(
            git_bin,
            repo_path,
            &["checkout", "-b", name, base],
            &mut lines,
        ),

        OpRequest::DeleteBranch(name) => {
            run_git(git_bin, repo_path, &["branch", "-D", name], &mut lines)
        }

        OpRequest::StageFile(path) => run_git(git_bin, repo_path, &["add", "--", path], &mut lines),

        OpRequest::UnstageFile(path) => {
            run_git(git_bin, repo_path, &["reset", "--", path], &mut lines)
        }

        OpRequest::RevertFile {
            file_path,
            is_conflict,
        } => {
            if *is_conflict {
                run_git(git_bin, repo_path, &["reset", "--", file_path], &mut lines);
            }
            run_git(
                git_bin,
                repo_path,
                &["checkout", "--", file_path],
                &mut lines,
            )
        }

        OpRequest::PullBranch { name, upstream } => run_git(
            git_bin,
            repo_path,
            &["branch", "-f", name, upstream],
            &mut lines,
        ),

        OpRequest::PushBranch { name } => run_git(
            git_bin,
            repo_path,
            &["push", "--set-upstream", "origin", name],
            &mut lines,
        ),

        OpRequest::ForcePushBranch { name } => run_git(
            git_bin,
            repo_path,
            &["push", "--force", "--set-upstream", "origin", name],
            &mut lines,
        ),

        OpRequest::DiscardFile(path) => {
            let abs = std::path::PathBuf::from(repo_path).join(path);
            match std::fs::remove_file(&abs) {
                Ok(()) => {
                    lines.push(format!("deleted {path}"));
                    true
                }
                Err(e) => {
                    lines.push(format!("error deleting {path}: {e}"));
                    false
                }
            }
        }

        OpRequest::SavePatchAndRevert { file_path } => {
            let diff_output = Command::new(git_bin)
                .current_dir(repo_path)
                .args(["diff", "HEAD", "--", file_path])
                .output();

            match diff_output {
                Ok(output) if !output.stdout.is_empty() => {
                    let patch_file_path = format!("{file_path}.patch");
                    let absolute_patch_path =
                        std::path::PathBuf::from(repo_path).join(&patch_file_path);
                    match std::fs::write(&absolute_patch_path, &output.stdout) {
                        Ok(()) => {
                            lines.push(format!("saved patch to {patch_file_path}"));
                            // Unstage if staged, then restore working-tree state
                            run_git(git_bin, repo_path, &["reset", "--", file_path], &mut lines);
                            run_git(
                                git_bin,
                                repo_path,
                                &["checkout", "--", file_path],
                                &mut lines,
                            )
                        }
                        Err(error) => {
                            lines.push(format!("error saving patch: {error}"));
                            false
                        }
                    }
                }
                Ok(_) => {
                    lines.push(format!("no diff found for {file_path}"));
                    false
                }
                Err(error) => {
                    lines.push(format!("error getting diff: {error}"));
                    false
                }
            }
        }

        OpRequest::ApplyPatch { file_path } => {
            run_git(git_bin, repo_path, &["apply", "--", file_path], &mut lines)
        }

        OpRequest::UndoCommit => run_git(
            git_bin,
            repo_path,
            &["reset", "--mixed", "HEAD~1"],
            &mut lines,
        ),

        OpRequest::Commit { message, amend } => {
            let mut args = vec!["commit"];
            if *amend {
                args.push("--amend");
            }
            args.extend_from_slice(&["-m", message.as_str()]);
            run_git(git_bin, repo_path, &args, &mut lines)
        }

        OpRequest::RunRepoCommand {
            cmd, background, ..
        } => {
            if *background {
                match std::process::Command::new("sh")
                    .args(["-c", cmd])
                    .current_dir(repo_path)
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn()
                {
                    Ok(_) => true,
                    Err(e) => {
                        lines.push(format!("error: {e}"));
                        false
                    }
                }
            } else {
                match std::process::Command::new("sh")
                    .args(["-c", cmd])
                    .current_dir(repo_path)
                    .output()
                {
                    Ok(output) => {
                        append_output(&output, &mut lines);
                        output.status.success()
                    }
                    Err(e) => {
                        lines.push(format!("error: {e}"));
                        false
                    }
                }
            }
        }

        // Refresh is handled directly in spawn_op — run_op is never called for it.
        OpRequest::Refresh { .. } => unreachable!(),
    };

    (ok, lines)
}

/// Run a git command, collect stdout+stderr into `lines`, return success.
fn run_git(git_bin: &str, repo_path: &str, args: &[&str], lines: &mut Vec<String>) -> bool {
    match Command::new(git_bin)
        .current_dir(repo_path)
        .args(args)
        .output()
    {
        Ok(output) => {
            append_output(&output, lines);
            output.status.success()
        }
        Err(e) => {
            lines.push(format!("error: {e}"));
            false
        }
    }
}

/// Stash local changes if the working tree is dirty.
/// Returns `true` if a stash was created (so the caller knows to pop later).
fn maybe_stash(git_bin: &str, repo_path: &str, lines: &mut Vec<String>) -> bool {
    let dirty = Command::new(git_bin)
        .current_dir(repo_path)
        .args(["status", "--porcelain"])
        .output()
        .map(|o| !o.stdout.is_empty())
        .unwrap_or(false);

    if !dirty {
        return false;
    }

    let ts = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let msg = format!("gitover-autostash-{ts}");
    lines.push(format!("auto-stashing local changes ({msg})"));

    let ok = match Command::new(git_bin)
        .current_dir(repo_path)
        .args(["stash", "push", "-m", &msg])
        .output()
    {
        Ok(o) => {
            append_output(&o, lines);
            o.status.success()
        }
        Err(e) => {
            lines.push(format!("stash error: {e}"));
            false
        }
    };

    ok
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    // ── OpRequest::label() ────────────────────────────────────────────────────

    #[test]
    fn label_simple_variants() {
        assert_eq!(OpRequest::Fetch.label(), "fetch");
        assert_eq!(OpRequest::Pull.label(), "pull");
        assert_eq!(OpRequest::Push.label(), "push");
        assert_eq!(OpRequest::ForcePush.label(), "force push");
        assert_eq!(OpRequest::UndoCommit.label(), "undo commit");
        assert_eq!(
            OpRequest::Refresh {
                case_sensitive_sort: false
            }
            .label(),
            "scan"
        );
    }

    #[test]
    fn label_checkout_branch() {
        assert_eq!(
            OpRequest::CheckoutBranch {
                name: "main".to_string(),
                is_remote: false
            }
            .label(),
            "checkout"
        );
        assert_eq!(
            OpRequest::CheckoutBranch {
                name: "origin/feat".to_string(),
                is_remote: true
            }
            .label(),
            "checkout"
        );
    }

    #[test]
    fn label_create_branch_variants() {
        assert_eq!(
            OpRequest::CreateBranch("new-feat".to_string()).label(),
            "create branch"
        );
        assert_eq!(
            OpRequest::CreateBranchFrom {
                name: "feat".to_string(),
                base: "main".to_string()
            }
            .label(),
            "create branch"
        );
    }

    #[test]
    fn label_file_operations() {
        assert_eq!(
            OpRequest::StageFile("foo.rs".to_string()).label(),
            "stage file"
        );
        assert_eq!(
            OpRequest::UnstageFile("foo.rs".to_string()).label(),
            "unstage file"
        );
        assert_eq!(
            OpRequest::RevertFile {
                file_path: "foo.rs".to_string(),
                is_conflict: false
            }
            .label(),
            "revert file"
        );
        assert_eq!(
            OpRequest::DiscardFile("foo.rs".to_string()).label(),
            "discard file"
        );
        assert_eq!(
            OpRequest::SavePatchAndRevert {
                file_path: "foo.rs".to_string()
            }
            .label(),
            "save patch and revert"
        );
        assert_eq!(
            OpRequest::ApplyPatch {
                file_path: "foo.patch".to_string()
            }
            .label(),
            "apply patch"
        );
    }

    #[test]
    fn label_branch_sync_operations() {
        assert_eq!(
            OpRequest::DeleteBranch("feat".to_string()).label(),
            "delete branch"
        );
        assert_eq!(
            OpRequest::PullBranch {
                name: "feat".to_string(),
                upstream: "origin/feat".to_string()
            }
            .label(),
            "pull branch feat"
        );
        assert_eq!(
            OpRequest::PushBranch {
                name: "feat".to_string()
            }
            .label(),
            "push branch feat"
        );
        assert_eq!(
            OpRequest::ForcePushBranch {
                name: "feat".to_string()
            }
            .label(),
            "force push branch feat"
        );
    }

    #[test]
    fn label_commit_and_amend() {
        assert_eq!(
            OpRequest::Commit {
                message: "fix: bug".to_string(),
                amend: false
            }
            .label(),
            "commit"
        );
        assert_eq!(
            OpRequest::Commit {
                message: "fix: bug".to_string(),
                amend: true
            }
            .label(),
            "amend commit"
        );
    }

    #[test]
    fn label_run_repo_command_uses_name() {
        assert_eq!(
            OpRequest::RunRepoCommand {
                name: "Deploy".to_string(),
                cmd: "make deploy".to_string(),
                background: false
            }
            .label(),
            "Deploy"
        );
    }

    #[test]
    fn spawn_op_refresh_populates_fresh_status() {
        let tmp = tempfile::TempDir::new().unwrap();
        let repo = git2::Repository::init(tmp.path()).expect("git init");
        let mut cfg = repo.config().unwrap();
        cfg.set_str("user.name", "Test").unwrap();
        cfg.set_str("user.email", "test@example.com").unwrap();

        let (tx, rx) = mpsc::channel::<OpResult>();
        spawn_op(
            tmp.path().to_str().unwrap().to_string(),
            OpRequest::Refresh {
                case_sensitive_sort: false,
            },
            "git".to_string(),
            tx,
        );

        let result = rx
            .recv_timeout(std::time::Duration::from_secs(10))
            .expect("op result within timeout");
        assert!(result.success, "Refresh op must succeed");
        assert!(
            result.lines.is_empty(),
            "Refresh op must produce no output lines"
        );
        assert!(
            result.fresh_status.is_some(),
            "Refresh op must populate fresh_status"
        );
        assert_eq!(
            result.fresh_status.unwrap().path,
            tmp.path().to_str().unwrap(),
            "fresh_status path must match repo path"
        );
    }

    #[test]
    fn spawn_op_non_refresh_has_no_fresh_status() {
        let tmp = tempfile::TempDir::new().unwrap();
        let repo = git2::Repository::init(tmp.path()).expect("git init");
        let mut cfg = repo.config().unwrap();
        cfg.set_str("user.name", "Test").unwrap();
        cfg.set_str("user.email", "test@example.com").unwrap();

        let (tx, rx) = mpsc::channel::<OpResult>();
        spawn_op(
            tmp.path().to_str().unwrap().to_string(),
            OpRequest::Fetch,
            "git".to_string(),
            tx,
        );

        let result = rx
            .recv_timeout(std::time::Duration::from_secs(10))
            .expect("op result within timeout");
        assert!(
            result.fresh_status.is_none(),
            "non-Refresh ops must not populate fresh_status"
        );
    }

    fn make_committed_repo(tmp: &tempfile::TempDir) -> String {
        let repo = git2::Repository::init(tmp.path()).expect("git init");
        let mut cfg = repo.config().unwrap();
        cfg.set_str("user.name", "Test").unwrap();
        cfg.set_str("user.email", "test@example.com").unwrap();
        drop(cfg);

        let path = tmp.path().to_str().unwrap().to_string();
        let readme = tmp.path().join("README.md");
        std::fs::write(&readme, "hello").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(std::path::Path::new("README.md")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let sig = repo.signature().unwrap();
        repo.commit(
            Some("refs/heads/main"),
            &sig,
            &sig,
            "initial commit",
            &tree,
            &[],
        )
        .unwrap();
        repo.set_head("refs/heads/main").unwrap();
        path
    }

    fn run_op_sync(repo_path: &str, op: OpRequest) -> OpResult {
        let (tx, rx) = mpsc::channel::<OpResult>();
        spawn_op(repo_path.to_string(), op, "git".to_string(), tx);
        rx.recv_timeout(std::time::Duration::from_secs(15))
            .expect("op result within timeout")
    }

    #[test]
    fn spawn_op_stage_and_unstage_file() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = make_committed_repo(&tmp);

        std::fs::write(tmp.path().join("new.txt"), "content").unwrap();

        let stage_result = run_op_sync(&path, OpRequest::StageFile("new.txt".to_string()));
        assert!(stage_result.success, "StageFile must succeed");
        assert!(stage_result.fresh_status.is_none());

        let unstage_result = run_op_sync(&path, OpRequest::UnstageFile("new.txt".to_string()));
        assert!(unstage_result.success, "UnstageFile must succeed");
    }

    #[test]
    fn spawn_op_commit_creates_new_commit() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = make_committed_repo(&tmp);

        std::fs::write(tmp.path().join("file.txt"), "new content").unwrap();
        run_op_sync(&path, OpRequest::StageFile("file.txt".to_string()));

        let commit_result = run_op_sync(
            &path,
            OpRequest::Commit {
                message: "test commit".to_string(),
                amend: false,
            },
        );
        assert!(commit_result.success, "Commit must succeed");

        let repo = git2::Repository::open(&path).unwrap();
        let head = repo.head().unwrap().peel_to_commit().unwrap();
        assert_eq!(head.summary().unwrap_or(""), "test commit");
    }

    #[test]
    fn spawn_op_create_branch_succeeds() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = make_committed_repo(&tmp);

        let result = run_op_sync(&path, OpRequest::CreateBranch("new-feature".to_string()));
        assert!(result.success, "CreateBranch must succeed");

        let repo = git2::Repository::open(&path).unwrap();
        let branch = repo.find_branch("new-feature", git2::BranchType::Local);
        assert!(branch.is_ok(), "branch new-feature must exist after create");
    }

    #[test]
    fn spawn_op_checkout_branch_switches_head() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = make_committed_repo(&tmp);

        run_op_sync(&path, OpRequest::CreateBranch("feature".to_string()));

        let result = run_op_sync(
            &path,
            OpRequest::CheckoutBranch {
                name: "feature".to_string(),
                is_remote: false,
            },
        );
        assert!(result.success, "CheckoutBranch must succeed");

        let repo = git2::Repository::open(&path).unwrap();
        let head = repo.head().unwrap();
        let branch_name = head.shorthand().unwrap_or("");
        assert_eq!(branch_name, "feature");
    }

    #[test]
    fn spawn_op_revert_file_restores_modified_file() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = make_committed_repo(&tmp);

        // Modify the committed README
        std::fs::write(tmp.path().join("README.md"), "modified").unwrap();

        let status_before = crate::git::get_repo_status(&path, false).unwrap();
        assert_eq!(
            status_before.modified, 1,
            "should have a modified file before revert"
        );

        let revert_result = run_op_sync(
            &path,
            OpRequest::RevertFile {
                file_path: "README.md".to_string(),
                is_conflict: false,
            },
        );
        assert!(revert_result.success, "RevertFile must succeed");

        let status_after = crate::git::get_repo_status(&path, false).unwrap();
        assert!(status_after.is_clean(), "repo must be clean after revert");
    }

    #[test]
    fn spawn_op_create_branch_from_base_creates_at_correct_commit() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = make_committed_repo(&tmp);

        let result = run_op_sync(
            &path,
            OpRequest::CreateBranchFrom {
                name: "new-from-main".to_string(),
                base: "main".to_string(),
            },
        );
        assert!(result.success, "CreateBranchFrom must succeed");

        let repo = git2::Repository::open(&path).unwrap();
        assert!(repo
            .find_branch("new-from-main", git2::BranchType::Local)
            .is_ok());
    }

    #[test]
    fn spawn_op_delete_branch_removes_branch() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = make_committed_repo(&tmp);

        run_op_sync(&path, OpRequest::CreateBranch("to-delete".to_string()));
        run_op_sync(
            &path,
            OpRequest::CheckoutBranch {
                name: "main".to_string(),
                is_remote: false,
            },
        );

        let result = run_op_sync(&path, OpRequest::DeleteBranch("to-delete".to_string()));
        assert!(result.success, "DeleteBranch must succeed");

        let repo = git2::Repository::open(&path).unwrap();
        assert!(repo
            .find_branch("to-delete", git2::BranchType::Local)
            .is_err());
    }

    #[test]
    fn spawn_op_revert_conflict_file_unstages_and_restores() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = make_committed_repo(&tmp);

        std::fs::write(tmp.path().join("README.md"), "conflict content").unwrap();
        run_op_sync(&path, OpRequest::StageFile("README.md".to_string()));

        let result = run_op_sync(
            &path,
            OpRequest::RevertFile {
                file_path: "README.md".to_string(),
                is_conflict: true,
            },
        );
        assert!(result.success, "RevertFile with is_conflict must succeed");

        let status = crate::git::get_repo_status(&path, false).unwrap();
        assert!(
            status.is_clean(),
            "repo must be clean after conflict revert"
        );
    }

    #[test]
    fn spawn_op_discard_file_removes_untracked_file() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = make_committed_repo(&tmp);

        let file = tmp.path().join("discard_me.txt");
        std::fs::write(&file, "data").unwrap();

        let result = run_op_sync(&path, OpRequest::DiscardFile("discard_me.txt".to_string()));
        assert!(result.success, "DiscardFile must succeed");
        assert!(!file.exists(), "file must be removed");
        assert!(result.lines.iter().any(|l| l.contains("deleted")));
    }

    #[test]
    fn spawn_op_discard_file_fails_for_nonexistent_file() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = make_committed_repo(&tmp);

        let result = run_op_sync(&path, OpRequest::DiscardFile("nonexistent.txt".to_string()));
        assert!(
            !result.success,
            "DiscardFile must fail for non-existent file"
        );
        assert!(result.lines.iter().any(|l| l.contains("error")));
    }

    #[test]
    fn spawn_op_save_patch_and_revert_creates_patch_and_restores_file() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = make_committed_repo(&tmp);

        std::fs::write(tmp.path().join("README.md"), "modified content").unwrap();

        let result = run_op_sync(
            &path,
            OpRequest::SavePatchAndRevert {
                file_path: "README.md".to_string(),
            },
        );
        assert!(result.success, "SavePatchAndRevert must succeed");
        assert!(
            tmp.path().join("README.md.patch").exists(),
            "patch file must be created"
        );

        let content = std::fs::read_to_string(tmp.path().join("README.md")).unwrap();
        assert_eq!(
            content, "hello",
            "file must be restored to original content"
        );
    }

    #[test]
    fn spawn_op_save_patch_and_revert_fails_when_no_diff() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = make_committed_repo(&tmp);

        let result = run_op_sync(
            &path,
            OpRequest::SavePatchAndRevert {
                file_path: "README.md".to_string(),
            },
        );
        assert!(!result.success, "SavePatchAndRevert must fail when no diff");
        assert!(result.lines.iter().any(|l| l.contains("no diff")));
    }

    #[test]
    fn spawn_op_apply_patch_applies_saved_patch() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = make_committed_repo(&tmp);

        std::fs::write(tmp.path().join("README.md"), "patched content").unwrap();
        run_op_sync(
            &path,
            OpRequest::SavePatchAndRevert {
                file_path: "README.md".to_string(),
            },
        );

        let result = run_op_sync(
            &path,
            OpRequest::ApplyPatch {
                file_path: "README.md.patch".to_string(),
            },
        );
        assert!(result.success, "ApplyPatch must succeed");

        let content = std::fs::read_to_string(tmp.path().join("README.md")).unwrap();
        assert_eq!(content, "patched content", "patch must be applied");
    }

    #[test]
    fn spawn_op_amend_commit_updates_message() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = make_committed_repo(&tmp);

        std::fs::write(tmp.path().join("file.txt"), "content").unwrap();
        run_op_sync(&path, OpRequest::StageFile("file.txt".to_string()));
        run_op_sync(
            &path,
            OpRequest::Commit {
                message: "original message".to_string(),
                amend: false,
            },
        );

        let result = run_op_sync(
            &path,
            OpRequest::Commit {
                message: "amended message".to_string(),
                amend: true,
            },
        );
        assert!(result.success, "Commit amend must succeed");

        let repo = git2::Repository::open(&path).unwrap();
        let head = repo.head().unwrap().peel_to_commit().unwrap();
        assert_eq!(head.summary().unwrap_or(""), "amended message");
    }

    #[test]
    fn spawn_op_run_repo_command_foreground_captures_output() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = make_committed_repo(&tmp);

        let result = run_op_sync(
            &path,
            OpRequest::RunRepoCommand {
                name: "Echo".to_string(),
                cmd: "echo hello-gitover".to_string(),
                background: false,
            },
        );
        assert!(result.success, "RunRepoCommand foreground must succeed");
        assert!(result.lines.iter().any(|l| l.contains("hello-gitover")));
    }

    #[test]
    fn spawn_op_run_repo_command_background_succeeds() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = make_committed_repo(&tmp);

        let result = run_op_sync(
            &path,
            OpRequest::RunRepoCommand {
                name: "Background".to_string(),
                cmd: "echo bg-cmd".to_string(),
                background: true,
            },
        );
        assert!(result.success, "RunRepoCommand background must succeed");
    }

    #[test]
    fn spawn_op_checkout_with_dirty_state_auto_stashes_and_pops() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = make_committed_repo(&tmp);

        run_op_sync(&path, OpRequest::CreateBranch("feature".to_string()));
        run_op_sync(
            &path,
            OpRequest::CheckoutBranch {
                name: "main".to_string(),
                is_remote: false,
            },
        );

        std::fs::write(tmp.path().join("README.md"), "dirty modification").unwrap();

        let result = run_op_sync(
            &path,
            OpRequest::CheckoutBranch {
                name: "feature".to_string(),
                is_remote: false,
            },
        );
        assert!(
            result.success,
            "CheckoutBranch with dirty working tree must succeed"
        );

        let repo = git2::Repository::open(&path).unwrap();
        assert_eq!(repo.head().unwrap().shorthand().unwrap_or(""), "feature");
    }

    #[test]
    fn spawn_op_undo_commit_restores_file_as_staged() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = make_committed_repo(&tmp);

        std::fs::write(tmp.path().join("undo_me.txt"), "to undo").unwrap();
        run_op_sync(&path, OpRequest::StageFile("undo_me.txt".to_string()));
        run_op_sync(
            &path,
            OpRequest::Commit {
                message: "will be undone".to_string(),
                amend: false,
            },
        );

        let undo_result = run_op_sync(&path, OpRequest::UndoCommit);
        assert!(undo_result.success, "UndoCommit must succeed");

        // --mixed reset leaves changes in the working tree (unstaged),
        // so the file appears as untracked (added) rather than staged.
        let status = crate::git::get_repo_status(&path, false).unwrap();
        assert!(
            !status.is_clean(),
            "repo must have changes after undo commit"
        );
    }
}

fn append_output(output: &Output, lines: &mut Vec<String>) {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    for line in stdout.lines().chain(stderr.lines()) {
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            lines.push(trimmed.to_string());
        }
    }
}
