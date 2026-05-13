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
use std::collections::HashMap;
use std::path::PathBuf;

/// Application configuration loaded from `~/.config/gitover/config.yaml`.
#[derive(Debug, Default, Deserialize)]
#[allow(dead_code)]
pub struct Config {
    #[serde(default)]
    pub general: GeneralConfig,
    /// Named commands that can be executed for any repo.
    #[serde(default)]
    pub repo_commands: Vec<RepoCommand>,
    /// Named commands scoped to a specific file status (e.g. "modified").
    #[serde(default)]
    pub status_commands: HashMap<String, Vec<RepoCommand>>,
}

#[derive(Debug, Default, Deserialize)]
#[allow(dead_code)]
pub struct GeneralConfig {
    /// Path to the git executable. When set it will be used instead of
    /// whatever `git` is on `$PATH`.
    pub git: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct RepoCommand {
    /// Shell command to execute. Required.
    pub cmd: String,
    /// Internal identifier (defaults to first word of `cmd`).
    #[serde(default)]
    pub name: String,
    /// Display label shown in the UI.
    #[serde(default)]
    pub title: String,
    /// Optional keyboard shortcut hint.
    #[serde(default)]
    pub shortcut: String,
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
