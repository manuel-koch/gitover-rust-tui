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

use std::process::{Command, Output, Stdio};
use std::sync::mpsc::Sender;
use std::time::SystemTime;

/// Result of a background git operation sent back to the main thread.
pub struct OpResult {
    pub repo_path: String,
    pub op_label: String,
    pub success: bool,
    pub lines: Vec<String>,
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
    PullBranch { name: String, upstream: String },
    /// Run a custom shell command from config (already interpolated).
    RunRepoCommand {
        name: String,
        cmd: String,
        /// When true, spawn without waiting and discard output.
        background: bool,
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
            OpRequest::CreateBranch(_) => "create branch".into(),
            OpRequest::DeleteBranch(_) => "delete branch".into(),
            OpRequest::StageFile(_) => "stage file".into(),
            OpRequest::UnstageFile(_) => "unstage file".into(),
            OpRequest::RevertFile { .. } => "revert file".into(),
            OpRequest::DiscardFile(_) => "discard file".into(),
            OpRequest::PullBranch { name, .. } => format!("pull branch {name}"),
            OpRequest::RunRepoCommand { name, .. } => name.clone(),
        }
    }
}

/// Spawn a background thread that executes `request` and sends the result to `tx`.
pub fn spawn_op(repo_path: String, request: OpRequest, git_bin: String, tx: Sender<OpResult>) {
    std::thread::spawn(move || {
        let label = request.label();
        let (success, lines) = run_op(&repo_path, &request, &git_bin);
        let _ = tx.send(OpResult {
            repo_path,
            op_label: label,
            success,
            lines,
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

        OpRequest::PullBranch { name, upstream } => {
            run_git(git_bin, repo_path, &["branch", "-f", name, upstream], &mut lines)
        }

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
