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

use crate::config::Config;
pub use crate::git::HistoryFilter;
use crate::git::{CommitEntry, FileStatusKind, RepoStatus};
use crate::state::State;
use ratatui_explorer::FileExplorer;
use std::collections::HashMap;
use std::time::Instant;

/// What kind of git operation is currently running for a repo.
/// Drives the per-repo busy indicator + activity column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum RepoOperation {
    Scanning,
    Fetching,
    Pulling,
    Pushing,
    Rebasing,
}

impl RepoOperation {
    pub fn label(&self) -> &'static str {
        match self {
            RepoOperation::Scanning => "scanning",
            RepoOperation::Fetching => "fetching",
            RepoOperation::Pulling => "pulling",
            RepoOperation::Pushing => "pushing",
            RepoOperation::Rebasing => "rebasing",
        }
    }
}

/// One line in the output log panel.
#[derive(Debug, Clone)]
pub struct LogLine {
    /// Wall-clock time the line was recorded, formatted as `HH:MM:SS`.
    pub timestamp: String,
    pub text: String,
}

/// Which pane currently has keyboard focus. Tab cycles through the visible
/// panes (Repos is always visible; Detail/Log only when their panel is open).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Repos,
    Detail,
    Log,
    History,
}

/// What the UI is currently showing.
#[derive(Debug, PartialEq)]
pub enum AppMode {
    /// Normal table view.
    Normal,
    /// File-picker popup for choosing a repo directory.
    FilePicker,
    /// Confirmation dialog before removing the selected repo.
    ConfirmRemove,
    /// Per-repo action menu (Enter on a repo row).
    ActionMenu,
    /// Branch selection list (checkout branch).
    BranchSelect,
    /// Text input for creating a new branch.
    NewBranchInput,
    /// Confirmation dialog for force-push.
    ConfirmForcePush,
    /// Confirmation dialog for deleting a branch.
    ConfirmDeleteBranch,
    /// Commit history pane (h key).
    History,
}

/// One entry in the action menu.
#[derive(Debug, Clone)]
pub struct MenuItem {
    pub label: String,
    pub key: char,
}

/// An item in the branch selection list.
#[derive(Debug, Clone)]
pub struct BranchItem {
    pub name: String,
    pub is_remote: bool,
}

pub struct App {
    pub repos: Vec<RepoStatus>,
    pub selected: usize,
    pub should_quit: bool,
    pub last_refreshed: Option<Instant>,
    pub mode: AppMode,
    /// File explorer widget used in FilePicker mode.
    pub file_explorer: Option<FileExplorer>,
    /// Recent repos loaded from state (path, display-name).
    pub recent_repos: Vec<(String, String)>,
    /// Loaded application configuration.
    #[allow(dead_code)]
    pub config: Config,
    /// Persisted state (repo list, recents).
    pub state: State,

    // ── UX Polish ─────────────────────────────────────────────────────────────
    /// True while the initial/global repo scan is running.
    pub scanning: bool,
    /// Per-repo active operation (busy indicator + activity column).
    pub operations: HashMap<String, RepoOperation>,
    /// Timestamped log lines from git command output.
    pub log: Vec<LogLine>,
    /// Whether the detail panel (per-file status) is shown.
    pub show_detail: bool,
    /// Whether the output log panel is shown.
    pub show_log: bool,
    /// Top row of the repo table viewport (for scrolling).
    pub table_offset: usize,
    /// Spinner animation tick — incremented on each event-loop tick.
    pub spinner_tick: u64,
    /// Currently focused pane (Tab cycles through visible panes).
    pub focus: Focus,
    /// Selected row inside the detail panel (per-file list).
    pub detail_selected: usize,
    /// Scroll offset (top row) of the detail panel.
    pub detail_scroll: usize,
    /// Scroll offset of the log panel, measured in lines **back from the tail**.
    /// 0  = show the most-recent entries (tail).
    /// N  = show the view starting N lines before the tail.
    /// This representation is stable when new entries are appended: the relative
    /// position the user has scrolled to never moves as the log grows.
    pub log_offset: usize,
    /// When true the log panel auto-follows new entries (scrolls to tail).
    /// Cleared when the user scrolls up; restored when they reach the bottom.
    pub log_follow: bool,
    /// Next time the automatic background fetch of all repos should fire.
    /// Resets to `Instant::now() + AUTO_FETCH_INTERVAL` after each fetch.
    pub next_auto_fetch: Option<Instant>,

