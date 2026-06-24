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

/// One group of repositories with an optional name.
/// `name: None` identifies the default (unnamed) section, which is always sections[0].
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RepoSection {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default)]
    pub repos: Vec<String>,
    /// Collapsed state — only meaningful for named sections; default section is never collapsed.
    #[serde(default, skip_serializing_if = "is_false")]
    pub collapsed: bool,
}

fn is_false(b: &bool) -> bool {
    !*b
}

impl RepoSection {
    fn new_default() -> Self {
        Self {
            name: None,
            repos: Vec::new(),
            collapsed: false,
        }
    }

    pub fn is_default(&self) -> bool {
        self.name.is_none()
    }
}

/// Intermediate struct for loading both the legacy flat-repos format and the new sections format.
#[derive(Deserialize)]
struct RawState {
    #[serde(default)]
    repos: Vec<String>,
    #[serde(default)]
    sections: Vec<RepoSection>,
    #[serde(default)]
    show_file_status: bool,
    #[serde(default)]
    show_log: bool,
    #[serde(default)]
    show_history: bool,
    #[serde(default)]
    show_details: bool,
}

/// Persisted application state saved to `gitover.state.yaml`.
/// Schema: `docs/state.schema.json` — update it when adding or changing fields.
///
/// `sections[0]` is always the default (unnamed) section.
/// `sections[1..]` are named sections kept in case-insensitive alphabetical order.
#[derive(Debug, Serialize, Deserialize)]
pub struct State {
    #[serde(default = "default_sections")]
    pub sections: Vec<RepoSection>,
    /// Whether the File Status pane was open on last exit.
    #[serde(default)]
    pub show_file_status: bool,
    /// Whether the Output Log pane was open on last exit.
    #[serde(default)]
    pub show_log: bool,
    /// Whether the History pane was open on last exit.
    #[serde(default)]
    pub show_history: bool,
    /// Whether the Details pane was open on last exit.
    #[serde(default)]
    pub show_details: bool,
    /// Where this state was loaded from and will be saved to.
    #[serde(skip)]
    pub path: PathBuf,
}

fn default_sections() -> Vec<RepoSection> {
    vec![RepoSection::new_default()]
}

