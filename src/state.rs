use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Persisted application state saved to `~/.config/gitover/state.yaml`.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct State {
    /// Ordered list of currently-tracked repository paths.
    #[serde(default)]
    pub repos: Vec<String>,
    /// Recently used repos (path + display name).  Capped at 20 entries.
    #[serde(default)]
    pub recent: Vec<RecentRepo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentRepo {
    pub path: String,
    pub name: String,
}

const MAX_RECENT: usize = 20;

impl State {
    pub fn load() -> Self {
        Self::try_load().unwrap_or_default()
    }

    fn try_load() -> Result<Self> {
        let path = state_path();
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;
        let state: State = serde_yaml::from_str(&content)
            .with_context(|| format!("parsing {}", path.display()))?;
        // Drop entries whose paths no longer exist
        Ok(State {
            repos: state
                .repos
                .into_iter()
                .filter(|p| std::path::Path::new(p).is_dir())
                .collect(),
            recent: state
                .recent
                .into_iter()
                .filter(|r| std::path::Path::new(&r.path).is_dir())
                .collect(),
        })
    }

    pub fn save(&self) -> Result<()> {
        let path = state_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating {}", parent.display()))?;
        }
        let content = serde_yaml::to_string(self)?;
        std::fs::write(&path, content).with_context(|| format!("writing {}", path.display()))?;
        Ok(())
    }

    /// Add a repo path to the tracked list (no-op if already tracked).
    /// Returns true if it was newly added.
    pub fn add_repo(&mut self, path: &str) -> bool {
        if self.repos.iter().any(|p| p == path) {
            return false;
        }
        self.repos.push(path.to_string());
        self.repos.sort_by_key(|p| p.to_lowercase());
        self.add_recent(path);
        true
    }

    /// Remove a repo path from the tracked list.
    pub fn remove_repo(&mut self, path: &str) {
        self.repos.retain(|p| p != path);
    }

    /// Record a path as recently used.
    fn add_recent(&mut self, path: &str) {
        if self.recent.iter().any(|r| r.path == path) {
            return;
        }
        let name = std::path::Path::new(path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(path)
            .to_string();
        self.recent.push(RecentRepo {
            path: path.to_string(),
            name,
        });
        self.recent.sort_by_key(|r| r.path.to_lowercase());
        if self.recent.len() > MAX_RECENT {
            self.recent.truncate(MAX_RECENT);
        }
    }
}

fn state_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home)
        .join(".config")
        .join("gitover")
        .join("state.yaml")
}