    // ── Git operations ────────────────────────────────────────────────────────
    /// Items shown in the action menu popup.
    pub menu_items: Vec<MenuItem>,
    /// Currently highlighted action menu item.
    pub menu_selected: usize,
    /// Branches shown in the branch-select popup.
    pub branch_items: Vec<BranchItem>,
    /// Currently highlighted branch-select item.
    pub branch_selected: usize,
    /// Text being typed in the new-branch-name input.
    pub branch_input: String,
    /// Branch name staged for deletion (shown in confirm dialog).
    pub branch_to_delete: String,

    /// Index into `theme::THEMES` for the active theme.
    pub theme_idx: usize,

    // ── Git History ───────────────────────────────────────────────────────────
    /// Commit history for the repo that was selected when `h` was pressed.
    pub history: Vec<CommitEntry>,
    /// Whether the history pane is visible (persistent, independent of mode).
    pub show_history: bool,
    /// Path of the repo whose history is loaded (used to detect staleness).
    pub history_repo_path: String,
    /// Active filter applied to the history.
    pub history_filter: HistoryFilter,
    /// Currently highlighted row in the flat history list (commits + file sub-rows).
    pub history_selected: usize,
    /// Scroll offset (top visible row) for the history pane.
    pub history_scroll: usize,
    /// Cached pane areas from last draw call — used for mouse click detection.
    /// This is populated at the start of each draw() call.
    pub cached_pane_areas: Option<crate::ui::PaneAreas>,
    /// Timestamp of the last left-click, for double-click detection.
    pub last_click_time: Option<Instant>,
    /// Position of the last left-click, for double-click detection.
    pub last_click_pos: Option<(u16, u16)>,
}

/// Maximum number of log lines retained.
const MAX_LOG_LINES: usize = 1000;

/// Number of rows to jump when using Fn-Up/Down (PageUp/PageDown).
const PAGE_STEP: usize = 10;

impl App {
    pub fn new() -> Self {
        let config = Config::load();
        let config_clone = config.clone();
        let interval = config.general.auto_fetch_interval();

        let state = State::load();
        let recent_repos = state
            .recent
            .iter()
            .map(|r| (r.path.clone(), r.name.clone()))
            .collect();

        App {
            repos: Vec::new(),
            selected: 0,
            should_quit: false,
            last_refreshed: None,
            mode: AppMode::Normal,
            file_explorer: None,
            recent_repos,
            config: config_clone,
            state,
            scanning: false,
            operations: HashMap::new(),
            log: Vec::new(),
            show_detail: false,
            show_log: false,
            table_offset: 0,
            spinner_tick: 0,
            focus: Focus::Repos,
            detail_selected: 0,
            detail_scroll: 0,
            log_offset: 0,
            log_follow: true,
            next_auto_fetch: Some(Instant::now() + interval),
            menu_items: Vec::new(),
            menu_selected: 0,
            branch_items: Vec::new(),
            branch_selected: 0,
            branch_input: String::new(),
            branch_to_delete: String::new(),
            theme_idx: 0,
            history: Vec::new(),
            show_history: false,
            history_repo_path: String::new(),
            history_filter: HistoryFilter::Full,
            history_selected: 0,
            history_scroll: 0,
            cached_pane_areas: None,
            last_click_time: None,
            last_click_pos: None,
        }
    }

    // ── Navigation ────────────────────────────────────────────────────────────

    /// Move selection down in the currently focused pane.
    pub fn next(&mut self) {
        match self.focus {
            Focus::Repos => {
                if !self.repos.is_empty() {
                    let last = self.repos.len() - 1;
                    if self.selected < last {
                        self.selected += 1;
                    }
                    self.detail_selected = 0;
                    self.detail_scroll = 0;
                }
            }
            Focus::Detail => {
                let n = self.selected_files().len();
                if n > 0 && self.detail_selected + 1 < n {
                    self.detail_selected += 1;
                }
            }
            Focus::Log => {
                // Scroll toward the tail (offset closer to 0).
                if self.log_offset > 0 {
                    self.log_offset -= 1;
                }
                // At offset 0 we're showing the tail — re-enable follow.
                if self.log_offset == 0 {
                    self.log_follow = true;
                }
            }
            Focus::History => {
                let n = self.history_row_count();
                if n > 0 && self.history_selected + 1 < n {
                    self.history_selected += 1;
                }
            }
        }
    }