impl Default for State {
    fn default() -> Self {
        Self {
            sections: default_sections(),
            show_file_status: false,
            show_log: false,
            show_history: false,
            show_details: false,
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
        let raw: RawState = serde_yaml::from_str(&content)
            .with_context(|| format!("parsing {}", path.display()))?;
        let base_dir = path.parent().unwrap_or(Path::new("."));

        let sections = if !raw.sections.is_empty() {
            // New format: use sections directly, ensuring default section at index 0.
            let mut sections = raw.sections;
            if sections.first().map(|s| s.name.is_some()).unwrap_or(true) {
                sections.insert(0, RepoSection::new_default());
            }
            for section in sections.iter_mut() {
                section.repos = section
                    .repos
                    .iter()
                    .map(|p| resolve_path(p, base_dir))
                    .filter(|p| Path::new(p).is_dir())
                    .collect();
                sort_section_repos(section, false);
            }
            sections
        } else if !raw.repos.is_empty() {
            // Legacy flat-list format: migrate everything into the default section.
            let repos: Vec<String> = raw
                .repos
                .into_iter()
                .map(|p| resolve_path(&p, base_dir))
                .filter(|p| Path::new(p).is_dir())
                .collect();
            let mut default = RepoSection::new_default();
            default.repos = repos;
            sort_section_repos(&mut default, false);
            vec![default]
        } else {
            default_sections()
        };

        Ok(State {
            sections,
            show_file_status: raw.show_file_status,
            show_log: raw.show_log,
            show_history: raw.show_history,
            show_details: raw.show_details,
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
            sections: self
                .sections
                .iter()
                .map(|s| RepoSection {
                    name: s.name.clone(),
                    repos: s.repos.iter().map(|p| make_relative(p, base_dir)).collect(),
                    collapsed: s.collapsed,
                })
                .collect(),
            show_file_status: self.show_file_status,
            show_log: self.show_log,
            show_history: self.show_history,
            show_details: self.show_details,
            path: PathBuf::new(),
        };
        let content = serde_yaml::to_string(&saveable)?;
        std::fs::write(path, content).with_context(|| format!("writing {}", path.display()))?;
        Ok(())
    }

    // ── Section accessors ──────────────────────────────────────────────────────

    /// The default section (always `sections[0]`).
    pub fn default_section(&self) -> &RepoSection {
        &self.sections[0]
    }

    /// Mutable reference to the default section.
    pub fn default_section_mut(&mut self) -> &mut RepoSection {
        &mut self.sections[0]
    }

    /// All repo paths from all sections in display order (default section first,
    /// then named sections in their stored order, which is alphabetical).
    pub fn all_repos_flat(&self) -> Vec<String> {
        self.sections
            .iter()
            .flat_map(|s| s.repos.iter().cloned())
            .collect()
    }

    /// Returns true if any section contains at least one repo.
    pub fn has_any_repos(&self) -> bool {
        self.sections.iter().any(|s| !s.repos.is_empty())
    }

    /// Returns true if there is at least one named section.
    pub fn has_named_sections(&self) -> bool {
        self.sections.len() > 1
    }

    /// Returns true when only named sections exist (default section is empty).
    /// In this state, new repos cannot be added directly to the default section.
    pub fn only_named_sections(&self) -> bool {
        self.has_named_sections() && self.sections[0].repos.is_empty()
    }

    /// Returns the `sections` index of the section that owns the repo at the given
    /// flat index (i.e. the index into `all_repos_flat()`).
    pub fn section_idx_for_flat_repo_idx(&self, flat_idx: usize) -> usize {
        let mut seen = 0;
        for (idx, section) in self.sections.iter().enumerate() {
            if flat_idx < seen + section.repos.len() {
                return idx;
            }
            seen += section.repos.len();
        }
        0
    }

    /// Returns the `sections` index of the section that contains `path`, or `None`.
    pub fn section_idx_for_path(&self, path: &str) -> Option<usize> {
        self.sections
            .iter()
            .enumerate()
            .find(|(_, s)| s.repos.iter().any(|p| p == path))
            .map(|(idx, _)| idx)
    }

    // ── Repo management ────────────────────────────────────────────────────────

    /// Add a repo to the default section.  Returns `true` if newly added.
    pub fn add_repo(&mut self, path: &str) -> bool {
        self.add_repo_to_section(path, 0)
    }

    /// Add a repo to `section_idx`.  Returns `false` if already tracked anywhere.
    pub fn add_repo_to_section(&mut self, path: &str, section_idx: usize) -> bool {
        if self
            .sections
            .iter()
            .any(|s| s.repos.iter().any(|p| p == path))
        {
            return false;
        }
        let idx = section_idx.min(self.sections.len().saturating_sub(1));
        self.sections[idx].repos.push(path.to_string());
        sort_section_repos(&mut self.sections[idx], false);
        true
    }

    /// Remove a repo path from whichever section contains it.
    pub fn remove_repo(&mut self, path: &str) {
        for section in self.sections.iter_mut() {
            section.repos.retain(|p| p != path);
        }
    }

    // ── Section management ─────────────────────────────────────────────────────

    /// Add a named section.  Inserts at the correct alphabetical position among
    /// `sections[1..]`.  Returns the new section's index, or `None` if the name
    /// is a case-insensitive duplicate of an existing named section.
    pub fn add_section(&mut self, name: String) -> Option<usize> {
        let lower = name.to_lowercase();
        if self
            .sections
            .iter()
            .skip(1)
            .any(|s| s.name.as_deref().map(|n| n.to_lowercase()) == Some(lower.clone()))
        {
            return None;
        }
        let section = RepoSection {
            name: Some(name),
            repos: vec![],
            collapsed: false,
        };
        let insert_pos = self
            .sections
            .iter()
            .skip(1)
            .enumerate()
            .find(|(_, s)| {
                s.name
                    .as_deref()
                    .map(|n| n.to_lowercase())
                    .unwrap_or_default()
                    > lower
            })
            .map(|(i, _)| i + 1)
            .unwrap_or(self.sections.len());
        self.sections.insert(insert_pos, section);
        Some(insert_pos)
    }

    /// Rename `sections[section_idx]` (must be >= 1).  Returns `false` if the
    /// new name is a case-insensitive duplicate or the index is invalid.
    /// After renaming the sections are re-sorted alphabetically; the returned
    /// value is the new index of the renamed section.
    pub fn rename_section(&mut self, section_idx: usize, new_name: String) -> Option<usize> {
        if section_idx == 0 || section_idx >= self.sections.len() {
            return None;
        }
        let lower = new_name.to_lowercase();
        if self
            .sections
            .iter()
            .enumerate()
            .skip(1)
            .filter(|(i, _)| *i != section_idx)
            .any(|(_, s)| s.name.as_deref().map(|n| n.to_lowercase()) == Some(lower.clone()))
        {
            return None;
        }
        self.sections[section_idx].name = Some(new_name);
        // Re-sort named sections alphabetically.
        let mut named: Vec<RepoSection> = self.sections.drain(1..).collect();
        named.sort_by(|a, b| {
            a.name
                .as_deref()
                .unwrap_or("")
                .to_lowercase()
                .cmp(&b.name.as_deref().unwrap_or("").to_lowercase())
        });
        self.sections.extend(named);
        // Find where the renamed section ended up.
        let new_idx = self
            .sections
            .iter()
            .position(|s| s.name.as_deref().map(|n| n.to_lowercase()) == Some(lower.clone()))
            .unwrap_or(section_idx);
        Some(new_idx)
    }

    /// Remove `sections[section_idx]` (must be >= 1), moving its repos to the
    /// default section.
    pub fn remove_section(&mut self, section_idx: usize) {
        if section_idx == 0 || section_idx >= self.sections.len() {
            return;
        }
        let repos = self.sections.remove(section_idx).repos;
        self.sections[0].repos.extend(repos);
        sort_section_repos(&mut self.sections[0], false);
    }

    /// Move `path` from its current section to `target_section_idx`.
    pub fn move_repo_to_section(&mut self, path: &str, target_section_idx: usize) {
        for section in self.sections.iter_mut() {
            section.repos.retain(|p| p != path);
        }
        if target_section_idx < self.sections.len() {
            self.sections[target_section_idx]
                .repos
                .push(path.to_string());
            sort_section_repos(&mut self.sections[target_section_idx], false);
        }
    }

    /// Re-sort repos within every section using the given case-sensitivity setting.
    /// Call this after loading config to apply the user's sorting preference.
    pub fn sort_all_section_repos(&mut self, case_sensitive: bool) {
        for section in self.sections.iter_mut() {
            sort_section_repos(section, case_sensitive);
        }
    }
}

// ── Sorting helper ─────────────────────────────────────────────────────────────

pub(crate) fn sort_section_repos(section: &mut RepoSection, case_sensitive: bool) {
    if case_sensitive {
        section.repos.sort();
    } else {
        section.repos.sort_by_key(|p| p.to_lowercase());
    }
}

// ── File-system helpers ────────────────────────────────────────────────────────

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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn repo_section_default_has_no_name() {
        let section = RepoSection::new_default();
        assert!(section.is_default());
        assert!(section.name.is_none());
    }

    #[test]
    fn repo_section_named_is_not_default() {
        let section = RepoSection {
            name: Some("Work".to_string()),
            repos: vec![],
            collapsed: false,
        };
        assert!(!section.is_default());
    }

    #[test]
    fn find_state_path_returns_a_path() {
        let p = find_state_path();
        assert!(!p.to_string_lossy().is_empty());
    }

    #[test]
    fn default_section_returns_immutable_ref_to_first_section() {
        let state = State::default();
        assert!(state.default_section().is_default());
        assert!(state.default_section().repos.is_empty());
    }

    #[test]
    fn default_section_mut_allows_modifying_first_section() {
        let mut state = State::default();
        state.default_section_mut().repos.push("/some/path".to_string());
        assert_eq!(state.sections[0].repos.len(), 1);
    }

    #[test]
    fn only_named_sections_false_when_no_named_sections() {
        let state = State::default();
        assert!(!state.only_named_sections());
    }

    #[test]
    fn only_named_sections_true_when_default_empty_and_named_exists() {
        let mut state = State::default();
        state.add_section("Work".to_string());
        assert!(state.only_named_sections());
    }

    #[test]
    fn section_idx_for_flat_repo_idx_finds_correct_section() {
        let mut state = State::default();
        state.sections[0].repos.push("/repo/a".to_string());
        state.sections[0].repos.push("/repo/b".to_string());
        state.add_section("Work".to_string());
        state.sections[1].repos.push("/repo/c".to_string());

        assert_eq!(state.section_idx_for_flat_repo_idx(0), 0);
        assert_eq!(state.section_idx_for_flat_repo_idx(1), 0);
        assert_eq!(state.section_idx_for_flat_repo_idx(2), 1);
        assert_eq!(state.section_idx_for_flat_repo_idx(99), 0);
    }

    #[test]
    fn section_idx_for_path_finds_repo_in_named_section() {
        let mut state = State::default();
        state.add_section("Work".to_string());
        state.sections[1].repos.push("/work/repo".to_string());

        assert_eq!(state.section_idx_for_path("/work/repo"), Some(1));
        assert_eq!(state.section_idx_for_path("/not/there"), None);
    }

    #[test]
    fn sort_section_repos_case_sensitive_uses_ascii_order() {
        let mut section = RepoSection {
            name: None,
            repos: vec!["/b/repo".to_string(), "/A/repo".to_string()],
            collapsed: false,
        };
        sort_section_repos(&mut section, true);
        assert_eq!(section.repos[0], "/A/repo");
    }

    #[test]
    fn rename_section_invalid_idx_returns_none() {
        let mut state = State::default();
        assert!(state.rename_section(0, "Name".to_string()).is_none());
        assert!(state.rename_section(99, "Name".to_string()).is_none());
    }

    #[test]
    fn rename_section_duplicate_name_returns_none() {
        let mut state = State::default();
        state.add_section("Alpha".to_string());
        state.add_section("Beta".to_string());
        let alpha_idx = state
            .sections
            .iter()
            .position(|s| s.name.as_deref() == Some("Alpha"))
            .unwrap();
        assert!(state.rename_section(alpha_idx, "Beta".to_string()).is_none());
    }

    #[test]
    fn remove_section_idx_zero_is_noop() {
        let mut state = State::default();
        state.add_section("Work".to_string());
        let len = state.sections.len();
        state.remove_section(0);
        assert_eq!(state.sections.len(), len);
    }

    #[test]
    fn remove_section_out_of_bounds_is_noop() {
        let mut state = State::default();
        let len = state.sections.len();
        state.remove_section(99);
        assert_eq!(state.sections.len(), len);
    }

    #[test]
    fn state_save_creates_nested_parent_dirs() {
        let tmp = TempDir::new().unwrap();
        let state_path = tmp.path().join("nested").join("dir").join("state.yaml");
        let state = State {
            path: state_path.clone(),
            ..Default::default()
        };
        state.save().unwrap();
        assert!(state_path.exists());
    }

    #[test]
    fn state_load_from_yaml_with_empty_content_uses_default_sections() {
        let tmp = TempDir::new().unwrap();
        let state_path = tmp.path().join("s.yaml");
        std::fs::write(&state_path, "{}\n").unwrap();
        let loaded = State::load_from_path(state_path);
        assert_eq!(loaded.sections.len(), 1);
        assert!(loaded.sections[0].is_default());
    }

    #[test]
    fn state_load_from_yaml_inserts_default_section_when_named_is_first() {
        let tmp = TempDir::new().unwrap();
        let state_path = tmp.path().join("s.yaml");
        std::fs::write(&state_path, "sections:\n  - name: Work\n    repos: []\n").unwrap();
        let loaded = State::load_from_path(state_path);
        assert!(
            loaded.sections[0].is_default(),
            "default section must be at index 0"
        );
        assert_eq!(loaded.sections.len(), 2);
        assert_eq!(loaded.sections[1].name.as_deref(), Some("Work"));
    }
}
