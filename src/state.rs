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

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Persisted application state saved to `gitover.state.yaml`.
#[derive(Debug, Serialize, Deserialize)]
pub struct State {
    /// Ordered list of currently-tracked repository paths.
    #[serde(default)]
    pub repos: Vec<String>,
    /// Whether the File Status pane was open on last exit.
    #[serde(default)]
    pub show_file_status: bool,
    /// Whether the Output Log pane was open on last exit.
    #[serde(default)]
    pub show_log: bool,
    /// Whether the History pane was open on last exit.
    #[serde(default)]
    pub show_history: bool,
    /// Whether the Diff pane was open on last exit.
    #[serde(default)]
    pub show_diff: bool,
    /// Where this state was loaded from and will be saved to.
    #[serde(skip)]
    pub path: PathBuf,
}

impl Default for State {
    fn default() -> Self {
        Self {
            repos: Vec::new(),
            show_file_status: false,
            show_log: false,
            show_history: false,
            show_diff: false,
            path: global_state_path(),
        }
    }
}

impl State {
    pub fn load() -> Self {
        let path = find_state_path();
        Self::try_load_from(&path).unwrap_or_else(|_| State {
            path,
            ..Default::default()
        })
    }

    /// Load state from an explicit path (e.g. from `--state` CLI override).
    /// If the file does not exist, returns a default state that will save to `path`.
    pub fn load_from_path(path: PathBuf) -> Self {
        Self::try_load_from(&path).unwrap_or_else(|_| State {
            path,
            ..Default::default()
        })
    }

    fn try_load_from(path: &Path) -> Result<Self> {
        let content =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        let raw: State = serde_yaml::from_str(&content)
            .with_context(|| format!("parsing {}", path.display()))?;
        let base_dir = path.parent().unwrap_or(Path::new("."));
        let repos = raw
            .repos
            .into_iter()
            .map(|p| resolve_path(&p, base_dir))
            .filter(|p| Path::new(p).is_dir())
            .collect();
        Ok(State {
            repos,
            show_file_status: raw.show_file_status,
            show_log: raw.show_log,
            show_history: raw.show_history,
            show_diff: raw.show_diff,
            path: path.to_path_buf(),
        })
    }

    pub fn save(&self) -> Result<()> {
        let path = &self.path;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating {}", parent.display()))?;
        }
        let base_dir = path.parent().unwrap_or(Path::new("."));
        let saveable = State {
            repos: self
                .repos
                .iter()
                .map(|p| make_relative(p, base_dir))
                .collect(),
            show_file_status: self.show_file_status,
            show_log: self.show_log,
            show_history: self.show_history,
            show_diff: self.show_diff,
            path: PathBuf::new(),
        };
        let content = serde_yaml::to_string(&saveable)?;
        std::fs::write(path, content).with_context(|| format!("writing {}", path.display()))?;
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
        true
    }

    /// Remove a repo path from the tracked list.
    pub fn remove_repo(&mut self, path: &str) {
        self.repos.retain(|p| p != path);
    }
}

/// Walk from CWD up to the root looking for `gitover.state.yaml`.
/// Falls back to `~/.config/gitover/state.yaml` if none is found.
pub fn find_state_path() -> PathBuf {
    if let Ok(cwd) = std::env::current_dir() {
        let mut dir: &Path = &cwd;
        loop {
            let candidate = dir.join("gitover.state.yaml");
            if candidate.exists() {
                return candidate;
            }
            match dir.parent() {
                Some(p) => dir = p,
                None => break,
            }
        }
    }
    global_state_path()
}

fn global_state_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home)
        .join(".config")
        .join("gitover")
        .join("state.yaml")
}

/// If `raw` is a relative path, resolve it against `base_dir`; otherwise return as-is.
fn resolve_path(raw: &str, base_dir: &Path) -> String {
    let p = Path::new(raw);
    if p.is_absolute() {
        raw.to_string()
    } else {
        base_dir.join(p).to_string_lossy().into_owned()
    }
}

/// If `abs` is under `base_dir`, strip the prefix and return the relative form;
/// otherwise return `abs` unchanged.
fn make_relative(abs: &str, base_dir: &Path) -> String {
    let p = Path::new(abs);
    match p.strip_prefix(base_dir) {
        Ok(rel) => rel.to_string_lossy().into_owned(),
        Err(_) => abs.to_string(),
    }
}