    /// Move selection up in the currently focused pane.
    pub fn previous(&mut self) {
        match self.focus {
            Focus::Repos => {
                if !self.repos.is_empty() {
                    if self.selected > 0 {
                        self.selected -= 1;
                    }
                    self.detail_selected = 0;
                    self.detail_scroll = 0;
                }
            }
            Focus::Detail => {
                if self.detail_selected > 0 {
                    self.detail_selected -= 1;
                }
            }
            Focus::Log => {
                // Scroll away from the tail (increase offset = go back in history).
                // Cap at log.len() - 1 so we can't scroll past the first entry.
                let max_back = self.log.len().saturating_sub(1);
                if self.log_offset < max_back {
                    self.log_offset += 1;
                }
                self.log_follow = false;
            }
            Focus::History => {
                if self.history_selected > 0 {
                    self.history_selected -= 1;
                }
            }
        }
    }

    /// Cycle the keyboard focus to the next visible pane.
    /// Order is fixed: Repos -> Detail -> History -> Log (only shown panes are included).
    pub fn cycle_focus(&mut self) {
        let mut order: Vec<Focus> = vec![Focus::Repos];
        // Detail is shown before History and Log in the layout, so tab to it first if enabled
        if self.show_detail {
            order.push(Focus::Detail);
        }
        // History pane is rendered before Log
        if self.show_history {
            order.push(Focus::History);
        }
        // Log pane is last in the sequence
        if self.show_log {
            order.push(Focus::Log);
        }

        if order.len() < 2 {
            self.focus = Focus::Repos;
            return;
        }
        let idx = order.iter().position(|f| *f == self.focus).unwrap_or(0);
        self.focus = order[(idx + 1) % order.len()];
    }

    /// Cycle the keyboard focus to the previous visible pane.
    /// Reverse order of `cycle_focus`: Log -> History -> Detail -> Repos.
    pub fn cycle_focus_reverse(&mut self) {
        let mut order: Vec<Focus> = vec![Focus::Repos];
        if self.show_detail {
            order.push(Focus::Detail);
        }
        if self.show_history {
            order.push(Focus::History);
        }
        if self.show_log {
            order.push(Focus::Log);
        }

        if order.len() < 2 {
            self.focus = Focus::Repos;
            return;
        }
        let idx = order.iter().position(|f| *f == self.focus).unwrap_or(0);
        // Reverse: (idx + len - 1) % len
        self.focus = order[(idx + order.len() - 1) % order.len()];
    }

    /// Move down by `PAGE_STEP` rows in the currently focused pane.
    /// Clamps at the last item — does not wrap around.
    pub fn next_page(&mut self) {
        match self.focus {
            Focus::Repos => {
                if !self.repos.is_empty() {
                    let last = self.repos.len() - 1;
                    self.selected = (self.selected + PAGE_STEP).min(last);
                    self.detail_selected = 0;
                    self.detail_scroll = 0;
                }
            }
            Focus::Detail => {
                let n = self.selected_files().len();
                if n > 0 {
                    self.detail_selected = (self.detail_selected + PAGE_STEP).min(n - 1);
                }
            }
            Focus::Log => {
                // Page toward tail.
                self.log_offset = self.log_offset.saturating_sub(PAGE_STEP);
                if self.log_offset == 0 {
                    self.log_follow = true;
                }
            }
            Focus::History => {
                let n = self.history_row_count();
                if n > 0 {
                    self.history_selected = (self.history_selected + PAGE_STEP).min(n - 1);
                }
            }
        }
    }

    /// Move up by `PAGE_STEP` rows in the currently focused pane.
    /// Clamps at the first item — does not wrap around.
    pub fn previous_page(&mut self) {
        match self.focus {
            Focus::Repos => {
                self.selected = self.selected.saturating_sub(PAGE_STEP);
                self.detail_selected = 0;
                self.detail_scroll = 0;
            }
            Focus::Detail => {
                self.detail_selected = self.detail_selected.saturating_sub(PAGE_STEP);
            }
            Focus::Log => {
                // Page away from tail (back in history).
                let max_back = self.log.len().saturating_sub(1);
                self.log_offset = (self.log_offset + PAGE_STEP).min(max_back);
                self.log_follow = false;
            }
            Focus::History => {
                self.history_selected = self.history_selected.saturating_sub(PAGE_STEP);
            }
        }
    }

