use anyhow::Result;
use git2::{Repository, Status};

#[derive(Debug, Clone)]
pub struct AheadBehind {
    pub ahead: usize,
    pub behind: usize,
    /// Human-readable name of the reference being compared against
    /// (e.g. "origin/main", "origin/develop").
    pub branch: String,
}

/// One entry in a repo's changed-files list — drives the detail panel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileStatusKind {
    Staged,
    Modified,
    Deleted,
    Conflict,
    Untracked,
}

impl FileStatusKind {
    pub fn code(&self) -> &'static str {
        match self {
            FileStatusKind::Staged => "S",
            FileStatusKind::Modified => "M",
            FileStatusKind::Deleted => "D",
            FileStatusKind::Conflict => "C",
            FileStatusKind::Untracked => "U",
        }
    }

    #[allow(dead_code)]
    pub fn label(&self) -> &'static str {
        match self {
            FileStatusKind::Staged => "staged",
            FileStatusKind::Modified => "modified",
            FileStatusKind::Deleted => "deleted",
            FileStatusKind::Conflict => "conflict",
            FileStatusKind::Untracked => "untracked",
        }
    }

    /// Sort priority for display: Conflict first, then Staged, Modified,
    /// Deleted, Untracked (lowest priority).
    pub fn sort_priority(&self) -> u8 {
        match self {
            FileStatusKind::Conflict => 0,
            FileStatusKind::Staged => 1,
            FileStatusKind::Modified => 2,
            FileStatusKind::Deleted => 3,
            FileStatusKind::Untracked => 4,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FileEntry {
    pub path: String,
    pub status: FileStatusKind,
}

#[derive(Debug, Clone)]
pub struct RepoStatus {
    pub path: String,
    pub branch: String,
    pub added: usize,
    pub modified: usize,
    pub staged: usize,
    pub deleted: usize,
    pub conflict: usize,
    /// ahead/behind vs the configured upstream tracking branch
    pub upstream: Option<AheadBehind>,
    /// ahead/behind vs the trunk branch
    pub trunk: Option<AheadBehind>,
    /// All local branch names
    pub local_branches: Vec<String>,
    /// Remote branches that have no corresponding local branch
    pub remote_only_branches: Vec<String>,
    /// Local branches already merged into the trunk branch
    pub merged_branches: Vec<String>,
    /// Per-file status entries, used by the detail panel.
    pub files: Vec<FileEntry>,
    /// Error message when scanning this path failed (invalid path, not a git
    /// repo, etc). When `Some`, the entry should be rendered as an error row.
    pub error: Option<String>,
}

impl RepoStatus {
    pub fn is_clean(&self) -> bool {
        self.error.is_none()
            && self.added == 0
            && self.modified == 0
            && self.staged == 0
            && self.deleted == 0
            && self.conflict == 0
    }

    /// Construct a minimal placeholder entry used when `get_repo_status` fails.
    /// This keeps the path visible in the table with an inline error message
    /// instead of silently dropping it.
    pub fn error_entry(path: &str, msg: impl Into<String>) -> Self {
        RepoStatus {
            path: path.to_string(),
            branch: String::new(),
            added: 0,
            modified: 0,
            staged: 0,
            deleted: 0,
            conflict: 0,
            upstream: None,
            trunk: None,
            local_branches: Vec::new(),
            remote_only_branches: Vec::new(),
            merged_branches: Vec::new(),
            files: Vec::new(),
            error: Some(msg.into()),
        }
    }
}

pub fn get_repo_status(path: &str) -> Result<RepoStatus> {
    let repo = Repository::open(path)?;

    let branch = get_branch_name(&repo);
    let statuses = repo.statuses(None)?;

    let mut added = 0usize;
    let mut modified = 0usize;
    let mut staged = 0usize;
    let mut deleted = 0usize;
    let mut conflict = 0usize;
    let mut files: Vec<FileEntry> = Vec::new();

    for entry in statuses.iter() {
        let s = entry.status();
        let entry_path = entry.path().unwrap_or("?").to_string();

        if s.contains(Status::CONFLICTED) {
            conflict += 1;
            files.push(FileEntry {
                path: entry_path,
                status: FileStatusKind::Conflict,
            });
            continue;
        }
        if s.contains(Status::INDEX_NEW)
            || s.contains(Status::INDEX_MODIFIED)
            || s.contains(Status::INDEX_DELETED)
            || s.contains(Status::INDEX_RENAMED)
            || s.contains(Status::INDEX_TYPECHANGE)
        {
            staged += 1;
            files.push(FileEntry {
                path: entry_path.clone(),
                status: FileStatusKind::Staged,
            });
        }
        if s.contains(Status::WT_NEW) {
            added += 1;
            files.push(FileEntry {
                path: entry_path.clone(),
                status: FileStatusKind::Untracked,
            });
        }
        if s.contains(Status::WT_MODIFIED) || s.contains(Status::WT_TYPECHANGE) {
            modified += 1;
            files.push(FileEntry {
                path: entry_path.clone(),
                status: FileStatusKind::Modified,
            });
        }
        if s.contains(Status::WT_DELETED) {
            deleted += 1;
            files.push(FileEntry {
                path: entry_path,
                status: FileStatusKind::Deleted,
            });
        }
    }

    let upstream = get_ahead_behind_upstream(&repo);
    let trunk = get_ahead_behind_trunk(&repo);
    let local_branches = get_local_branches(&repo);
    let remote_only_branches = get_remote_only_branches(&repo, &local_branches);
    let merged_branches = get_merged_branches(&repo);

    // Sort files: Conflict → Staged → Modified → Deleted → Untracked,
    // then alphabetically within each group.
    files.sort_by(|a, b| {
        a.status
            .sort_priority()
            .cmp(&b.status.sort_priority())
            .then_with(|| a.path.cmp(&b.path))
    });

    Ok(RepoStatus {
        path: path.to_string(),
        branch,
        added,
        modified,
        staged,
        deleted,
        conflict,
        upstream,
        trunk,
        local_branches,
        remote_only_branches,
        merged_branches,
        files,
        error: None,
    })
}

fn get_branch_name(repo: &Repository) -> String {
    match repo.head() {
        Ok(head) => {
            if head.is_branch() {
                // Normal branch — return its short name
                head.shorthand().unwrap_or("unknown").to_string()
            } else {
                // Detached HEAD — show first 8 chars of the commit SHA
                match head.target() {
                    Some(oid) => format!("detached {}", &oid.to_string()[..8]),
                    None => "detached".to_string(),
                }
            }
        }
        Err(ref e) if e.code() == git2::ErrorCode::UnbornBranch => {
            // The repo has no commits yet but HEAD points to a branch name.
            // Read the branch name directly from the HEAD symbolic reference.
            repo.find_reference("HEAD")
                .ok()
                .and_then(|r| r.symbolic_target().map(|s| s.to_string()))
                .and_then(|refname| {
                    // refname is e.g. "refs/heads/main" — strip the prefix
                    refname.strip_prefix("refs/heads/").map(|s| s.to_string())
                })
                .unwrap_or_else(|| "unborn".to_string())
        }
        Err(_) => "detached".to_string(),
    }
}

/// ahead/behind vs the tracking branch configured for the current branch.
fn get_ahead_behind_upstream(repo: &Repository) -> Option<AheadBehind> {
    let head = repo.head().ok()?;
    let local_oid = head.target()?;

    let branch_name = head.shorthand()?;
    let branch = repo
        .find_branch(branch_name, git2::BranchType::Local)
        .ok()?;
    let upstream = branch.upstream().ok()?;
    let upstream_oid = upstream.get().target()?;

    // Capture the upstream branch's short name for display
    let upstream_branch = upstream
        .name()
        .ok()
        .flatten()
        .unwrap_or("upstream")
        .to_string();

    let (ahead, behind) = repo.graph_ahead_behind(local_oid, upstream_oid).ok()?;
    Some(AheadBehind {
        ahead,
        behind,
        branch: upstream_branch,
    })
}

/// ahead/behind vs the trunk branch.
/// Resolution order:
///   1. git config gitover.trunkbranch  (local repo config)
///   2. origin/main
///   3. origin/develop
///   4. origin/master
fn get_ahead_behind_trunk(repo: &Repository) -> Option<AheadBehind> {
    let head = repo.head().ok()?;
    let local_oid = head.target()?;

    let (trunk_ref, trunk_name) = resolve_trunk_ref(repo)?;
    let trunk_oid = trunk_ref.target()?;

    let (ahead, behind) = repo.graph_ahead_behind(local_oid, trunk_oid).ok()?;
    Some(AheadBehind {
        ahead,
        behind,
        branch: trunk_name,
    })
}

fn resolve_trunk_ref(repo: &Repository) -> Option<(git2::Reference<'_>, String)> {
    // 1. Check gitover.trunkbranch in repo config
    if let Ok(cfg) = repo.config() {
        if let Ok(name) = cfg.get_string("gitover.trunkbranch") {
            let refname = format!("refs/remotes/{name}");
            if let Ok(r) = repo.find_reference(&refname) {
                return Some((r, name));
            }
            let refname2 = format!("refs/remotes/origin/{name}");
            let display = format!("origin/{name}");
            if let Ok(r) = repo.find_reference(&refname2) {
                return Some((r, display));
            }
        }
    }

    // 2. origin/main
    if let Ok(r) = repo.find_reference("refs/remotes/origin/main") {
        return Some((r, "origin/main".to_string()));
    }

    // 3. origin/develop
    if let Ok(r) = repo.find_reference("refs/remotes/origin/develop") {
        return Some((r, "origin/develop".to_string()));
    }

    // 4. origin/master
    if let Ok(r) = repo.find_reference("refs/remotes/origin/master") {
        return Some((r, "origin/master".to_string()));
    }

    None
}

/// Return the names of all local branches, sorted alphabetically.
fn get_local_branches(repo: &Repository) -> Vec<String> {
    let mut names = Vec::new();
    if let Ok(branches) = repo.branches(Some(git2::BranchType::Local)) {
        for (branch, _) in branches.flatten() {
            if let Ok(Some(name)) = branch.name() {
                names.push(name.to_string());
            }
        }
    }
    names.sort();
    names
}

/// Return remote branch names (stripped of the remote prefix, e.g. "origin/")
/// that have no corresponding local branch.
fn get_remote_only_branches(repo: &Repository, local_branches: &[String]) -> Vec<String> {
    let mut remote_only = Vec::new();
    if let Ok(branches) = repo.branches(Some(git2::BranchType::Remote)) {
        for (branch, _) in branches.flatten() {
            if let Ok(Some(full_name)) = branch.name() {
                // Skip HEAD pointer refs like "origin/HEAD"
                if full_name.ends_with("/HEAD") {
                    continue;
                }
                // Strip the remote prefix: "origin/feature-x" → "feature-x"
                let short = full_name
                    .find('/')
                    .map(|i| &full_name[i + 1..])
                    .unwrap_or(full_name);
                if !local_branches.iter().any(|l| l == short) {
                    remote_only.push(full_name.to_string());
                }
            }
        }
    }
    remote_only.sort();
    remote_only
}

/// Return local branches that are fully merged into the trunk branch.
/// A branch is "merged" if the trunk commit is an ancestor of or equal to
/// the branch tip (i.e. trunk is ahead or equal — branch has 0 ahead commits).
fn get_merged_branches(repo: &Repository) -> Vec<String> {
    let mut merged = Vec::new();

    let trunk_oid = match resolve_trunk_ref(repo) {
        Some((r, _)) => match r.target() {
            Some(oid) => oid,
            None => return merged,
        },
        None => return merged,
    };

    if let Ok(branches) = repo.branches(Some(git2::BranchType::Local)) {
        for (branch, _) in branches.flatten() {
            let name = match branch.name() {
                Ok(Some(n)) => n.to_string(),
                _ => continue,
            };
            let branch_oid = match branch.get().target() {
                Some(oid) => oid,
                None => continue,
            };
            // A branch is merged if it has 0 commits ahead of trunk
            if let Ok((ahead, _behind)) = repo.graph_ahead_behind(branch_oid, trunk_oid) {
                if ahead == 0 {
                    merged.push(name);
                }
            }
        }
    }
    merged.sort();
    merged
}
