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

/// Application configuration loaded from `~/.config/gitover/config.yaml`.
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
    /// Load config from `~/.config/gitover/config.yaml`.
    /// Returns a default (empty) config if the file does not exist or cannot
    /// be parsed, so a missing config is always a valid state.
    pub fn load() -> Self {
        Self::load_from(&config_path())
    }

    /// Load config from an explicit path. Useful for tests.
    pub fn load_from(path: &std::path::Path) -> Self {
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

/// Returns the canonical config file path: `~/.config/gitover/config.yaml`.
pub fn config_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home)
        .join(".config")
        .join("gitover")
        .join("config.yaml")
}