    // ── Refresh helpers ───────────────────────────────────────────────────────

    pub fn seconds_since_refresh(&self) -> Option<u64> {
        self.last_refreshed.map(|t| t.elapsed().as_secs())
    }

    /// Returns true if the automatic background fetch timer has fired.
    pub fn is_auto_fetch_due(&self) -> bool {
        self.next_auto_fetch
            .map(|t| t <= Instant::now())
            .unwrap_or(false)
    }

    /// Reset the automatic background fetch timer to now + the interval.
    pub fn reset_auto_fetch_timer(&mut self) {
        let interval = self.config.general.auto_fetch_interval();
        if interval.as_secs() > 0 {
            self.next_auto_fetch = Some(Instant::now() + interval);
        } else {
            self.next_auto_fetch = None;
        }
    }

    pub fn tracked_paths(&self) -> Vec<String> {
        self.state.repos.clone()
    }

    // ── File picker ───────────────────────────────────────────────────────────

    /// Open the file-picker popup, starting at $HOME (or cwd as fallback).
    pub fn enter_pick_mode(&mut self) {
        let start_dir = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());

        let mut explorer = match FileExplorer::new() {
            Ok(e) => e,
            Err(_) => return,
        };

        // Apply a theme that makes the selected row clearly visible:
        // dark-gray background + cyan foreground on the highlighted entry,
        // plus a "> " prefix so there is no ambiguity about what is selected.
        let t = self.theme();
        let explorer_theme = ratatui_explorer::Theme::default()
            .with_highlight_dir_style(
                ratatui::style::Style::default()
                    .fg(t.selection_fg)
                    .bg(t.selection_bg)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            )
            .with_highlight_symbol("> ");
        explorer.set_theme(explorer_theme);

        // Navigate to home dir
        let _ = explorer.set_cwd(start_dir);

        // Show only directories (not plain files — we're picking a repo root)
        let _ = explorer.set_filter_map(|f| if f.is_dir { Some(f) } else { None });

