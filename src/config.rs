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

use serde::Deserialize;
use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;

/// A custom command that can be run against the selected repository.
#[derive(Debug, Default, Clone, Deserialize)]
pub struct RepoCommand {
    /// Human-readable name shown in the action menu.
    pub name: String,
    /// Shell command to execute. Supports variables:
    /// - $ROOT : repo root path
    /// - $BRANCH : current branch name
    pub cmd: String,
    /// When true the command is spawned without waiting for it to finish and its output is discarded.
    #[serde(default)]
    pub background: bool,
}

/// Application configuration loaded from `gitover.config.yaml` (CWD-local or global).
/// Schema: `docs/config.schema.json` — update it when adding or changing fields.
#[derive(Debug, Default, Clone, Deserialize)]
#[allow(dead_code)]
pub struct Config {
    #[serde(default)]
    pub general: GeneralConfig,
    /// Optional list of custom repo commands shown at the bottom of the action menu.
    #[serde(default)]
    pub repo_commands: Vec<RepoCommand>,
}

#[derive(Debug, Default, Clone, Deserialize)]
pub struct GeneralConfig {
    /// Path to the git executable. When set it will be used instead of
    /// whatever `git` is on `$PATH`.
    pub git: Option<String>,
    /// Interval in seconds for automatic background fetch of all repos.
    /// Defaults to 600 seconds (10 minutes) if not set.
    #[serde(default)]
    pub auto_fetch_interval: Option<u64>,
    /// Path to the debug log file. When set, debug logging is written to this
    /// file (appended if it already exists). Overridden by `--debug-log` CLI flag.
    #[serde(default)]
    pub debug_log: Option<String>,
    /// When true, paths are sorted case-sensitively across all panes.
    /// Defaults to false (case-insensitive sorting).
    #[serde(default)]
    pub case_sensitive_path_sorting: bool,
}

impl GeneralConfig {
    /// Get the auto_fetch_interval as Duration, falling back to default (600 seconds) if not set.
    /// Set to 0 to disable automatic fetching completely.
    pub fn auto_fetch_interval(&self) -> Duration {
        self.auto_fetch_interval
            .map(Duration::from_secs)
            .unwrap_or(Duration::from_secs(600))
    }
}

impl Config {
    /// Load config by searching for `gitover.config.yaml` from CWD upward,
    /// falling back to `~/.config/gitover/config.yaml`.
    /// Returns a default (empty) config if no file is found or parsing fails.
    pub fn load() -> Self {
        Self::load_from(&find_config_path())
    }

    /// Load config from an explicit path. Useful for tests.
    pub fn load_from(path: &Path) -> Self {
        match std::fs::read_to_string(path) {
            Ok(content) => match serde_yaml::from_str::<Config>(&content) {
                Ok(cfg) => cfg,
                Err(e) => {
                    eprintln!("gitover: failed to parse {}: {e}", path.display());
                    Config::default()
                }
            },
            Err(_) => Config::default(), // file absent — that's fine
        }
    }
}

/// Walk from CWD up to the root looking for `gitover.config.yaml`.
/// Falls back to `~/.config/gitover/config.yaml` if none is found.
pub fn find_config_path() -> PathBuf {
    if let Ok(cwd) = std::env::current_dir() {
        let mut dir: &Path = &cwd;
        loop {
            let candidate = dir.join("gitover.config.yaml");
            if candidate.exists() {
                return candidate;
            }
            match dir.parent() {
                Some(p) => dir = p,
                None => break,
            }
        }
    }
    global_config_path()
}

fn global_config_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home)
        .join(".config")
        .join("gitover")
        .join("config.yaml")
}
