use std::process::{Command, Output};
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
}

impl OpRequest {
    pub fn label(&self) -> &'static str {
        match self {
            OpRequest::Fetch => "fetch",
            OpRequest::Pull => "pull",
            OpRequest::Push => "push",
            OpRequest::ForcePush => "force push",
            OpRequest::CheckoutBranch { .. } => "checkout",
            OpRequest::CreateBranch(_) => "create branch",
            OpRequest::DeleteBranch(_) => "delete branch",
        }
    }
}

/// Spawn a background thread that executes `request` and sends the result to `tx`.
pub fn spawn_op(repo_path: String, request: OpRequest, git_bin: String, tx: Sender<OpResult>) {
    std::thread::spawn(move || {
        let label = request.label().to_string();
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