        self.file_explorer = Some(explorer);
        self.mode = AppMode::FilePicker;
    }

    /// Cancel the file picker and return to base mode.
    pub fn cancel_pick(&mut self) {
        self.file_explorer = None;
        self.restore_base_mode();
    }

    /// Return the path currently highlighted in the file explorer.
    pub fn picker_selected_path(&self) -> Option<String> {
        self.file_explorer
            .as_ref()
            .map(|e| e.current().path.to_string_lossy().to_string())
    }

    /// Return the CWD currently shown in the file explorer.
    #[allow(dead_code)]
    pub fn picker_cwd(&self) -> Option<String> {
        self.file_explorer
            .as_ref()
            .map(|e| e.cwd().to_string_lossy().to_string())
    }

    /// Validate and add the given path as a tracked repo.
    /// Returns `Ok(Some(path))` if newly added, `Ok(None)` if already tracked,
    /// `Err(msg)` on validation failure.
    pub fn add_repo_path(&mut self, path: &str) -> Result<Option<String>, String> {
        if !std::path::Path::new(path).is_dir() {
            return Err(format!("Not a directory: {path}"));
        }
        if git2::Repository::open(path).is_err() {
            return Err(format!("Not a git repository: {path}"));
        }

        let added = self.state.add_repo(path);
        if let Err(e) = self.state.save() {
            eprintln!("gitover: failed to save state: {e}");
        }

        self.file_explorer = None;
        self.restore_base_mode();

        if added {
            Ok(Some(path.to_string()))
        } else {
            Ok(None)
        }
    }

    // ── Remove repo ───────────────────────────────────────────────────────────

    /// Enter the confirmation dialog for removing the selected repo.
    /// No-op if the list is empty.
    pub fn request_remove_selected(&mut self) {
        if !self.repos.is_empty() {
            self.mode = AppMode::ConfirmRemove;
        }
    }

    /// Dismiss the confirmation dialog without removing anything.
    pub fn cancel_remove(&mut self) {
        self.restore_base_mode();
    }

    /// Remove the currently-selected repo from tracking and persist.
    /// Returns the removed path on success.
    pub fn remove_selected(&mut self) -> Option<String> {
        if self.repos.is_empty() {
            return None;
        }
        let path = self.repos[self.selected].path.clone();
        self.state.remove_repo(&path);
        if let Err(e) = self.state.save() {
            eprintln!("gitover: failed to save state: {e}");
        }
        self.operations.remove(&path);
        if self.selected > 0 && self.selected >= self.repos.len() - 1 {
            self.selected -= 1;
        }
        self.restore_base_mode();
        Some(path)
    }

    // ── Operations / Log / UX helpers ─────────────────────────────────────────

    /// Mark a repo as having an active git operation in progress.
    #[allow(dead_code)]
    pub fn begin_operation(&mut self, path: &str, op: RepoOperation) {
        self.operations.insert(path.to_string(), op);
    }

    /// Clear the active operation for a repo.
    #[allow(dead_code)]
    pub fn end_operation(&mut self, path: &str) {
        self.operations.remove(path);
    }

    /// Dismiss any transient popup and return to the appropriate base mode:
    /// History if the history pane is open, Normal otherwise.
    pub fn restore_base_mode(&mut self) {
        if self.show_history {
            self.mode = AppMode::History;
        } else {
            self.mode = AppMode::Normal;
        }
    }

    // ── Action menu ───────────────────────────────────────────────────────────

    /// Open the per-repo action menu for the selected repo.
    /// Builds the item list based on current repo state.
    pub fn open_action_menu(&mut self) {
        if self.repos.is_empty() {
            return;
        }
        let repo = &self.repos[self.selected];
        let has_upstream = repo.upstream.is_some();
        let has_error = repo.error.is_some();

        let mut items = Vec::new();
        if !has_error {
            items.push(MenuItem {
                label: "Fetch".into(),
                key: 'f',
            });
            items.push(MenuItem {
                label: "Pull".into(),
                key: 'p',
            });
            if has_upstream {
                items.push(MenuItem {
                    label: "Push".into(),
                    key: 'P',
                });
                items.push(MenuItem {
                    label: "Force Push".into(),
                    key: 'F',
                });
            }
            items.push(MenuItem {
                label: "Checkout branch".into(),
                key: 'c',
            });
            items.push(MenuItem {
                label: "Create new branch".into(),
                key: 'n',
            });
            items.push(MenuItem {
                label: "Delete branch".into(),
                key: 'x',
            });
            items.push(MenuItem {
                label: "Commit history".into(),
                key: 'h',
            });
            if let Some(upstream) = &repo.upstream {
                items.push(MenuItem {
                    label: format!("History: ahead of {}", upstream.branch),
                    key: 'u',
                });
                items.push(MenuItem {
                    label: format!("History: behind {}", upstream.branch),
                    key: 'U',
                });
            }
            if let Some(trunk) = &repo.trunk {
                items.push(MenuItem {
                    label: format!("History: ahead of {}", trunk.branch),
                    key: 't',
                });
                items.push(MenuItem {
                    label: format!("History: behind {}", trunk.branch),
                    key: 'T',
                });
            }
        }
        self.menu_items = items;
        self.menu_selected = 0;
        self.mode = AppMode::ActionMenu;
    }

    pub fn menu_next(&mut self) {
        if !self.menu_items.is_empty() {
            self.menu_selected = (self.menu_selected + 1) % self.menu_items.len();
        }
    }

    pub fn menu_previous(&mut self) {
        if !self.menu_items.is_empty() {
            if self.menu_selected == 0 {
                self.menu_selected = self.menu_items.len() - 1;
            } else {
                self.menu_selected -= 1;
            }
        }
    }

    pub fn close_menu(&mut self) {
        self.restore_base_mode();
    }

    // ── Branch select ─────────────────────────────────────────────────────────

    /// Open the branch-select popup for the selected repo.
    pub fn open_branch_select(&mut self) {
        if self.repos.is_empty() {
            return;
        }
        let repo = &self.repos[self.selected];
        let current = &repo.branch;
        let mut items: Vec<BranchItem> = repo
            .local_branches
            .iter()
            .filter(|b| b.as_str() != current)
            .map(|b| BranchItem {
                name: b.clone(),
                is_remote: false,
            })
            .collect();
        for rb in &repo.remote_only_branches {
            items.push(BranchItem {
                name: rb.clone(),
                is_remote: true,
            });
        }
        self.branch_items = items;
        self.branch_selected = 0;
        self.mode = AppMode::BranchSelect;
    }

    pub fn branch_select_next(&mut self) {
        if !self.branch_items.is_empty() {
            self.branch_selected = (self.branch_selected + 1) % self.branch_items.len();
        }
    }

    pub fn branch_select_previous(&mut self) {
        if !self.branch_items.is_empty() {
            if self.branch_selected == 0 {
                self.branch_selected = self.branch_items.len() - 1;
            } else {
                self.branch_selected -= 1;
            }
        }
    }

    /// Return the currently highlighted branch item for checkout, if any.
    pub fn selected_branch_item(&self) -> Option<&BranchItem> {
        self.branch_items.get(self.branch_selected)
    }

    pub fn close_branch_select(&mut self) {
        self.restore_base_mode();
    }

    // ── New branch input ──────────────────────────────────────────────────────

    pub fn open_new_branch_input(&mut self) {
        self.branch_input.clear();
        self.mode = AppMode::NewBranchInput;
    }

    /// Sanitise the current branch_input (replace spaces, strip invalid chars).
    pub fn sanitised_branch_name(&self) -> String {
        self.branch_input
            .trim()
            .replace(' ', "-")
            .chars()
            .filter(|c| c.is_alphanumeric() || matches!(c, '-' | '_' | '/' | '.'))
            .collect()
    }

    pub fn close_new_branch_input(&mut self) {
        self.restore_base_mode();
    }

    // ── Delete branch confirm ─────────────────────────────────────────────────

    /// Open the delete-branch flow: first show branch-select, then confirm.
    pub fn open_delete_branch_select(&mut self) {
        if self.repos.is_empty() {
            return;
        }
        let repo = &self.repos[self.selected];
        let current = &repo.branch;
        let items: Vec<BranchItem> = repo
            .local_branches
            .iter()
            .filter(|b| b.as_str() != current)
            .map(|b| BranchItem {
                name: b.clone(),
                is_remote: false,
            })
            .collect();
        self.branch_items = items;
        self.branch_selected = 0;
        self.mode = AppMode::ConfirmDeleteBranch;
    }

    pub fn confirm_force_push(&mut self) {
        self.mode = AppMode::ConfirmForcePush;
    }

    // ── Git History ───────────────────────────────────────────────────────────

    /// Open the history pane for the selected repo, loading fresh commit data.
    pub fn open_history(&mut self, filter: HistoryFilter) {
        if self.repos.is_empty() {
            return;
        }
        let repo = &self.repos[self.selected];
        if repo.error.is_some() {
            return;
        }
        let path = repo.path.clone();
        self.history = crate::git::get_commit_history(&path, &filter, 200).unwrap_or_default();
        self.history_repo_path = path;
        self.history_filter = filter;
        self.history_selected = 0;
        self.history_scroll = 0;
        self.show_history = true;
        self.focus = Focus::History;
        self.restore_base_mode();
    }

    /// Close the history pane and return to normal focus.
    pub fn close_history(&mut self) {
        self.show_history = false;
        if self.focus == Focus::History {
            self.focus = Focus::Repos;
        }
        self.restore_base_mode();
    }

    /// Reload history for the current selected repo if the history pane is open
    /// and the selected repo has changed.
    pub fn reload_history_if_open(&mut self) {
        if !self.show_history {
            return;
        }
        let current_path = match self.repos.get(self.selected) {
            Some(r) if r.error.is_none() => r.path.clone(),
            _ => return,
        };
        if current_path == self.history_repo_path {
            return;
        }
        let filter = self.history_filter.clone();
        self.history =
            crate::git::get_commit_history(&current_path, &filter, 200).unwrap_or_default();
        self.history_repo_path = current_path;
        self.history_selected = 0;
        self.history_scroll = 0;
    }

    /// Force-reload history if the pane is open and it belongs to `repo_path`.
    /// Called after a git operation completes on that repo.
    pub fn refresh_history_for_repo(&mut self, repo_path: &str) {
        if !self.show_history || self.history_repo_path != repo_path {
            return;
        }
        let filter = self.history_filter.clone();
        self.history = crate::git::get_commit_history(repo_path, &filter, 200).unwrap_or_default();
        self.history_selected = 0;
        self.history_scroll = 0;
    }
    /// Return the total number of visible rows in the history pane:
    /// one row per commit + one row per file delta within each commit.
    pub fn history_row_count(&self) -> usize {
        self.history.iter().map(|c| 1 + c.files.len()).sum()
    }

    /// Resolve a flat `row_index` into (commit_index, Option<file_index>).
    /// Returns None if row_index is out of bounds.
    pub fn history_row_at(&self, row_index: usize) -> Option<(usize, Option<usize>)> {
        let mut remaining = row_index;
        for (ci, commit) in self.history.iter().enumerate() {
            if remaining == 0 {
                return Some((ci, None));
            }
            remaining -= 1;
            if remaining < commit.files.len() {
                return Some((ci, Some(remaining)));
            }
            remaining -= commit.files.len();
        }
        None
    }

    /// Append a timestamped line to the output log.
    /// If `log_follow` is true (or the log panel is closed), the offset is
    /// advanced so the newest entry stays visible when the panel is open.
    pub fn log(&mut self, text: impl Into<String>) {
        let timestamp = current_hms();
        self.log.push(LogLine {
            timestamp,
            text: text.into(),
        });
        if self.log.len() > MAX_LOG_LINES {
            let drop = self.log.len() - MAX_LOG_LINES;
            self.log.drain(0..drop);
            // log_offset is lines-from-tail, so drain doesn't affect it.
        }
        // With lines-from-tail semantics, follow just means keeping offset at 0.
        // No adjustment needed — offset 0 always shows the current tail.
    }

    pub fn toggle_detail(&mut self) {
        self.show_detail = !self.show_detail;
        if self.show_detail {
            self.detail_selected = 0;
            self.detail_scroll = 0;
            self.focus = Focus::Detail;
        } else {
            if self.focus == Focus::Detail {
                self.focus = Focus::Repos;
            }
        }
    }

    pub fn toggle_log(&mut self) {
        self.show_log = !self.show_log;
        if self.show_log {
            // Re-enable follow and snap to tail (offset 0) when opened.
            self.log_follow = true;
            self.log_offset = 0;
            self.focus = Focus::Log;
        } else if self.focus == Focus::Log {
            self.focus = Focus::Repos;
        }
    }

    /// One-character spinner frame derived from `spinner_tick`.
    pub fn spinner_frame(&self) -> &'static str {
        const FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
        FRAMES[(self.spinner_tick as usize) % FRAMES.len()]
    }

    /// Return the per-repo operation for `path`, if any.
    pub fn repo_operation(&self, path: &str) -> Option<RepoOperation> {
        self.operations.get(path).copied()
    }

    /// Return the per-file changes of the currently-selected repo (for the
    /// detail panel). Empty when no repo is selected or the repo errored.
    pub fn selected_files(&self) -> &[crate::git::FileEntry] {
        match self.repos.get(self.selected) {
            Some(r) => &r.files,
            None => &[],
        }
    }

    /// Convenience: the kinds present in the selected repo (used to colour
    /// the detail header).
    #[allow(dead_code)]
    pub fn selected_file_kinds(&self) -> Vec<FileStatusKind> {
        let mut kinds: Vec<FileStatusKind> = Vec::new();
        for f in self.selected_files() {
            if !kinds.contains(&f.status) {
                kinds.push(f.status.clone());
            }
        }
        kinds
    }

    // ── Helpers ───────────────────────────────────────────────────────────────

    /// Return the currently active theme.
    pub fn theme(&self) -> &'static crate::theme::Theme {
        crate::theme::THEMES[self.theme_idx % crate::theme::THEMES.len()]
    }

    /// Advance to the next theme in the cycle.
    pub fn next_theme(&mut self) {
        self.theme_idx = (self.theme_idx + 1) % crate::theme::THEMES.len();
    }

    /// Sort the repo list by absolute path (case-insensitive).
    /// Call this after any operation that adds entries to self.repos.
    pub fn sort_repos(&mut self) {
        self.repos
            .sort_by(|a, b| a.path.to_lowercase().cmp(&b.path.to_lowercase()));
    }
}

/// Format the current local wall-clock time as `HH:MM:SS`.
fn current_hms() -> String {
    use chrono::Local;
    Local::now().format("%H:%M:%S").to_string()
}
