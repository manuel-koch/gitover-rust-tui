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
use crate::git::{BranchInfo, CommitEntry, FileStatusKind, RepoStatus};
use crate::state::State;
use ratatui::style::Style;
use ratatui_explorer::FileExplorer;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;
use tui_textarea::TextArea;

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
    Committing,
    Working,
}

impl RepoOperation {
    pub fn label(&self) -> &'static str {
        match self {
            RepoOperation::Scanning => "scanning",
            RepoOperation::Fetching => "fetching",
            RepoOperation::Pulling => "pulling",
            RepoOperation::Pushing => "pushing",
            RepoOperation::Rebasing => "rebasing",
            RepoOperation::Committing => "committing",
            RepoOperation::Working => "working",
        }
    }
}

/// Severity level for a log line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LogLevel {
    Debug,
    #[default]
    Info,
    Warn,
    Error,
}

impl LogLevel {
    pub fn label(self) -> &'static str {
        match self {
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
        }
    }
}

/// One line in the output log panel.
#[derive(Debug, Clone)]
pub struct LogLine {
    /// Wall-clock time the line was recorded, formatted as `HH:MM:SS`.
    pub timestamp: String,
    pub level: LogLevel,
    pub text: String,
}

impl LogLine {
    pub fn new(text: impl Into<String>) -> Self {
        Self::new_at(LogLevel::Info, text)
    }

    pub fn new_at(level: LogLevel, text: impl Into<String>) -> Self {
        Self {
            timestamp: current_hms(),
            level,
            text: text.into(),
        }
    }

    pub fn formatted(&self) -> String {
        format!(
            "[{} {:>5}] {}",
            self.timestamp,
            self.level.label(),
            self.text
        )
    }
}

/// Which pane currently has keyboard focus. Tab cycles through the visible
/// panes (Repos is always visible; FileStatus/Log only when their panel is open).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Repos,
    Branches,
    FileStatus,
    Log,
    History,
    Details,
}

/// Which source last drove the content of the Details panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetailsSource {
    /// A file selected in the File Status pane.
    FileStatus,
    /// A file row selected inside a commit in the History pane.
    HistoryFile,
    /// A commit row selected in the History pane.
    HistoryCommit,
}

/// What the Details pane is currently displaying.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetailsMode {
    /// Showing a file diff.
    Diff,
    /// Showing commit details (a commit row is selected in the History pane).
    Commit,
    /// No relevant selection — showing the placeholder prompt.
    Empty,
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
    /// Text-area popup for composing a commit message (new commit or amend).
    CommitMessageInput,
    /// Confirmation dialog for force-push of HEAD.
    ConfirmForcePush,
    /// Confirmation dialog for force-push of a specific branch from the Branches pane.
    ConfirmForcePushBranch,
    /// Simple yes/no confirmation before deleting the branch selected in the Branches pane.
    ConfirmDeleteLocalBranch,
    /// Commit history pane (h key).
    History,
    /// Per-commit action menu (Enter on a commit header row in the History pane).
    HistoryActionMenu,
    /// Log action menu (Enter on Output Log pane).
    LogActionMenu,
    /// Per-file action menu (Enter or double-click on a file in the File Status pane).
    FileActionMenu,
    /// Per-branch action menu (Enter on a branch in the Branches pane).
    BranchActionMenu,
    /// Transient popup message that auto-dismisses after a timeout.
    PopupMessage,
    /// Full-screen keybinding reference overlay (toggled with '?').
    HelpOverlay,
    /// Text input for naming (create or rename) a repository section.
    SectionNameInput,
    /// Confirmation dialog for removing a section.
    ConfirmRemoveSection,
    /// Selection list for choosing a target section to move a repo into.
    SectionSelect,
}

/// One row in the Repositories pane visible list.
///
/// `app.selected` is an index into the `visible_rows()` output.
/// Named-section title rows appear before their repos; the default section has no title row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisibleRow {
    /// A named section title row; the value is the index into `state.sections` (always >= 1).
    SectionTitle(usize),
    /// A repository row; the value is the index into `app.repos`.
    Repo(usize),
}

/// One entry in the action menu.
#[derive(Debug, Clone)]
pub struct MenuItem {
    pub label: String,
    pub key: char,
    /// When true this item is a visual separator row and cannot be activated.
    pub is_separator: bool,
    /// Raw (un-interpolated) shell command + background flag for custom repo commands from config.
    pub repo_cmd: Option<(String, bool)>,
}

impl MenuItem {
    pub fn item(label: impl Into<String>, key: char) -> Self {
        Self {
            label: label.into(),
            key,
            is_separator: false,
            repo_cmd: None,
        }
    }
    pub fn separator() -> Self {
        Self {
            label: String::new(),
            key: '\0',
            is_separator: true,
            repo_cmd: None,
        }
    }
    pub fn repo_command(
        label: impl Into<String>,
        key: char,
        cmd: String,
        background: bool,
    ) -> Self {
        Self {
            label: label.into(),
            key,
            is_separator: false,
            repo_cmd: Some((cmd, background)),
        }
    }
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
    /// Loaded application configuration.
    #[allow(dead_code)]
    pub config: Config,
    /// Persisted state (repo list, pane visibility).
    pub state: State,

    // ── UX Polish ─────────────────────────────────────────────────────────────
    /// True while the initial/global repo scan is running.
    pub scanning: bool,
    /// Per-repo active operation (busy indicator + activity column).
    pub operations: HashMap<String, RepoOperation>,
    /// Timestamped log lines from git command output.
    pub log: Vec<LogLine>,
    /// Whether the File Status pane is shown.
    pub show_file_status: bool,
    /// Whether the output log panel is shown.
    pub show_log: bool,
    /// Top row of the repo table viewport (for scrolling).
    pub table_offset: usize,
    /// Spinner animation tick — incremented on each event-loop tick.
    pub spinner_tick: u64,
    /// Currently focused pane (Tab cycles through visible panes).
    pub focus: Focus,
    /// Selected row inside the File Status pane (per-file list).
    pub file_status_selected: usize,
    /// Scroll offset (top row) of the File Status pane.
    pub file_status_scroll: usize,
    /// File path to restore as selected after the next file-op refresh completes.
    /// Set before launching a stage/unstage op; cleared by `reselect_file_after_refresh`.
    pub reselect_file_path: Option<String>,
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
    /// Scroll offset (top visible row) for the action menu popup.
    pub menu_scroll: usize,
    /// Branches shown in the branch-select popup.
    pub branch_items: Vec<BranchItem>,
    /// Currently highlighted branch-select item.
    pub branch_selected: usize,
    /// Text being typed in the new-branch-name input.
    pub branch_input: String,
    /// Base branch to branch off from when in NewBranchInput mode.
    /// Empty string means branch from HEAD (repos-menu flow).
    pub branch_input_base: String,
    /// Branch name staged for deletion (shown in confirm dialog).
    pub branch_to_delete: String,
    /// Branch name staged for force-push from the Branches pane (shown in confirm dialog).
    pub branch_to_force_push: String,
    /// Commit message being composed in the CommitMessageInput popup.
    pub commit_textarea: TextArea<'static>,
    /// When true the CommitMessageInput popup runs `git commit --amend`.
    pub commit_is_amend: bool,
    /// Number of files changed in the HEAD commit; populated when opening the amend dialog.
    pub commit_head_file_count: usize,

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
    /// Active popup message to display ( PopupMessage mode ).
    pub popup_message: Option<String>,
    /// Timestamp when the popup was shown (for auto-dismissal).
    pub popup_show_time: Option<Instant>,
    /// Transient status text shown in the header for 2 s after a triggered action.
    pub header_flash: Option<(String, Instant)>,

    // ── Branches pane ─────────────────────────────────────────────────────────
    /// Whether the Branches pane is visible (replaces Repositories pane).
    pub show_branches: bool,
    /// Branch list for the Branches pane (current repo's local branches).
    pub branch_info_list: Vec<BranchInfo>,
    /// Currently highlighted row in the Branches pane.
    pub branches_pane_selected: usize,
    /// Scroll offset (top visible row) for the Branches pane.
    pub branches_pane_scroll: usize,
    /// True when the History pane is being driven by the Branches pane selection.
    pub branches_history_active: bool,

    // ── Details pane ──────────────────────────────────────────────────────────
    /// Whether the Details pane is visible.
    pub show_details: bool,
    /// What the Details pane is currently displaying.
    pub details_mode: DetailsMode,
    /// Raw patch-format text shown when details_mode == Diff.
    pub details_content: String,
    /// Scroll offset (lines from top) for the Details pane.
    pub details_scroll: usize,
    /// Which pane last provided the diff context.
    pub details_source: DetailsSource,

    // ── Help overlay ──────────────────────────────────────────────────────────
    /// Scroll offset (lines from top) for the help keybindings overlay.
    pub help_overlay_scroll: usize,
    /// Maximum valid scroll offset for the help overlay (set each draw frame).
    pub help_overlay_max_scroll: usize,
    /// Bounding rect of the help overlay popup (set each draw frame).
    pub help_overlay_area: Option<ratatui::layout::Rect>,

    // ── Section management ────────────────────────────────────────────────────
    /// Text being typed in the section name input (create or rename).
    pub section_input: String,
    /// When true the section name input is for a new section; when false it renames the current.
    pub section_input_is_create: bool,
    /// The `sections` index of the section being renamed (only valid when `!section_input_is_create`).
    pub section_input_target_idx: usize,
    /// The `sections` index of the section staged for removal.
    pub section_to_remove_idx: Option<usize>,
    /// Items offered in the section-select popup: `(sections_index, display_name)`.
    pub section_select_items: Vec<(usize, String)>,
    /// Currently highlighted item in the section-select popup.
    pub section_select_selected: usize,

    // ── Pane resize ───────────────────────────────────────────────────────────
    /// User-overridden height for the Repositories pane (drag-to-resize).
    /// None = automatic equal-distribution layout. Not saved to state.
    pub repos_height_override: Option<u16>,
    /// True while a mouse drag on the repos divider is in progress.
    pub dragging_repos_divider: bool,
    /// True when the mouse is hovering over the repos divider row.
    pub hover_repos_divider: bool,
    /// User-overridden width for the Details pane (drag-to-resize).
    /// None = 50% of terminal width. Not saved to state.
    pub details_width_override: Option<u16>,
    /// True while a mouse drag on the details vertical divider is in progress.
    pub dragging_details_divider: bool,
    /// True when the mouse is hovering over the details vertical divider column.
    pub hover_details_divider: bool,
}

/// Maximum number of log lines retained.
const MAX_LOG_LINES: usize = 1000;

/// Number of rows to jump when using Fn-Up/Down (PageUp/PageDown).
const PAGE_STEP: usize = 10;

/// Maximum number of commits loaded into the history pane.
const HISTORY_COMMIT_LIMIT: usize = 200;

impl App {
    pub fn new() -> Self {
        Self::new_with_overrides(None, None)
    }

    /// Create the app, optionally overriding the config and/or state file paths
    /// (from `--config` / `--state` CLI flags).
    pub fn new_with_overrides(config_path: Option<PathBuf>, state_path: Option<PathBuf>) -> Self {
        let config = match config_path {
            Some(p) => Config::load_from(&p),
            None => Config::load(),
        };
        let config_clone = config.clone();
        let interval = config.general.auto_fetch_interval();

        let mut state = match state_path {
            Some(p) => State::load_from_path(p),
            None => State::load(),
        };
        state.sort_all_section_repos(config_clone.general.case_sensitive_path_sorting);
        let show_file_status = state.show_file_status;
        let show_log = state.show_log;
        let show_history = state.show_history;
        let show_details = state.show_details;

        App {
            repos: Vec::new(),
            selected: 0,
            should_quit: false,
            last_refreshed: None,
            mode: AppMode::Normal,
            file_explorer: None,
            config: config_clone,
            state,
            scanning: false,
            operations: HashMap::new(),
            log: Vec::new(),
            show_file_status,
            show_log,
            table_offset: 0,
            spinner_tick: 0,
            focus: Focus::Repos,
            file_status_selected: 0,
            file_status_scroll: 0,
            reselect_file_path: None,
            log_offset: 0,
            log_follow: true,
            next_auto_fetch: Some(Instant::now() + interval),
            menu_items: Vec::new(),
            menu_selected: 0,
            menu_scroll: 0,
            branch_items: Vec::new(),
            branch_selected: 0,
            branch_input: String::new(),
            branch_input_base: String::new(),
            branch_to_delete: String::new(),
            branch_to_force_push: String::new(),
            commit_textarea: TextArea::default(),
            commit_is_amend: false,
            commit_head_file_count: 0,
            theme_idx: 0,
            history: Vec::new(),
            show_history,
            history_repo_path: String::new(),
            history_filter: HistoryFilter::Full,
            history_selected: 0,
            history_scroll: 0,
            cached_pane_areas: None,
            last_click_time: None,
            last_click_pos: None,
            popup_message: None,
            popup_show_time: None,
            header_flash: None,
            show_branches: false,
            branch_info_list: Vec::new(),
            branches_pane_selected: 0,
            branches_pane_scroll: 0,
            branches_history_active: false,
            show_details,
            details_mode: DetailsMode::Empty,
            details_content: String::new(),
            details_scroll: 0,
            details_source: DetailsSource::FileStatus,
            help_overlay_scroll: 0,
            help_overlay_max_scroll: usize::MAX,
            help_overlay_area: None,
            repos_height_override: None,
            dragging_repos_divider: false,
            hover_repos_divider: false,
            details_width_override: None,
            dragging_details_divider: false,
            hover_details_divider: false,
            section_input: String::new(),
            section_input_is_create: true,
            section_input_target_idx: 0,
            section_to_remove_idx: None,
            section_select_items: Vec::new(),
            section_select_selected: 0,
        }
    }

    // ── Navigation ────────────────────────────────────────────────────────────

    /// Move selection down in the currently focused pane.
    pub fn next(&mut self) {
        match self.focus {
            Focus::Repos => {
                let visible_count = self.visible_rows().len();
                if visible_count > 0 {
                    let last = visible_count - 1;
                    if self.selected < last {
                        self.selected += 1;
                    }
                    self.file_status_selected = 0;
                    self.file_status_scroll = 0;
                }
            }
            Focus::Branches => {
                let n = self.branch_info_list.len();
                if n > 0 && self.branches_pane_selected + 1 < n {
                    self.branches_pane_selected += 1;
                }
            }
            Focus::FileStatus => {
                let n = self.selected_files().len();
                if n > 0 && self.file_status_selected + 1 < n {
                    self.file_status_selected += 1;
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
            Focus::Details => {
                let n = self.details_line_count();
                if n > 0 && self.details_scroll + 1 < n {
                    self.details_scroll += 1;
                }
            }
        }
    }

    /// Move selection up in the currently focused pane.
    pub fn previous(&mut self) {
        match self.focus {
            Focus::Repos => {
                if !self.visible_rows().is_empty() {
                    if self.selected > 0 {
                        self.selected -= 1;
                    }
                    self.file_status_selected = 0;
                    self.file_status_scroll = 0;
                }
            }
            Focus::Branches => {
                if self.branches_pane_selected > 0 {
                    self.branches_pane_selected -= 1;
                }
            }
            Focus::FileStatus => {
                if self.file_status_selected > 0 {
                    self.file_status_selected -= 1;
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
            Focus::Details => {
                if self.details_scroll > 0 {
                    self.details_scroll -= 1;
                }
            }
        }
    }

    /// Build the ordered list of focusable panes based on what is currently visible.
    /// Order: Repos/Branches → FileStatus → History → Diff → Log.
    fn focus_order(&self) -> Vec<Focus> {
        let mut order: Vec<Focus> = if self.show_branches {
            vec![Focus::Branches]
        } else {
            vec![Focus::Repos]
        };
        if self.show_file_status && !self.show_branches {
            order.push(Focus::FileStatus);
        }
        if self.show_history {
            order.push(Focus::History);
        }
        if self.show_details && (self.show_file_status || self.show_history) {
            order.push(Focus::Details);
        }
        if self.show_log {
            order.push(Focus::Log);
        }
        order
    }

    /// Cycle the keyboard focus to the next visible pane.
    pub fn cycle_focus(&mut self) {
        let order = self.focus_order();
        if order.len() < 2 {
            self.focus = Focus::Repos;
            return;
        }
        let idx = order.iter().position(|f| *f == self.focus).unwrap_or(0);
        self.focus = order[(idx + 1) % order.len()];
    }

    /// Cycle the keyboard focus to the previous visible pane.
    pub fn cycle_focus_reverse(&mut self) {
        let order = self.focus_order();
        if order.len() < 2 {
            self.focus = Focus::Repos;
            return;
        }
        let idx = order.iter().position(|f| *f == self.focus).unwrap_or(0);
        self.focus = order[(idx + order.len() - 1) % order.len()];
    }

    /// Move down by `PAGE_STEP` rows in the currently focused pane.
    /// Clamps at the last item — does not wrap around.
    pub fn next_page(&mut self) {
        match self.focus {
            Focus::Repos => {
                let visible_count = self.visible_rows().len();
                if visible_count > 0 {
                    self.selected = (self.selected + PAGE_STEP).min(visible_count - 1);
                    self.file_status_selected = 0;
                    self.file_status_scroll = 0;
                }
            }
            Focus::Branches => {
                let n = self.branch_info_list.len();
                if n > 0 {
                    self.branches_pane_selected =
                        (self.branches_pane_selected + PAGE_STEP).min(n - 1);
                }
            }
            Focus::FileStatus => {
                let n = self.selected_files().len();
                if n > 0 {
                    self.file_status_selected = (self.file_status_selected + PAGE_STEP).min(n - 1);
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
            Focus::Details => {
                let n = self.details_line_count();
                if n > 0 {
                    self.details_scroll = (self.details_scroll + PAGE_STEP).min(n - 1);
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
                self.file_status_selected = 0;
                self.file_status_scroll = 0;
            }
            Focus::Branches => {
                self.branches_pane_selected = self.branches_pane_selected.saturating_sub(PAGE_STEP);
            }
            Focus::FileStatus => {
                self.file_status_selected = self.file_status_selected.saturating_sub(PAGE_STEP);
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
            Focus::Details => {
                self.details_scroll = self.details_scroll.saturating_sub(PAGE_STEP);
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
        self.state.all_repos_flat()
    }

    // ── File picker ───────────────────────────────────────────────────────────

    /// Open the file-picker popup, starting at $HOME (or cwd as fallback).
    pub fn enter_pick_mode(&mut self) {
        let start_dir = std::env::current_dir().unwrap_or_else(|_| {
            std::path::PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".to_string()))
        });

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
        match git2::Repository::open(path) {
            Err(_) => return Err(format!("Not a git repository: {path}")),
            Ok(repo) if repo.is_bare() => {
                return Err(format!("Bare repositories are not supported: {path}"))
            }
            Ok(_) => {}
        }

        let target_section_idx = self.current_section_idx();
        let added = self.state.add_repo_to_section(path, target_section_idx);
        if let Err(e) = self.state.save() {
            eprintln!("gitover: failed to save state: {e}");
        }

        if added {
            Ok(Some(path.to_string()))
        } else {
            Ok(None)
        }
    }

    // ── Remove repo ───────────────────────────────────────────────────────────

    /// Enter the confirmation dialog for removing the selected repo.
    /// No-op when a section title row (not a repo row) is selected.
    pub fn request_remove_selected(&mut self) {
        if self.selected_repo_idx().is_some() {
            self.mode = AppMode::ConfirmRemove;
        }
    }

    /// Dismiss the confirmation dialog without removing anything.
    pub fn cancel_remove(&mut self) {
        self.restore_base_mode();
    }

    /// Remove the currently-selected repo from tracking and persist.
    /// Returns the removed path on success.  No-op when a section title is selected.
    pub fn remove_selected(&mut self) -> Option<String> {
        let repo_idx = self.selected_repo_idx()?;
        let path = self.repos[repo_idx].path.clone();
        self.state.remove_repo(&path);
        if let Err(e) = self.state.save() {
            eprintln!("gitover: failed to save state: {e}");
        }
        self.operations.remove(&path);
        self.repos.remove(repo_idx);
        let visible_count = self.visible_rows().len();
        if visible_count == 0 {
            self.selected = 0;
        } else if self.selected >= visible_count {
            self.selected = visible_count - 1;
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

    /// Open the action menu for the currently selected row.
    /// Shows section actions when a section title row is selected, repo actions otherwise.
    pub fn open_repo_action_menu(&mut self) {
        match self.visible_rows().get(self.selected).copied() {
            Some(VisibleRow::SectionTitle(section_idx)) => {
                self.open_section_action_menu(section_idx);
            }
            Some(VisibleRow::Repo(_)) => {
                self.open_repo_row_action_menu();
            }
            None => {}
        }
    }

    fn open_section_action_menu(&mut self, section_idx: usize) {
        let mut items = Vec::new();
        items.push(MenuItem::item("Create Repo Section", 'N'));
        if section_idx > 0 {
            items.push(MenuItem::item("Rename Repo Section", 'R'));
            items.push(MenuItem::item("Remove Repo Section", 'X'));
        }
        self.open_menu(items, AppMode::ActionMenu);
    }

    fn open_repo_row_action_menu(&mut self) {
        let repo_idx = match self.selected_repo_idx() {
            Some(idx) => idx,
            None => return,
        };
        let repo = &self.repos[repo_idx];
        let has_error = repo.error.is_some();

        let mut items = Vec::new();
        if !has_error {
            items.push(MenuItem::item("Fetch", 'f'));
            items.push(MenuItem::item("Pull Branch", 'p'));
            items.push(MenuItem::item("Push Branch", 'P'));
            items.push(MenuItem::item("Force Push Branch", 'F'));
            items.push(MenuItem::item("Checkout Branch", 'c'));
            items.push(MenuItem::item("Create Branch", 'n'));

            items.push(MenuItem::item("Commit History", 'h'));
            if let Some(upstream) = &repo.upstream {
                if upstream.ahead > 0 {
                    items.push(MenuItem::item(
                        format!("History: Ahead of {}", upstream.branch),
                        'u',
                    ));
                }
                if upstream.behind > 0 {
                    items.push(MenuItem::item(
                        format!("History: Behind {}", upstream.branch),
                        'U',
                    ));
                }
            }
            // Skip trunk entries when trunk branch == upstream branch (duplicates).
            let upstream_branch = repo
                .upstream
                .as_ref()
                .map(|u| u.branch.as_str())
                .unwrap_or("");
            if let Some(trunk) = &repo.trunk {
                if trunk.branch != upstream_branch {
                    if trunk.ahead > 0 {
                        items.push(MenuItem::item(
                            format!("History: Ahead of {}", trunk.branch),
                            't',
                        ));
                    }
                    if trunk.behind > 0 {
                        items.push(MenuItem::item(
                            format!("History: Behind {}", trunk.branch),
                            'T',
                        ));
                    }
                }
            }
        }

        // Section management items for repo rows
        items.push(MenuItem::separator());
        items.push(MenuItem::item("Create Repo Section", 'N'));
        if self.state.has_named_sections() {
            items.push(MenuItem::item("Move to Repo Section", 'M'));
        }

        // Custom repo commands from config
        if !has_error {
            let cmds = self.config.repo_commands.clone();
            if !cmds.is_empty() {
                items.push(MenuItem::separator());
                for (i, rc) in cmds.iter().enumerate() {
                    let key = char::from_digit((i + 1) as u32 % 10, 10).unwrap_or('\0');
                    items.push(MenuItem::repo_command(
                        rc.name.clone(),
                        key,
                        rc.cmd.clone(),
                        rc.background,
                    ));
                }
            }
        }
        self.open_menu(items, AppMode::ActionMenu);
    }

    fn open_menu(&mut self, items: Vec<MenuItem>, mode: AppMode) {
        self.menu_items = items;
        self.menu_selected = 0;
        self.menu_scroll = 0;
        self.mode = mode;
    }

    pub fn menu_next(&mut self) {
        let n = self.menu_items.len();
        if n == 0 {
            return;
        }
        let mut idx = self.menu_selected + 1;
        while idx < n {
            if !self.menu_items[idx].is_separator {
                self.menu_selected = idx;
                return;
            }
            idx += 1;
        }
    }

    pub fn menu_previous(&mut self) {
        let n = self.menu_items.len();
        if n == 0 {
            return;
        }
        let mut idx = self.menu_selected;
        while idx > 0 {
            idx -= 1;
            if !self.menu_items[idx].is_separator {
                self.menu_selected = idx;
                return;
            }
        }
    }

    pub fn menu_next_page(&mut self) {
        for _ in 0..PAGE_STEP {
            self.menu_next();
        }
    }

    pub fn menu_previous_page(&mut self) {
        for _ in 0..PAGE_STEP {
            self.menu_previous();
        }
    }

    pub fn close_menu(&mut self) {
        self.restore_base_mode();
    }

    // ── File action menu ──────────────────────────────────────────────────────

    /// Return the currently selected file entry in the File Status pane, if any.
    pub fn selected_file_entry(&self) -> Option<&crate::git::FileEntry> {
        self.selected_files().get(self.file_status_selected)
    }

    /// Open the per-file action menu for the currently selected file.
    /// Menu items are built based on the file's status.
    pub fn open_file_action_menu(&mut self) {
        let entry = match self
            .selected_files()
            .get(self.file_status_selected)
            .cloned()
        {
            Some(e) => e,
            None => return,
        };

        let mut items = Vec::new();
        match entry.status {
            FileStatusKind::Staged => {
                items.push(MenuItem::item("Commit", 'c'));
                items.push(MenuItem::item("Amend Commit", 'a'));
                items.push(MenuItem::item("Unstage File", 'u'));
                items.push(MenuItem::item("Save as Patch and Revert File", 'p'));
            }
            FileStatusKind::Modified => {
                items.push(MenuItem::item("Stage File", 's'));
                items.push(MenuItem::item("Revert File", 'r'));
                items.push(MenuItem::item("Save as Patch and Revert File", 'p'));
            }
            FileStatusKind::Deleted => {
                items.push(MenuItem::item("Stage Deletion", 's'));
                items.push(MenuItem::item("Revert File", 'r'));
                items.push(MenuItem::item("Save as Patch and Revert File", 'p'));
            }
            FileStatusKind::Conflict => {
                items.push(MenuItem::item("Revert File", 'r'));
            }
            FileStatusKind::Untracked => {
                items.push(MenuItem::item("Stage File", 's'));
                items.push(MenuItem::item("Discard File", 'd'));
            }
        }
        if entry.path.ends_with(".patch") {
            items.push(MenuItem::item("Apply Patch", 'P'));
        }

        self.open_menu(items, AppMode::FileActionMenu);
    }

    /// Open the log action menu for the Output Log pane.
    pub fn open_log_action_menu(&mut self) {
        self.open_menu(
            vec![
                MenuItem::item("Copy Log Output", 'c'),
                MenuItem::item("Clear Log", 'x'),
            ],
            AppMode::LogActionMenu,
        );
    }

    /// Open the per-commit action menu for the currently selected history row.
    /// Only opens when:
    /// - the history shows the full current-branch log (HistoryFilter::Full), and
    /// - the selected row is a commit header row (not a file sub-row), and
    /// - that commit is HEAD (index 0 in the Full filter).
    pub fn open_history_action_menu(&mut self) {
        let Some((commit_index, None)) = self.history_row_at(self.history_selected) else {
            return;
        };
        if self.history_filter != HistoryFilter::Full || commit_index != 0 {
            return;
        }
        self.open_menu(
            vec![MenuItem::item("Undo Commit", 'u')],
            AppMode::HistoryActionMenu,
        );
    }

    /// Clear all log lines and reset the scroll position.
    pub fn clear_log(&mut self) {
        self.log.clear();
        self.log_offset = 0;
        self.log_follow = true;
        self.set_header_flash("Log cleared");
    }

    /// Copy the entire Output Log content to the system clipboard.
    /// Also shows a transient popup notification.
    pub fn copy_log_to_clipboard(&mut self) {
        let text: String = self
            .log
            .iter()
            .map(|l| l.formatted())
            .collect::<Vec<_>>()
            .join("\n");
        if let Ok(mut clipboard) = arboard::Clipboard::new() {
            let _ = clipboard.set_text(text);
        }
        // Show popup notification
        self.popup_message = Some("Log output copied to clipboard!".into());
        self.popup_show_time = Some(Instant::now());
        self.mode = AppMode::PopupMessage;
    }

    /// Check if the popup message should auto-dismiss (2 seconds timeout).
    /// Show a short status text in the header for 2 seconds.
    pub fn set_header_flash(&mut self, msg: impl Into<String>) {
        self.header_flash = Some((msg.into(), Instant::now()));
    }

    /// Clear the header flash once its 2-second lifetime has passed.
    pub fn tick_header_flash(&mut self) {
        if let Some((_, t)) = self.header_flash {
            if t.elapsed().as_secs() >= 2 {
                self.header_flash = None;
            }
        }
    }

    pub fn check_popup_timeout(&mut self) {
        if let (Some(show_time), Some(_msg)) = (self.popup_show_time, &self.popup_message) {
            if show_time.elapsed().as_secs() >= 2 {
                self.popup_message = None;
                self.popup_show_time = None;
                self.restore_base_mode();
            }
        }
    }

    // ── Branch select ─────────────────────────────────────────────────────────

    /// Open the branch-select popup for the selected repo.
    pub fn open_branch_select(&mut self) {
        let path = match self.selected_repo_idx().and_then(|i| self.repos.get(i)) {
            Some(r) => r.path.clone(),
            None => return,
        };
        let branches = crate::git::get_branches_with_ahead_behind(&path).unwrap_or_default();
        let items: Vec<BranchItem> = branches
            .into_iter()
            .filter(|b| !b.is_current)
            .map(|b| BranchItem {
                name: b.name,
                is_remote: b.is_remote_only,
            })
            .collect();
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
        self.branch_input_base.clear();
        self.mode = AppMode::NewBranchInput;
    }

    /// Open the new-branch name input, branching off `base` instead of HEAD.
    pub fn open_new_branch_from_input(&mut self, base: String) {
        self.branch_input.clear();
        self.branch_input_base = base;
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

    pub fn confirm_force_push(&mut self) {
        self.mode = AppMode::ConfirmForcePush;
    }

    pub fn confirm_force_push_branch(&mut self, name: String) {
        self.branch_to_force_push = name;
        self.mode = AppMode::ConfirmForcePushBranch;
    }

    /// Number of staged files in the currently selected repo.
    pub fn staged_file_count(&self) -> usize {
        self.selected_repo_idx()
            .and_then(|i| self.repos.get(i))
            .map(|r| r.staged)
            .unwrap_or(0)
    }

    /// Open the commit message dialog for a fresh commit.
    pub fn open_commit_input(&mut self) {
        self.commit_textarea = Self::new_commit_textarea(self.theme().input_text);
        self.commit_is_amend = false;
        self.mode = AppMode::CommitMessageInput;
    }

    /// Open the commit message dialog pre-filled with the HEAD commit message for amending.
    pub fn open_amend_input(&mut self) {
        let path = match self.selected_repo_idx().and_then(|i| self.repos.get(i)) {
            Some(r) => r.path.clone(),
            None => return,
        };
        let message = crate::git::get_head_commit_message(&path)
            .map(|m| m.trim_end().to_string())
            .unwrap_or_default();
        let input_text_color = self.theme().input_text;
        let lines: Vec<String> = message.lines().map(|l| l.to_string()).collect();
        let mut textarea = if lines.is_empty() {
            Self::new_commit_textarea(input_text_color)
        } else {
            let mut ta = TextArea::new(lines);
            Self::style_commit_textarea(&mut ta, input_text_color);
            ta
        };
        textarea.move_cursor(tui_textarea::CursorMove::Bottom);
        textarea.move_cursor(tui_textarea::CursorMove::End);
        self.commit_textarea = textarea;
        self.commit_head_file_count = crate::git::get_head_commit_file_count(&path);
        self.commit_is_amend = true;
        self.mode = AppMode::CommitMessageInput;
    }

    pub fn new_commit_textarea(input_text_color: ratatui::style::Color) -> TextArea<'static> {
        let mut textarea = TextArea::default();
        Self::style_commit_textarea(&mut textarea, input_text_color);
        textarea
    }

    fn style_commit_textarea(
        textarea: &mut TextArea<'static>,
        input_text_color: ratatui::style::Color,
    ) {
        textarea.set_style(Style::default().fg(input_text_color));
        textarea.set_cursor_line_style(Style::default());
    }

    /// Extract the current commit message text from the textarea.
    pub fn commit_message_text(&self) -> String {
        self.commit_textarea.lines().join("\n")
    }

    // ── Git History ───────────────────────────────────────────────────────────

    /// Load commit history into the history pane, resetting scroll and selection.
    fn load_history(&mut self, path: String, filter: HistoryFilter) {
        let case_sensitive_sort = self.config.general.case_sensitive_path_sorting;
        self.history = crate::git::get_commit_history(
            &path,
            &filter,
            HISTORY_COMMIT_LIMIT,
            case_sensitive_sort,
        )
        .unwrap_or_default();
        self.history_repo_path = path;
        self.history_filter = filter;
        self.history_selected = 0;
        self.history_scroll = 0;
    }

    /// Open the history pane for the selected repo, loading fresh commit data.
    pub fn open_history(&mut self, filter: HistoryFilter) {
        let path = match self.selected_repo_idx().and_then(|i| self.repos.get(i)) {
            Some(r) if r.error.is_none() => r.path.clone(),
            _ => return,
        };
        self.load_history(path, filter);
        self.show_history = true;
        self.focus = Focus::History;
        self.save_pane_state();
        self.restore_base_mode();
    }

    /// Close the history pane and return to normal focus.
    pub fn close_history(&mut self) {
        self.show_history = false;
        if self.focus == Focus::History {
            self.focus = Focus::Repos;
        }
        self.save_pane_state();
        self.restore_base_mode();
    }

    /// Reload history for the current selected repo if the history pane is open.
    pub fn reload_history_if_open(&mut self, force: bool) {
        if !self.show_history {
            return;
        }
        let repo_idx = match self.selected_repo_idx() {
            Some(i) => i,
            None => {
                // Section title selected — clear stale history so the pane shows a placeholder.
                self.history.clear();
                self.history_repo_path.clear();
                self.history_selected = 0;
                self.history_scroll = 0;
                return;
            }
        };
        let current_path = match self.repos.get(repo_idx) {
            Some(r) if r.error.is_none() => r.path.clone(),
            _ => return,
        };
        if !force && current_path == self.history_repo_path {
            return;
        }
        if force && current_path != self.history_repo_path {
            return;
        }
        if self.show_branches && self.branches_history_active {
            self.reload_history_from_branches();
            return;
        }
        // For AheadOf/BehindOf filters, re-resolve the ref against the new repo's
        // own trunk/upstream names in case the stored ref belongs to a different
        // remote (e.g. "origin/master" → "origin/main"). Candidates are tried in
        // order — stored ref, trunk ref, upstream ref — falling back to Full.
        let filter = self.history_filter.clone();
        let stored_ref = match &filter {
            HistoryFilter::AheadOf(r) | HistoryFilter::BehindOf(r) => Some(r.clone()),
            _ => None,
        };
        let case_sensitive_sort = self.config.general.case_sensitive_path_sorting;
        let (commits, effective_filter) = if let Some(stored) = stored_ref {
            let is_ahead = matches!(filter, HistoryFilter::AheadOf(_));
            let make = |r: &str| -> HistoryFilter {
                if is_ahead {
                    HistoryFilter::AheadOf(r.to_string())
                } else {
                    HistoryFilter::BehindOf(r.to_string())
                }
            };
            let trunk_ref = self
                .repos
                .get(repo_idx)
                .and_then(|r| r.trunk.as_ref())
                .map(|t| t.branch.clone());
            let upstream_ref = self
                .repos
                .get(repo_idx)
                .and_then(|r| r.upstream.as_ref())
                .map(|u| u.branch.clone());
            let mut candidates: Vec<HistoryFilter> = vec![filter];
            let mut seen = vec![stored];
            for alt in [trunk_ref.as_deref(), upstream_ref.as_deref()]
                .into_iter()
                .flatten()
            {
                if !seen.iter().any(|s| s == alt) {
                    candidates.push(make(alt));
                    seen.push(alt.to_string());
                }
            }
            candidates.push(HistoryFilter::Full);
            candidates
                .into_iter()
                .find_map(|f| {
                    let c = crate::git::get_commit_history(
                        &current_path,
                        &f,
                        HISTORY_COMMIT_LIMIT,
                        case_sensitive_sort,
                    )
                    .unwrap_or_default();
                    if !c.is_empty() || matches!(f, HistoryFilter::Full) {
                        Some((c, f))
                    } else {
                        None
                    }
                })
                .unwrap_or((Vec::new(), HistoryFilter::Full))
        } else {
            let commits = crate::git::get_commit_history(
                &current_path,
                &filter,
                HISTORY_COMMIT_LIMIT,
                case_sensitive_sort,
            )
            .unwrap_or_default();
            (commits, filter)
        };
        self.history = commits;
        self.history_filter = effective_filter;
        self.history_repo_path = current_path;
        self.history_selected = 0;
        self.history_scroll = 0;
    }
    // ── Branches pane ─────────────────────────────────────────────────────────

    /// Open the Branches pane for the selected repo, loading branch info.
    pub fn open_branches_pane(&mut self) {
        let path = match self.selected_repo_idx().and_then(|i| self.repos.get(i)) {
            Some(r) if r.error.is_none() => r.path.clone(),
            _ => return,
        };
        self.branch_info_list =
            crate::git::get_branches_with_ahead_behind(&path).unwrap_or_default();
        self.branches_pane_selected = self
            .branch_info_list
            .iter()
            .position(|b| b.is_current)
            .unwrap_or(0);
        self.branches_pane_scroll = 0;
        self.show_branches = true;
        self.focus = Focus::Branches;
        if self.show_history {
            self.reload_history_from_branches();
        }
        self.restore_base_mode();
    }

    /// Close the Branches pane, restoring focus to Repos and reverting History.
    pub fn close_branches_pane(&mut self) {
        self.show_branches = false;
        if self.focus == Focus::Branches {
            self.focus = Focus::Repos;
        }
        if self.show_history && self.branches_history_active {
            let filter = HistoryFilter::Full;
            if let Some(repo) = self.selected_repo_idx().and_then(|i| self.repos.get(i)) {
                if repo.error.is_none() {
                    let path = repo.path.clone();
                    self.load_history(path, filter);
                }
            }
            self.branches_history_active = false;
        }
        self.restore_base_mode();
    }

    /// Reload the branch list for the given repo path if it matches the current repo.
    pub fn refresh_branches_for_repo(&mut self, repo_path: &str) {
        if !self.show_branches {
            return;
        }
        let current_path = match self.selected_repo_idx().and_then(|i| self.repos.get(i)) {
            Some(r) => r.path.clone(),
            None => return,
        };
        if current_path != repo_path {
            return;
        }
        self.branch_info_list =
            crate::git::get_branches_with_ahead_behind(&current_path).unwrap_or_default();
        let n = self.branch_info_list.len();
        if n > 0 && self.branches_pane_selected >= n {
            self.branches_pane_selected = n - 1;
        }
    }

    /// Return the currently highlighted BranchInfo, if any.
    pub fn selected_branch_info(&self) -> Option<&BranchInfo> {
        self.branch_info_list.get(self.branches_pane_selected)
    }

    /// Reload the History pane from the currently selected branch in the Branches pane.
    pub fn reload_history_from_branches(&mut self) {
        if !self.show_history || !self.show_branches {
            return;
        }
        let branch_name = match self.branch_info_list.get(self.branches_pane_selected) {
            Some(b) => b.name.clone(),
            None => return,
        };
        let path = match self.repos.get(self.selected) {
            Some(r) if r.error.is_none() => r.path.clone(),
            _ => return,
        };
        let filter = HistoryFilter::BranchFull(branch_name);
        self.load_history(path, filter);
        self.branches_history_active = true;
    }

    /// Open history with a branch-specific filter (used from branch action menu).
    /// Shifts keyboard focus to the History pane.
    pub fn open_history_for_branch(&mut self, filter: HistoryFilter) {
        let path = match self.selected_repo_idx().and_then(|i| self.repos.get(i)) {
            Some(r) if r.error.is_none() => r.path.clone(),
            _ => return,
        };
        self.load_history(path, filter);
        self.show_history = true;
        self.focus = Focus::History;
        self.branches_history_active = true;
        self.save_pane_state();
        self.restore_base_mode();
    }

    /// Open the per-branch action menu for the currently selected branch.
    pub fn open_branch_action_menu(&mut self) {
        let branch = match self.branch_info_list.get(self.branches_pane_selected) {
            Some(b) => b.clone(),
            None => return,
        };
        let mut items = Vec::new();
        if !branch.is_current {
            items.push(MenuItem::item("Checkout", 'c'));
        }
        // Sync ops: pull then push — mirrors the repos-pane order (Pull / Push / Force Push).
        if let Some(upstream) = &branch.upstream {
            if upstream.behind > 0 && upstream.ahead == 0 {
                items.push(MenuItem::item("Pull Branch (fast-forward)", 'p'));
            }
        }
        if !branch.is_remote_only {
            let can_push = branch.upstream.as_ref().is_none_or(|u| u.ahead > 0);
            if can_push {
                items.push(MenuItem::item("Push Branch", 'P'));
                items.push(MenuItem::item("Force Push Branch", 'F'));
            }
        }
        items.push(MenuItem::item("Create Branch", 'n'));
        items.push(MenuItem::item("Commit History", 'h'));
        if let Some(upstream) = &branch.upstream {
            if upstream.ahead > 0 {
                items.push(MenuItem::item(
                    format!("History: Ahead of {}", upstream.branch),
                    'u',
                ));
            }
            if upstream.behind > 0 {
                items.push(MenuItem::item(
                    format!("History: Behind {}", upstream.branch),
                    'U',
                ));
            }
        }
        let upstream_branch = branch
            .upstream
            .as_ref()
            .map(|u| u.branch.as_str())
            .unwrap_or("");
        if let Some(trunk) = &branch.trunk {
            if trunk.branch != upstream_branch {
                if trunk.ahead > 0 {
                    items.push(MenuItem::item(
                        format!("History: Ahead of {}", trunk.branch),
                        't',
                    ));
                }
                if trunk.behind > 0 {
                    items.push(MenuItem::item(
                        format!("History: Behind {}", trunk.branch),
                        'T',
                    ));
                }
            }
        }
        if !branch.is_current && !branch.is_remote_only && !branch.is_trunk {
            items.push(MenuItem::separator());
            items.push(MenuItem::item("Delete Branch", 'd'));
        }
        self.open_menu(items, AppMode::BranchActionMenu);
    }

    /// Open the yes/no confirmation dialog for deleting the currently selected local branch.
    pub fn open_confirm_delete_local_branch(&mut self) {
        let Some(branch) = self.branch_info_list.get(self.branches_pane_selected) else {
            return;
        };
        if branch.is_current || branch.is_remote_only || branch.is_trunk {
            return;
        }
        self.branch_to_delete = branch.name.clone();
        self.restore_base_mode();
        self.mode = AppMode::ConfirmDeleteLocalBranch;
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

    /// Jump to the flat row index of commit `ci` (its header row).
    fn history_commit_flat_row(&self, ci: usize) -> usize {
        self.history[..ci].iter().map(|c| 1 + c.files.len()).sum()
    }

    /// Move history selection to the next commit header row (Shift+Down).
    pub fn next_commit(&mut self) {
        let Some((ci, _)) = self.history_row_at(self.history_selected) else {
            return;
        };
        let next_ci = ci + 1;
        if next_ci < self.history.len() {
            self.history_selected = self.history_commit_flat_row(next_ci);
        }
    }

    /// Move history selection to the previous commit header row (Shift+Up).
    pub fn previous_commit(&mut self) {
        let Some((ci, fi)) = self.history_row_at(self.history_selected) else {
            return;
        };
        // If already on a commit header, go to the one before it.
        // If on a file sub-row, go to this commit's header.
        let target_ci = if fi.is_none() {
            ci.saturating_sub(1)
        } else {
            ci
        };
        self.history_selected = self.history_commit_flat_row(target_ci);
    }

    pub fn log(&mut self, text: impl Into<String>) {
        self.log_at(LogLevel::Info, text);
    }

    pub fn log_debug(&mut self, text: impl Into<String>) {
        self.log_at(LogLevel::Debug, text);
    }

    pub fn log_warn(&mut self, text: impl Into<String>) {
        self.log_at(LogLevel::Warn, text);
    }

    pub fn log_error(&mut self, text: impl Into<String>) {
        self.log_at(LogLevel::Error, text);
    }

    fn log_at(&mut self, level: LogLevel, text: impl Into<String>) {
        let line = LogLine::new_at(level, text);
        crate::write_debug_log(&line);
        self.log.push(line);
        if self.log.len() > MAX_LOG_LINES {
            let drop = self.log.len() - MAX_LOG_LINES;
            self.log.drain(0..drop);
        }
    }

    pub fn toggle_details(&mut self) {
        self.show_details = !self.show_details;
        if !self.show_details {
            if self.focus == Focus::Details {
                self.focus = Focus::Repos;
            }
            self.details_content.clear();
            self.details_mode = DetailsMode::Empty;
        } else {
            self.refresh_details();
        }
        self.save_pane_state();
    }

    /// Reload Details pane content based on the currently focused/sourced pane.
    /// No-op when the Details pane is hidden.
    pub fn refresh_details(&mut self) {
        if !self.show_details {
            return;
        }
        // When the Details pane itself is focused the user is scrolling its content —
        // don't reload or reset the scroll position.
        if self.focus == Focus::Details {
            return;
        }
        match self.focus {
            Focus::FileStatus => {
                self.details_source = DetailsSource::FileStatus;
                self.details_mode = DetailsMode::Diff;
                self.details_content = self.load_file_status_diff();
                self.details_scroll = 0;
            }
            Focus::History => match self.history_row_at(self.history_selected) {
                Some((_, None)) => {
                    self.details_source = DetailsSource::HistoryCommit;
                    self.details_mode = DetailsMode::Commit;
                    self.details_content.clear();
                    self.details_scroll = 0;
                }
                Some((_, Some(_))) => {
                    self.details_source = DetailsSource::HistoryFile;
                    self.details_mode = DetailsMode::Diff;
                    self.details_content = self.load_history_diff();
                    self.details_scroll = 0;
                }
                None => {
                    self.details_mode = DetailsMode::Empty;
                    self.details_content.clear();
                    self.details_scroll = 0;
                }
            },
            _ => {
                self.details_mode = DetailsMode::Empty;
                self.details_content.clear();
                self.details_scroll = 0;
            }
        }
    }

    /// Number of scrollable lines in the Details pane (used for scroll clamping).
    pub fn details_line_count(&self) -> usize {
        match self.details_mode {
            DetailsMode::Diff => self.details_content.lines().count(),
            DetailsMode::Commit => {
                // commit hash+timestamp, change indicator, author, blank, summary, blank, body lines
                if let Some((ci, None)) = self.history_row_at(self.history_selected) {
                    if let Some(commit) = self.history.get(ci) {
                        return 4 + commit.body.lines().count().max(1);
                    }
                }
                0
            }
            DetailsMode::Empty => 0,
        }
    }

    fn load_file_status_diff(&self) -> String {
        let repo = match self.selected_repo_idx().and_then(|i| self.repos.get(i)) {
            Some(r) if r.error.is_none() => r,
            _ => return String::new(),
        };
        let file = match repo.files.get(self.file_status_selected) {
            Some(f) => f,
            None => return String::new(),
        };
        if file.status == FileStatusKind::Untracked {
            return crate::git::get_untracked_file_content(&repo.path, &file.path)
                .unwrap_or_default();
        }
        let git_bin = self.config.general.git.as_deref().unwrap_or("git");
        crate::git::get_file_diff(&repo.path, &file.path, git_bin).unwrap_or_default()
    }

    fn load_history_diff(&self) -> String {
        let (commit_idx, file_idx) = match self.history_row_at(self.history_selected) {
            Some((ci, Some(fi))) => (ci, fi),
            _ => return String::new(),
        };
        let commit = match self.history.get(commit_idx) {
            Some(c) => c,
            None => return String::new(),
        };
        let file = match commit.files.get(file_idx) {
            Some(f) => f,
            None => return String::new(),
        };
        let git_bin = self.config.general.git.as_deref().unwrap_or("git");
        crate::git::get_commit_file_diff(
            &self.history_repo_path,
            &commit.short_hash,
            &file.path,
            git_bin,
        )
        .unwrap_or_default()
    }

    pub fn toggle_file_status(&mut self) {
        self.show_file_status = !self.show_file_status;
        if self.show_file_status {
            self.file_status_selected = 0;
            self.file_status_scroll = 0;
            self.focus = Focus::FileStatus;
        } else {
            if self.focus == Focus::FileStatus {
                self.focus = Focus::Repos;
            }
        }
        self.save_pane_state();
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
        self.save_pane_state();
    }

    /// Sync current pane visibility to persisted state and save.
    fn save_pane_state(&mut self) {
        self.state.show_file_status = self.show_file_status;
        self.state.show_log = self.show_log;
        self.state.show_history = self.show_history;
        self.state.show_details = self.show_details;
        if let Err(e) = self.state.save() {
            self.log_error(format!("failed to save pane state: {e}"));
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
    /// File Status pane). Empty when no repo is selected or the repo errored.
    pub fn selected_files(&self) -> &[crate::git::FileEntry] {
        match self.selected_repo_idx().and_then(|i| self.repos.get(i)) {
            Some(r) => &r.files,
            None => &[],
        }
    }

    /// After a file-op refresh, restore the previously selected file by path.
    /// Falls back to clamping when the file no longer appears (e.g. after discard).
    /// Only acts when `reselect_file_path` is set AND the refreshed repo is the one
    /// currently shown in the status pane.
    pub fn reselect_file_after_refresh(&mut self, refreshed_repo_path: &str) {
        let target_path = match self.reselect_file_path.take() {
            Some(p) => p,
            None => return,
        };
        let selected_repo_path = self
            .selected_repo_idx()
            .and_then(|i| self.repos.get(i))
            .map(|r| r.path.clone());
        if selected_repo_path.as_deref() != Some(refreshed_repo_path) {
            return;
        }
        let files = self.selected_files();
        if let Some(idx) = files.iter().position(|f| f.path == target_path) {
            self.file_status_selected = idx;
        } else {
            let last = files.len().saturating_sub(1);
            self.file_status_selected = self.file_status_selected.min(last);
        }
    }

    /// Convenience: the kinds present in the selected repo (used to colour
    /// the File Status header).
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

    /// Reorder `app.repos` to reflect the current section structure in `state`.
    ///
    /// The canonical order is `state.all_repos_flat()` (default section first,
    /// then named sections in alphabetical order, repos within each section sorted
    /// by path).  Any repo in `app.repos` not found in the state is dropped.
    pub fn reorder_repos_to_match_sections(&mut self) {
        let ordered_paths = self.state.all_repos_flat();
        let mut by_path: HashMap<String, RepoStatus> =
            self.repos.drain(..).map(|r| (r.path.clone(), r)).collect();
        self.repos = ordered_paths
            .into_iter()
            .filter_map(|path| by_path.remove(&path))
            .collect();
    }

    // ── VisibleRow helpers ────────────────────────────────────────────────────

    /// Build the flat list of visible rows for the Repositories pane.
    ///
    /// The default section has no title row — its repos appear at the top.
    /// Each named section contributes a title row followed by its repo rows
    /// (unless collapsed, in which case only the title row is shown).
    pub fn visible_rows(&self) -> Vec<VisibleRow> {
        let mut rows: Vec<VisibleRow> = Vec::new();
        let mut flat_repo_idx: usize = 0;

        // Default section (sections[0]) — repos only, no title row.
        for _repo in &self.state.sections[0].repos {
            rows.push(VisibleRow::Repo(flat_repo_idx));
            flat_repo_idx += 1;
        }

        // Named sections (sections[1..]).
        for (section_idx, section) in self.state.sections.iter().enumerate().skip(1) {
            rows.push(VisibleRow::SectionTitle(section_idx));
            if !section.collapsed {
                for _repo in &section.repos {
                    rows.push(VisibleRow::Repo(flat_repo_idx));
                    flat_repo_idx += 1;
                }
            } else {
                flat_repo_idx += section.repos.len();
            }
        }

        rows
    }

    /// Return the `app.repos` index of the currently selected row if it is a repo
    /// row, or `None` when a section title row is selected.
    pub fn selected_repo_idx(&self) -> Option<usize> {
        match self.visible_rows().get(self.selected) {
            Some(VisibleRow::Repo(idx)) => Some(*idx),
            _ => None,
        }
    }

    /// Return the `state.sections` index when the currently selected row is a
    /// section title, otherwise `None`.
    pub fn selected_section_title_idx(&self) -> Option<usize> {
        match self.visible_rows().get(self.selected) {
            Some(VisibleRow::SectionTitle(idx)) => Some(*idx),
            _ => None,
        }
    }

    /// Return the `state.sections` index of the section containing the currently
    /// selected row (either the section whose title is selected, or the section
    /// that owns the selected repo).
    pub fn current_section_idx(&self) -> usize {
        match self.visible_rows().get(self.selected) {
            Some(VisibleRow::SectionTitle(section_idx)) => *section_idx,
            Some(VisibleRow::Repo(repo_idx)) => self.state.section_idx_for_flat_repo_idx(*repo_idx),
            None => 0,
        }
    }

    /// Title string for the Repositories pane block border.
    pub fn repos_pane_title(&self) -> String {
        let section_idx = self.current_section_idx();
        if section_idx == 0 {
            "Repositories".to_string()
        } else {
            let section_name = self.state.sections[section_idx]
                .name
                .as_deref()
                .unwrap_or("");
            format!("Repositories ( {} )", section_name)
        }
    }

    // ── Section collapse / expand ─────────────────────────────────────────────

    /// Collapse the current named section.  No-op for the default section.
    pub fn collapse_current_section(&mut self) {
        let section_idx = self.current_section_idx();
        if section_idx == 0 {
            return;
        }
        self.state.sections[section_idx].collapsed = true;
        // After collapsing, move selection to the section title row that was just collapsed.
        let rows = self.visible_rows();
        if let Some(title_pos) = rows
            .iter()
            .position(|r| matches!(r, VisibleRow::SectionTitle(idx) if *idx == section_idx))
        {
            self.selected = title_pos;
        } else {
            let row_count = rows.len();
            if row_count == 0 {
                self.selected = 0;
            } else if self.selected >= row_count {
                self.selected = row_count - 1;
            }
        }
        let _ = self.state.save();
    }

    /// Expand the current named section.  No-op for the default section.
    pub fn expand_current_section(&mut self) {
        let section_idx = self.current_section_idx();
        if section_idx == 0 {
            return;
        }
        self.state.sections[section_idx].collapsed = false;
        let _ = self.state.save();
    }

    // ── Section create / rename ───────────────────────────────────────────────

    /// Open the text-input popup for creating a new section.
    pub fn open_create_section_input(&mut self) {
        self.section_input.clear();
        self.section_input_is_create = true;
        self.section_input_target_idx = 0;
        self.mode = AppMode::SectionNameInput;
    }

    /// Open the text-input popup for renaming the currently selected section.
    /// No-op when the default section or a repo row is selected.
    pub fn open_rename_section_input(&mut self) {
        let section_idx = self.current_section_idx();
        if section_idx == 0 {
            return;
        }
        self.section_input = self.state.sections[section_idx]
            .name
            .clone()
            .unwrap_or_default();
        self.section_input_is_create = false;
        self.section_input_target_idx = section_idx;
        self.mode = AppMode::SectionNameInput;
    }

    /// Commit the current section name input (create or rename).
    /// Selects the new / renamed section title row on success.
    pub fn confirm_section_name_input(&mut self) {
        let name = self.section_input.trim().to_string();
        if name.is_empty() {
            self.restore_base_mode();
            return;
        }

        if self.section_input_is_create {
            if let Some(new_idx) = self.state.add_section(name) {
                let _ = self.state.save();
                let rows = self.visible_rows();
                if let Some(row_pos) = rows
                    .iter()
                    .position(|r| matches!(r, VisibleRow::SectionTitle(i) if *i == new_idx))
                {
                    self.selected = row_pos;
                }
            }
        } else {
            let target_idx = self.section_input_target_idx;
            if let Some(new_idx) = self.state.rename_section(target_idx, name) {
                let _ = self.state.save();
                let rows = self.visible_rows();
                if let Some(row_pos) = rows
                    .iter()
                    .position(|r| matches!(r, VisibleRow::SectionTitle(i) if *i == new_idx))
                {
                    self.selected = row_pos;
                }
            }
        }
        self.restore_base_mode();
    }

    // ── Section remove ────────────────────────────────────────────────────────

    /// Open the confirmation dialog for removing the currently selected section.
    /// No-op when the default section or a repo row is selected.
    pub fn open_confirm_remove_section(&mut self) {
        let section_idx = self.current_section_idx();
        if section_idx == 0 {
            return;
        }
        self.section_to_remove_idx = Some(section_idx);
        self.mode = AppMode::ConfirmRemoveSection;
    }

    /// Execute the confirmed section removal.
    /// All repos in the removed section are moved to the default section.
    /// Selects the first repo in the default section after removal.
    pub fn confirm_remove_section(&mut self) {
        if let Some(section_idx) = self.section_to_remove_idx.take() {
            self.state.remove_section(section_idx);
            let _ = self.state.save();
            self.reorder_repos_to_match_sections();
        }
        self.section_to_remove_idx = None;
        // Select first repo in default section or position 0.
        let new_selected = self
            .visible_rows()
            .iter()
            .position(|r| matches!(r, VisibleRow::Repo(_)))
            .unwrap_or(0);
        self.selected = new_selected;
        let visible_count = self.visible_rows().len();
        if visible_count > 0 && self.selected >= visible_count {
            self.selected = visible_count - 1;
        }
        self.restore_base_mode();
    }

    // ── Move repo to section ──────────────────────────────────────────────────

    /// Open the section-select popup for moving the selected repo to another section.
    /// No-op when a section title row is selected or there are no named sections.
    pub fn open_move_repo_section_select(&mut self) {
        let repo_idx = match self.selected_repo_idx() {
            Some(idx) => idx,
            None => return,
        };
        if !self.state.has_named_sections() {
            return;
        }
        let path = self.repos[repo_idx].path.clone();
        let current_section_idx = self.state.section_idx_for_path(&path).unwrap_or(0);

        let mut items: Vec<(usize, String)> = Vec::new();
        // Default section first (if not current).
        if current_section_idx != 0 {
            items.push((0, "Default".to_string()));
        }
        // Named sections alphabetically (already stored in alphabetical order).
        for (idx, section) in self.state.sections.iter().enumerate().skip(1) {
            if idx == current_section_idx {
                continue;
            }
            items.push((idx, section.name.clone().unwrap_or_default()));
        }

        if items.is_empty() {
            return;
        }

        self.section_select_items = items;
        self.section_select_selected = 0;
        self.mode = AppMode::SectionSelect;
    }

    pub fn section_select_next(&mut self) {
        if !self.section_select_items.is_empty() {
            let last = self.section_select_items.len() - 1;
            if self.section_select_selected < last {
                self.section_select_selected += 1;
            }
        }
    }

    pub fn section_select_previous(&mut self) {
        if self.section_select_selected > 0 {
            self.section_select_selected -= 1;
        }
    }

    /// Execute the move of the selected repo to the chosen target section.
    /// The target section is expanded; the moved repo is kept as the selected row.
    pub fn execute_move_repo(&mut self) {
        let repo_idx = match self.selected_repo_idx() {
            Some(idx) => idx,
            None => {
                self.restore_base_mode();
                return;
            }
        };
        let path = self.repos[repo_idx].path.clone();
        let target_section_idx = match self.section_select_items.get(self.section_select_selected) {
            Some((idx, _)) => *idx,
            None => {
                self.restore_base_mode();
                return;
            }
        };

        // Expand the target section so the moved repo is visible.
        if target_section_idx > 0 && target_section_idx < self.state.sections.len() {
            self.state.sections[target_section_idx].collapsed = false;
        }

        self.state.move_repo_to_section(&path, target_section_idx);
        let _ = self.state.save();
        self.reorder_repos_to_match_sections();

        // Re-select the moved repo at its new position.
        let new_pos = self
            .visible_rows()
            .iter()
            .enumerate()
            .find(|(_, row)| {
                if let VisibleRow::Repo(idx) = row {
                    self.repos.get(*idx).map(|r| r.path.as_str()) == Some(path.as_str())
                } else {
                    false
                }
            })
            .map(|(pos, _)| pos);

        if let Some(pos) = new_pos {
            self.selected = pos;
        }

        self.restore_base_mode();
    }
}

/// Format the current local wall-clock time as `HH:MM:SS`.
fn current_hms() -> String {
    use chrono::Local;
    Local::now().format("%H:%M:%S").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::{CommitEntry, CommitFileDelta, DeltaKind};
    use std::path::PathBuf;

    fn make_app() -> (App, tempfile::TempDir) {
        let tmp = tempfile::TempDir::new().unwrap();
        let app = App::new_with_overrides(None, Some(tmp.path().join("state.yaml")));
        (app, tmp)
    }

    fn make_commit(file_count: usize) -> CommitEntry {
        CommitEntry {
            short_hash: "abc12345".to_string(),
            timestamp: "2026-01-01 00:00:00".to_string(),
            author: "Test".to_string(),
            author_email: "test@example.com".to_string(),
            summary: "Test commit".to_string(),
            body: String::new(),
            files: (0..file_count)
                .map(|i| CommitFileDelta {
                    kind: DeltaKind::Modified,
                    path: format!("file{i}.txt"),
                })
                .collect(),
        }
    }

    fn init_temp_repo(dir: &PathBuf) {
        let repo = git2::Repository::init(dir).expect("git init");
        let mut cfg = repo.config().unwrap();
        cfg.set_str("user.name", "Test").unwrap();
        cfg.set_str("user.email", "test@example.com").unwrap();
    }

    #[test]
    fn remove_selected_removes_from_repos_immediately() {
        let tmp = tempfile::TempDir::new().unwrap();
        let state_file = tmp.path().join("state.yaml");

        let repo_a = tmp.path().join("a");
        let repo_b = tmp.path().join("b");
        let repo_c = tmp.path().join("c");
        for dir in [&repo_a, &repo_b, &repo_c] {
            std::fs::create_dir_all(dir).unwrap();
            init_temp_repo(dir);
        }

        let mut app = App::new_with_overrides(None, Some(state_file));
        for dir in [&repo_a, &repo_b, &repo_c] {
            app.state.add_repo(dir.to_str().unwrap());
        }

        // Manually populate app.repos so remove_selected has data to work with.
        app.repos = vec![
            crate::git::RepoStatus::error_entry(repo_a.to_str().unwrap(), ""),
            crate::git::RepoStatus::error_entry(repo_b.to_str().unwrap(), ""),
            crate::git::RepoStatus::error_entry(repo_c.to_str().unwrap(), ""),
        ];
        app.selected = 1;

        let removed = app.remove_selected();
        assert!(removed.is_some());
        assert_eq!(app.repos.len(), 2, "app.repos must shrink immediately");
        assert!(
            app.repos.iter().all(|r| r.path != repo_b.to_str().unwrap()),
            "removed repo must not appear in app.repos"
        );
        assert!(
            app.state
                .all_repos_flat()
                .iter()
                .all(|p| p != repo_b.to_str().unwrap()),
            "removed repo must not appear in state repos"
        );
    }

    #[test]
    fn selected_section_title_idx_returns_none_on_default_repo_row() {
        let tmp = tempfile::TempDir::new().unwrap();
        let state_file = tmp.path().join("state.yaml");
        let mut app = App::new_with_overrides(None, Some(state_file));
        app.state.sections[0].repos.push("/fake/repo-a".to_string());
        app.repos = vec![crate::git::RepoStatus::error_entry("/fake/repo-a", "")];
        app.selected = 0;
        assert_eq!(app.selected_section_title_idx(), None);
    }

    #[test]
    fn selected_section_title_idx_returns_some_on_named_section_title() {
        let tmp = tempfile::TempDir::new().unwrap();
        let state_file = tmp.path().join("state.yaml");
        let mut app = App::new_with_overrides(None, Some(state_file));
        // Empty default section + named section "Work" at sections[1].
        app.state.add_section("Work".to_string()).unwrap();
        // First visible row is SectionTitle(1) because default section is empty.
        app.selected = 0;
        assert_eq!(app.selected_section_title_idx(), Some(1));
    }

    #[test]
    fn selected_section_title_idx_returns_none_on_named_section_repo_row() {
        let tmp = tempfile::TempDir::new().unwrap();
        let state_file = tmp.path().join("state.yaml");
        let mut app = App::new_with_overrides(None, Some(state_file));
        app.state.add_section("Work".to_string()).unwrap();
        app.state.sections[1]
            .repos
            .push("/fake/work-repo".to_string());
        app.repos = vec![crate::git::RepoStatus::error_entry("/fake/work-repo", "")];
        // Row 0 = SectionTitle(1); row 1 = Repo(0).
        app.selected = 1;
        assert_eq!(app.selected_section_title_idx(), None);
    }

    #[test]
    fn reload_history_if_open_clears_history_when_section_title_is_selected() {
        let tmp = tempfile::TempDir::new().unwrap();
        let state_file = tmp.path().join("state.yaml");
        let mut app = App::new_with_overrides(None, Some(state_file));
        app.state.add_section("Work".to_string()).unwrap();
        // Empty default section → row 0 is SectionTitle(1).
        app.selected = 0;
        app.show_history = true;
        app.history = vec![crate::git::CommitEntry {
            short_hash: "abc12345".to_string(),
            timestamp: "2024-01-01 00:00:00".to_string(),
            author: "Test".to_string(),
            author_email: "test@test.com".to_string(),
            summary: "old commit".to_string(),
            body: String::new(),
            files: vec![],
        }];
        app.history_repo_path = "/fake/old-repo".to_string();
        app.history_selected = 2;
        app.history_scroll = 3;

        app.reload_history_if_open(false);

        assert!(app.history.is_empty(), "history must be cleared");
        assert!(
            app.history_repo_path.is_empty(),
            "history_repo_path must be cleared"
        );
        assert_eq!(app.history_selected, 0);
        assert_eq!(app.history_scroll, 0);
    }

    #[test]
    fn add_repo_path_does_not_close_file_picker() {
        let tmp_state = tempfile::TempDir::new().unwrap();
        let state_file = tmp_state.path().join("state.json");
        let repo_dir = tmp_state.path().join("repo");
        std::fs::create_dir_all(&repo_dir).unwrap();
        init_temp_repo(&repo_dir);

        let mut app = App::new_with_overrides(None, Some(state_file));
        app.enter_pick_mode();

        assert!(
            matches!(app.mode, AppMode::FilePicker),
            "expected FilePicker mode after enter_pick_mode"
        );
        assert!(app.file_explorer.is_some());

        let _ = app.add_repo_path(repo_dir.to_str().unwrap());

        assert!(
            matches!(app.mode, AppMode::FilePicker),
            "add_repo_path must not close the file picker"
        );
        assert!(
            app.file_explorer.is_some(),
            "file_explorer must remain set after add_repo_path"
        );
    }

    fn make_file_entry(path: &str, status: crate::git::FileStatusKind) -> crate::git::FileEntry {
        crate::git::FileEntry {
            path: path.to_string(),
            status,
        }
    }

    fn make_repo_with_files(
        path: &str,
        files: Vec<crate::git::FileEntry>,
    ) -> crate::git::RepoStatus {
        let mut repo = crate::git::RepoStatus::error_entry(path, "");
        repo.files = files;
        repo
    }

    fn app_with_repo(repo_path: &str, files: Vec<crate::git::FileEntry>) -> App {
        let tmp = tempfile::TempDir::new().unwrap();
        let state_file = tmp.path().join("state.yaml");
        let mut app = App::new_with_overrides(None, Some(state_file));
        app.state.sections[0].repos.push(repo_path.to_string());
        app.repos = vec![make_repo_with_files(repo_path, files)];
        app.selected = 0;
        app
    }

    #[test]
    fn reselect_file_after_refresh_finds_file_at_new_index() {
        // Before: [staged "a.rs" (idx 0), modified "b.rs" (idx 1)]  → user selects idx 0
        // After unstage: [modified "b.rs" (idx 0), modified "a.rs" (idx 1)]
        // Expect: idx 1 ("a.rs" is now at position 1)
        let repo_path = "/fake/repo";
        let mut app = app_with_repo(
            repo_path,
            vec![
                make_file_entry("a.rs", crate::git::FileStatusKind::Staged),
                make_file_entry("b.rs", crate::git::FileStatusKind::Modified),
            ],
        );
        app.file_status_selected = 0;
        app.reselect_file_path = Some("a.rs".to_string());

        // Simulate refresh: "a.rs" moved to index 1 (sort order changed after unstage).
        app.repos[0].files = vec![
            make_file_entry("b.rs", crate::git::FileStatusKind::Modified),
            make_file_entry("a.rs", crate::git::FileStatusKind::Modified),
        ];

        app.reselect_file_after_refresh(repo_path);

        assert_eq!(
            app.file_status_selected, 1,
            "should follow a.rs to its new index"
        );
        assert!(app.reselect_file_path.is_none(), "field must be cleared");
    }

    #[test]
    fn reselect_file_after_refresh_clamps_when_file_gone() {
        // File was discarded and is no longer in the list — selection should clamp
        // to the last remaining file rather than pointing out of bounds.
        let repo_path = "/fake/repo";
        let mut app = app_with_repo(
            repo_path,
            vec![
                make_file_entry("a.rs", crate::git::FileStatusKind::Modified),
                make_file_entry("b.rs", crate::git::FileStatusKind::Modified),
                make_file_entry("c.rs", crate::git::FileStatusKind::Modified),
            ],
        );
        app.file_status_selected = 2;
        app.reselect_file_path = Some("c.rs".to_string());

        // Simulate refresh: "c.rs" no longer present.
        app.repos[0].files = vec![
            make_file_entry("a.rs", crate::git::FileStatusKind::Modified),
            make_file_entry("b.rs", crate::git::FileStatusKind::Modified),
        ];

        app.reselect_file_after_refresh(repo_path);

        assert_eq!(
            app.file_status_selected, 1,
            "should clamp to last valid index"
        );
        assert!(app.reselect_file_path.is_none(), "field must be cleared");
    }

    #[test]
    fn reselect_file_after_refresh_no_op_when_path_not_set() {
        let repo_path = "/fake/repo";
        let mut app = app_with_repo(
            repo_path,
            vec![
                make_file_entry("a.rs", crate::git::FileStatusKind::Modified),
                make_file_entry("b.rs", crate::git::FileStatusKind::Modified),
            ],
        );
        app.file_status_selected = 1;
        app.reselect_file_path = None;

        app.reselect_file_after_refresh(repo_path);

        assert_eq!(
            app.file_status_selected, 1,
            "should not change when no path is pending"
        );
    }

    #[test]
    fn reselect_file_after_refresh_no_op_for_different_repo() {
        // The op completed for a repo that is NOT the currently displayed one.
        let displayed_repo = "/fake/repo-displayed";
        let other_repo = "/fake/repo-other";
        let mut app = app_with_repo(
            displayed_repo,
            vec![
                make_file_entry("a.rs", crate::git::FileStatusKind::Staged),
                make_file_entry("b.rs", crate::git::FileStatusKind::Modified),
            ],
        );
        app.file_status_selected = 0;
        app.reselect_file_path = Some("a.rs".to_string());

        app.reselect_file_after_refresh(other_repo);

        assert_eq!(
            app.file_status_selected, 0,
            "should not change for a different repo"
        );
        // Path is consumed even when skipped, so it won't linger.
        assert!(app.reselect_file_path.is_none(), "field must be cleared");
    }

    // ── sanitised_branch_name ─────────────────────────────────────────────────

    #[test]
    fn sanitised_branch_name_replaces_spaces_with_hyphens() {
        let (mut app, _tmp) = make_app();
        app.branch_input = "feature branch".to_string();
        assert_eq!(app.sanitised_branch_name(), "feature-branch");
    }

    #[test]
    fn sanitised_branch_name_strips_invalid_chars() {
        let (mut app, _tmp) = make_app();
        app.branch_input = "feat!@#ure".to_string();
        assert_eq!(app.sanitised_branch_name(), "feature");
    }

    #[test]
    fn sanitised_branch_name_keeps_allowed_chars() {
        let (mut app, _tmp) = make_app();
        app.branch_input = "feat/my-branch_v1.0".to_string();
        assert_eq!(app.sanitised_branch_name(), "feat/my-branch_v1.0");
    }

    #[test]
    fn sanitised_branch_name_trims_whitespace() {
        let (mut app, _tmp) = make_app();
        app.branch_input = "  main  ".to_string();
        assert_eq!(app.sanitised_branch_name(), "main");
    }

    // ── spinner_frame ─────────────────────────────────────────────────────────

    #[test]
    fn spinner_frame_ten_distinct_frames() {
        let (mut app, _tmp) = make_app();
        let frames: Vec<&str> = (0u64..10)
            .map(|i| {
                app.spinner_tick = i;
                app.spinner_frame()
            })
            .collect();
        let unique: std::collections::HashSet<_> = frames.iter().collect();
        assert_eq!(unique.len(), 10, "all 10 frames must be distinct");
    }

    #[test]
    fn spinner_frame_wraps_at_ten() {
        let (mut app, _tmp) = make_app();
        app.spinner_tick = 0;
        let frame_zero = app.spinner_frame();
        app.spinner_tick = 10;
        assert_eq!(app.spinner_frame(), frame_zero);
    }

    // ── history_row_count ─────────────────────────────────────────────────────

    #[test]
    fn history_row_count_empty_history() {
        let (app, _tmp) = make_app();
        assert_eq!(app.history_row_count(), 0);
    }

    #[test]
    fn history_row_count_one_commit_no_files() {
        let (mut app, _tmp) = make_app();
        app.history = vec![make_commit(0)];
        assert_eq!(app.history_row_count(), 1);
    }

    #[test]
    fn history_row_count_one_commit_with_files() {
        let (mut app, _tmp) = make_app();
        app.history = vec![make_commit(3)];
        assert_eq!(app.history_row_count(), 4); // 1 header + 3 files
    }

    #[test]
    fn history_row_count_multiple_commits() {
        let (mut app, _tmp) = make_app();
        app.history = vec![make_commit(2), make_commit(0), make_commit(1)];
        assert_eq!(app.history_row_count(), 6); // (1+2)+(1+0)+(1+1)
    }

    // ── history_row_at ────────────────────────────────────────────────────────

    #[test]
    fn history_row_at_returns_none_for_empty_history() {
        let (app, _tmp) = make_app();
        assert!(app.history_row_at(0).is_none());
    }

    #[test]
    fn history_row_at_commit_headers() {
        let (mut app, _tmp) = make_app();
        // commit 0: 2 files → rows 0,1,2  |  commit 1: 1 file → rows 3,4
        app.history = vec![make_commit(2), make_commit(1)];
        assert_eq!(app.history_row_at(0), Some((0, None)));
        assert_eq!(app.history_row_at(3), Some((1, None)));
        assert!(app.history_row_at(5).is_none());
    }

    #[test]
    fn history_row_at_file_sub_rows() {
        let (mut app, _tmp) = make_app();
        app.history = vec![make_commit(2), make_commit(1)];
        assert_eq!(app.history_row_at(1), Some((0, Some(0))));
        assert_eq!(app.history_row_at(2), Some((0, Some(1))));
        assert_eq!(app.history_row_at(4), Some((1, Some(0))));
    }

    // ── next_commit / previous_commit ─────────────────────────────────────────

    #[test]
    fn next_commit_moves_to_next_header() {
        let (mut app, _tmp) = make_app();
        // commit 0: 2 files → rows 0-2  |  commit 1: header at row 3
        app.history = vec![make_commit(2), make_commit(1)];
        app.history_selected = 0;
        app.next_commit();
        assert_eq!(app.history_selected, 3);
    }

    #[test]
    fn next_commit_at_last_commit_stays() {
        let (mut app, _tmp) = make_app();
        // commit 0: 1 file → rows 0-1  |  commit 1: no files → row 2
        app.history = vec![make_commit(1), make_commit(0)];
        app.history_selected = 2; // commit 1 header
        app.next_commit();
        assert_eq!(app.history_selected, 2, "must not advance past last commit");
    }

    #[test]
    fn previous_commit_from_file_row_goes_to_its_commit_header() {
        let (mut app, _tmp) = make_app();
        app.history = vec![make_commit(2), make_commit(1)];
        app.history_selected = 1; // file sub-row of commit 0
        app.previous_commit();
        assert_eq!(app.history_selected, 0); // commit 0 header
    }

    #[test]
    fn previous_commit_from_header_goes_to_previous_header() {
        let (mut app, _tmp) = make_app();
        app.history = vec![make_commit(2), make_commit(1)];
        app.history_selected = 3; // commit 1 header
        app.previous_commit();
        assert_eq!(app.history_selected, 0); // commit 0 header
    }

    #[test]
    fn previous_commit_at_first_header_stays() {
        let (mut app, _tmp) = make_app();
        app.history = vec![make_commit(1), make_commit(0)];
        app.history_selected = 0; // commit 0 header
        app.previous_commit();
        assert_eq!(app.history_selected, 0, "must not go before first commit");
    }

    // ── next / previous navigation (Repos focus) ──────────────────────────────

    #[test]
    fn next_repos_increments_selected() {
        let (mut app, _tmp) = make_app();
        app.state.sections[0].repos.push("/a".to_string());
        app.state.sections[0].repos.push("/b".to_string());
        app.focus = Focus::Repos;
        app.selected = 0;
        app.next();
        assert_eq!(app.selected, 1);
    }

    #[test]
    fn next_repos_clamps_at_last() {
        let (mut app, _tmp) = make_app();
        app.state.sections[0].repos.push("/a".to_string());
        app.focus = Focus::Repos;
        app.selected = 0;
        app.next();
        assert_eq!(app.selected, 0, "must not exceed last row");
    }

    #[test]
    fn next_repos_resets_file_status_selection() {
        let (mut app, _tmp) = make_app();
        app.state.sections[0].repos.push("/a".to_string());
        app.state.sections[0].repos.push("/b".to_string());
        app.focus = Focus::Repos;
        app.selected = 0;
        app.file_status_selected = 5;
        app.file_status_scroll = 3;
        app.next();
        assert_eq!(app.file_status_selected, 0);
        assert_eq!(app.file_status_scroll, 0);
    }

    #[test]
    fn previous_repos_decrements_selected() {
        let (mut app, _tmp) = make_app();
        app.state.sections[0].repos.push("/a".to_string());
        app.state.sections[0].repos.push("/b".to_string());
        app.focus = Focus::Repos;
        app.selected = 1;
        app.previous();
        assert_eq!(app.selected, 0);
    }

    #[test]
    fn previous_repos_clamps_at_zero() {
        let (mut app, _tmp) = make_app();
        app.state.sections[0].repos.push("/a".to_string());
        app.focus = Focus::Repos;
        app.selected = 0;
        app.previous();
        assert_eq!(app.selected, 0, "must not go below 0");
    }

    // ── next / previous (History focus) ──────────────────────────────────────

    #[test]
    fn next_history_increments_selected() {
        let (mut app, _tmp) = make_app();
        app.history = vec![make_commit(0), make_commit(0)];
        app.focus = Focus::History;
        app.history_selected = 0;
        app.next();
        assert_eq!(app.history_selected, 1);
    }

    #[test]
    fn next_history_clamps_at_last() {
        let (mut app, _tmp) = make_app();
        app.history = vec![make_commit(0)];
        app.focus = Focus::History;
        app.history_selected = 0;
        app.next();
        assert_eq!(app.history_selected, 0);
    }

    #[test]
    fn previous_history_decrements_selected() {
        let (mut app, _tmp) = make_app();
        app.history = vec![make_commit(0), make_commit(0)];
        app.focus = Focus::History;
        app.history_selected = 1;
        app.previous();
        assert_eq!(app.history_selected, 0);
    }

    // ── next / previous (Log focus) ───────────────────────────────────────────

    #[test]
    fn next_log_decrements_offset_toward_tail() {
        let (mut app, _tmp) = make_app();
        app.focus = Focus::Log;
        app.log_offset = 2;
        app.log_follow = false;
        app.next();
        assert_eq!(app.log_offset, 1);
        assert!(!app.log_follow);
    }

    #[test]
    fn next_log_at_offset_one_enables_follow() {
        let (mut app, _tmp) = make_app();
        app.focus = Focus::Log;
        app.log_offset = 1;
        app.log_follow = false;
        app.next();
        assert_eq!(app.log_offset, 0);
        assert!(app.log_follow);
    }

    #[test]
    fn previous_log_increments_offset_and_disables_follow() {
        let (mut app, _tmp) = make_app();
        app.log("line one".to_string());
        app.log("line two".to_string());
        app.focus = Focus::Log;
        app.log_offset = 0;
        app.log_follow = true;
        app.previous();
        assert_eq!(app.log_offset, 1);
        assert!(!app.log_follow);
    }

    // ── next_page / previous_page ─────────────────────────────────────────────

    #[test]
    fn next_page_repos_jumps_by_page_step() {
        let (mut app, _tmp) = make_app();
        for i in 0..20 {
            app.state.sections[0].repos.push(format!("/repo/{i}"));
        }
        app.focus = Focus::Repos;
        app.selected = 0;
        app.next_page();
        assert_eq!(app.selected, 10);
    }

    #[test]
    fn next_page_repos_clamps_at_last() {
        let (mut app, _tmp) = make_app();
        for i in 0..3 {
            app.state.sections[0].repos.push(format!("/repo/{i}"));
        }
        app.focus = Focus::Repos;
        app.selected = 0;
        app.next_page();
        assert_eq!(app.selected, 2);
    }

    #[test]
    fn previous_page_repos_jumps_back() {
        let (mut app, _tmp) = make_app();
        for i in 0..20 {
            app.state.sections[0].repos.push(format!("/repo/{i}"));
        }
        app.focus = Focus::Repos;
        app.selected = 15;
        app.previous_page();
        assert_eq!(app.selected, 5);
    }

    #[test]
    fn previous_page_repos_clamps_at_zero() {
        let (mut app, _tmp) = make_app();
        for i in 0..3 {
            app.state.sections[0].repos.push(format!("/repo/{i}"));
        }
        app.focus = Focus::Repos;
        app.selected = 2;
        app.previous_page();
        assert_eq!(app.selected, 0);
    }

    #[test]
    fn next_page_history_jumps_by_page_step() {
        let (mut app, _tmp) = make_app();
        for _ in 0..20 {
            app.history.push(make_commit(0));
        }
        app.focus = Focus::History;
        app.history_selected = 0;
        app.next_page();
        assert_eq!(app.history_selected, 10);
    }

    #[test]
    fn previous_page_history_clamps_at_zero() {
        let (mut app, _tmp) = make_app();
        for _ in 0..5 {
            app.history.push(make_commit(0));
        }
        app.focus = Focus::History;
        app.history_selected = 3;
        app.previous_page();
        assert_eq!(app.history_selected, 0);
    }

    // ── focus_order / cycle_focus / cycle_focus_reverse ───────────────────────

    #[test]
    fn focus_order_with_no_panes_is_repos_only() {
        let (mut app, _tmp) = make_app();
        app.show_branches = false;
        app.show_file_status = false;
        app.show_history = false;
        app.show_details = false;
        app.show_log = false;
        assert_eq!(app.focus_order(), vec![Focus::Repos]);
    }

    #[test]
    fn focus_order_with_file_status_and_log() {
        let (mut app, _tmp) = make_app();
        app.show_branches = false;
        app.show_file_status = true;
        app.show_history = false;
        app.show_details = false;
        app.show_log = true;
        assert_eq!(
            app.focus_order(),
            vec![Focus::Repos, Focus::FileStatus, Focus::Log]
        );
    }

    #[test]
    fn focus_order_branches_hides_file_status_entry() {
        let (mut app, _tmp) = make_app();
        app.show_branches = true;
        app.show_file_status = true;
        app.show_log = false;
        app.show_history = false;
        app.show_details = false;
        let order = app.focus_order();
        assert_eq!(order, vec![Focus::Branches]);
    }

    #[test]
    fn cycle_focus_advances_to_next_pane() {
        let (mut app, _tmp) = make_app();
        app.show_branches = false;
        app.show_file_status = true;
        app.show_history = false;
        app.show_details = false;
        app.show_log = false;
        app.focus = Focus::Repos;
        app.cycle_focus();
        assert_eq!(app.focus, Focus::FileStatus);
    }

    #[test]
    fn cycle_focus_wraps_from_last_to_first() {
        let (mut app, _tmp) = make_app();
        app.show_branches = false;
        app.show_file_status = true;
        app.show_history = false;
        app.show_details = false;
        app.show_log = false;
        app.focus = Focus::FileStatus;
        app.cycle_focus();
        assert_eq!(app.focus, Focus::Repos);
    }

    #[test]
    fn cycle_focus_single_pane_stays_on_repos() {
        let (mut app, _tmp) = make_app();
        app.show_branches = false;
        app.show_file_status = false;
        app.show_history = false;
        app.show_details = false;
        app.show_log = false;
        app.focus = Focus::Repos;
        app.cycle_focus();
        assert_eq!(app.focus, Focus::Repos);
    }

    #[test]
    fn cycle_focus_reverse_goes_backward() {
        let (mut app, _tmp) = make_app();
        app.show_branches = false;
        app.show_file_status = true;
        app.show_history = false;
        app.show_details = false;
        app.show_log = false;
        app.focus = Focus::FileStatus;
        app.cycle_focus_reverse();
        assert_eq!(app.focus, Focus::Repos);
    }

    // ── restore_base_mode / request_remove_selected / cancel_remove ───────────

    #[test]
    fn restore_base_mode_history_open_gives_history_mode() {
        let (mut app, _tmp) = make_app();
        app.show_history = true;
        app.mode = AppMode::ActionMenu;
        app.restore_base_mode();
        assert!(matches!(app.mode, AppMode::History));
    }

    #[test]
    fn restore_base_mode_history_closed_gives_normal_mode() {
        let (mut app, _tmp) = make_app();
        app.show_history = false;
        app.mode = AppMode::ActionMenu;
        app.restore_base_mode();
        assert!(matches!(app.mode, AppMode::Normal));
    }

    #[test]
    fn request_remove_selected_on_repo_enters_confirm_mode() {
        let (mut app, _tmp) = make_app();
        app.state.sections[0].repos.push("/fake/repo".to_string());
        app.repos = vec![crate::git::RepoStatus::error_entry("/fake/repo", "")];
        app.selected = 0;
        app.request_remove_selected();
        assert!(matches!(app.mode, AppMode::ConfirmRemove));
    }

    #[test]
    fn request_remove_selected_on_section_title_is_noop() {
        let (mut app, _tmp) = make_app();
        app.state.add_section("Work".to_string()).unwrap();
        app.selected = 0; // SectionTitle(1)
        app.mode = AppMode::Normal;
        app.request_remove_selected();
        assert!(
            matches!(app.mode, AppMode::Normal),
            "mode must not change for section title"
        );
    }

    #[test]
    fn cancel_remove_restores_normal_mode() {
        let (mut app, _tmp) = make_app();
        app.show_history = false;
        app.mode = AppMode::ConfirmRemove;
        app.cancel_remove();
        assert!(matches!(app.mode, AppMode::Normal));
    }

    // ── menu navigation ───────────────────────────────────────────────────────

    #[test]
    fn menu_next_advances_to_next_item() {
        let (mut app, _tmp) = make_app();
        app.menu_items = vec![
            MenuItem::item("A", 'a'),
            MenuItem::item("B", 'b'),
            MenuItem::item("C", 'c'),
        ];
        app.menu_selected = 0;
        app.menu_next();
        assert_eq!(app.menu_selected, 1);
    }

    #[test]
    fn menu_next_skips_separator() {
        let (mut app, _tmp) = make_app();
        app.menu_items = vec![
            MenuItem::item("A", 'a'),
            MenuItem::separator(),
            MenuItem::item("B", 'b'),
        ];
        app.menu_selected = 0;
        app.menu_next();
        assert_eq!(app.menu_selected, 2, "should skip separator and land on B");
    }

    #[test]
    fn menu_next_stays_at_last_item() {
        let (mut app, _tmp) = make_app();
        app.menu_items = vec![MenuItem::item("A", 'a'), MenuItem::item("B", 'b')];
        app.menu_selected = 1;
        app.menu_next();
        assert_eq!(app.menu_selected, 1, "must not advance past last item");
    }

    #[test]
    fn menu_previous_goes_to_prior_item() {
        let (mut app, _tmp) = make_app();
        app.menu_items = vec![
            MenuItem::item("A", 'a'),
            MenuItem::item("B", 'b'),
            MenuItem::item("C", 'c'),
        ];
        app.menu_selected = 2;
        app.menu_previous();
        assert_eq!(app.menu_selected, 1);
    }

    #[test]
    fn menu_previous_skips_separator() {
        let (mut app, _tmp) = make_app();
        app.menu_items = vec![
            MenuItem::item("A", 'a'),
            MenuItem::separator(),
            MenuItem::item("B", 'b'),
        ];
        app.menu_selected = 2;
        app.menu_previous();
        assert_eq!(app.menu_selected, 0, "should skip separator and land on A");
    }

    #[test]
    fn menu_previous_stays_at_first_item() {
        let (mut app, _tmp) = make_app();
        app.menu_items = vec![MenuItem::item("A", 'a'), MenuItem::item("B", 'b')];
        app.menu_selected = 0;
        app.menu_previous();
        assert_eq!(app.menu_selected, 0, "must not go before first item");
    }

    #[test]
    fn menu_next_page_jumps_by_page_step() {
        let (mut app, _tmp) = make_app();
        app.menu_items = (0..20)
            .map(|i| MenuItem::item(format!("item{i}"), 'x'))
            .collect();
        app.menu_selected = 0;
        app.menu_next_page();
        assert_eq!(app.menu_selected, 10);
    }

    #[test]
    fn close_menu_restores_normal_mode() {
        let (mut app, _tmp) = make_app();
        app.show_history = false;
        app.mode = AppMode::ActionMenu;
        app.close_menu();
        assert!(matches!(app.mode, AppMode::Normal));
    }

    // ── selected_file_entry ───────────────────────────────────────────────────

    #[test]
    fn selected_file_entry_returns_file_at_index() {
        let (mut app, _tmp) = make_app();
        app.state.sections[0].repos.push("/fake".to_string());
        app.repos = vec![make_repo_with_files(
            "/fake",
            vec![
                make_file_entry("a.rs", crate::git::FileStatusKind::Modified),
                make_file_entry("b.rs", crate::git::FileStatusKind::Staged),
            ],
        )];
        app.selected = 0;
        app.file_status_selected = 1;
        let entry = app.selected_file_entry().unwrap();
        assert_eq!(entry.path, "b.rs");
    }

    #[test]
    fn selected_file_entry_returns_none_when_no_files() {
        let (app, _tmp) = make_app();
        assert!(app.selected_file_entry().is_none());
    }

    // ── log functions ─────────────────────────────────────────────────────────

    #[test]
    fn log_appends_info_entry() {
        let (mut app, _tmp) = make_app();
        app.log("hello".to_string());
        assert_eq!(app.log.len(), 1);
        assert_eq!(app.log[0].text, "hello");
        assert!(matches!(app.log[0].level, LogLevel::Info));
    }

    #[test]
    fn log_debug_appends_debug_entry() {
        let (mut app, _tmp) = make_app();
        app.log_debug("dbg");
        assert_eq!(app.log.len(), 1);
        assert!(matches!(app.log[0].level, LogLevel::Debug));
    }

    #[test]
    fn log_warn_appends_warn_entry() {
        let (mut app, _tmp) = make_app();
        app.log_warn("warn");
        assert_eq!(app.log.len(), 1);
        assert!(matches!(app.log[0].level, LogLevel::Warn));
    }

    #[test]
    fn log_error_appends_error_entry() {
        let (mut app, _tmp) = make_app();
        app.log_error("err");
        assert_eq!(app.log.len(), 1);
        assert!(matches!(app.log[0].level, LogLevel::Error));
    }

    #[test]
    fn log_caps_at_max_log_lines() {
        let (mut app, _tmp) = make_app();
        for i in 0..1010u64 {
            app.log(format!("line {i}"));
        }
        assert_eq!(app.log.len(), 1000, "log must be capped at MAX_LOG_LINES");
        assert_eq!(app.log[0].text, "line 10", "oldest entries must be dropped");
    }

    // ── toggle_file_status ────────────────────────────────────────────────────

    #[test]
    fn toggle_file_status_on_sets_focus_and_resets_scroll() {
        let (mut app, _tmp) = make_app();
        app.show_file_status = false;
        app.file_status_selected = 5;
        app.toggle_file_status();
        assert!(app.show_file_status);
        assert_eq!(app.file_status_selected, 0);
        assert_eq!(app.file_status_scroll, 0);
        assert_eq!(app.focus, Focus::FileStatus);
    }

    #[test]
    fn toggle_file_status_off_resets_focus_if_was_file_status() {
        let (mut app, _tmp) = make_app();
        app.show_file_status = true;
        app.focus = Focus::FileStatus;
        app.toggle_file_status();
        assert!(!app.show_file_status);
        assert_eq!(app.focus, Focus::Repos);
    }

    // ── toggle_log ────────────────────────────────────────────────────────────

    #[test]
    fn toggle_log_on_enables_follow_and_sets_focus() {
        let (mut app, _tmp) = make_app();
        app.show_log = false;
        app.log_follow = false;
        app.log_offset = 5;
        app.toggle_log();
        assert!(app.show_log);
        assert!(app.log_follow);
        assert_eq!(app.log_offset, 0);
        assert_eq!(app.focus, Focus::Log);
    }

    #[test]
    fn toggle_log_off_resets_focus_if_was_log() {
        let (mut app, _tmp) = make_app();
        app.show_log = true;
        app.focus = Focus::Log;
        app.toggle_log();
        assert!(!app.show_log);
        assert_eq!(app.focus, Focus::Repos);
    }

    // ── toggle_details ────────────────────────────────────────────────────────

    #[test]
    fn toggle_details_off_clears_content_and_resets_focus() {
        let (mut app, _tmp) = make_app();
        app.show_details = true;
        app.focus = Focus::Details;
        app.details_content = "some diff".to_string();
        app.details_mode = DetailsMode::Diff;
        app.toggle_details();
        assert!(!app.show_details);
        assert!(app.details_content.is_empty());
        assert!(matches!(app.details_mode, DetailsMode::Empty));
        assert_eq!(app.focus, Focus::Repos);
    }

    #[test]
    fn toggle_details_on_sets_flag() {
        let (mut app, _tmp) = make_app();
        app.show_details = false;
        app.toggle_details();
        assert!(app.show_details);
    }

    // ── theme / next_theme ────────────────────────────────────────────────────

    #[test]
    fn theme_returns_valid_reference() {
        let (app, _tmp) = make_app();
        let t = app.theme();
        let _ = t.border_focused; // does not panic
    }

    #[test]
    fn next_theme_cycles_back_to_start() {
        let (mut app, _tmp) = make_app();
        let total = crate::theme::THEMES.len();
        app.theme_idx = 0;
        for _ in 0..total {
            app.next_theme();
        }
        assert_eq!(app.theme_idx, 0, "must wrap back to 0 after full cycle");
    }

    // ── visible_rows ──────────────────────────────────────────────────────────

    #[test]
    fn visible_rows_empty_state_returns_empty() {
        let (app, _tmp) = make_app();
        assert!(app.visible_rows().is_empty());
    }

    #[test]
    fn visible_rows_default_section_repos_only() {
        let (mut app, _tmp) = make_app();
        app.state.sections[0].repos.push("/a".to_string());
        app.state.sections[0].repos.push("/b".to_string());
        let rows = app.visible_rows();
        assert_eq!(rows.len(), 2);
        assert!(matches!(rows[0], VisibleRow::Repo(0)));
        assert!(matches!(rows[1], VisibleRow::Repo(1)));
    }

    #[test]
    fn visible_rows_named_section_adds_title_row() {
        let (mut app, _tmp) = make_app();
        app.state.add_section("Work".to_string()).unwrap();
        app.state.sections[1].repos.push("/w1".to_string());
        let rows = app.visible_rows();
        assert_eq!(rows.len(), 2);
        assert!(matches!(rows[0], VisibleRow::SectionTitle(1)));
        assert!(matches!(rows[1], VisibleRow::Repo(0)));
    }

    #[test]
    fn visible_rows_collapsed_section_hides_repos() {
        let (mut app, _tmp) = make_app();
        app.state.add_section("Work".to_string()).unwrap();
        app.state.sections[1].repos.push("/w1".to_string());
        app.state.sections[1].repos.push("/w2".to_string());
        app.state.sections[1].collapsed = true;
        let rows = app.visible_rows();
        assert_eq!(rows.len(), 1, "collapsed section shows only its title");
        assert!(matches!(rows[0], VisibleRow::SectionTitle(1)));
    }

    // ── selected_repo_idx / repos_pane_title ─────────────────────────────────

    #[test]
    fn selected_repo_idx_returns_idx_for_repo_row() {
        let (mut app, _tmp) = make_app();
        app.state.sections[0].repos.push("/a".to_string());
        app.selected = 0;
        assert_eq!(app.selected_repo_idx(), Some(0));
    }

    #[test]
    fn selected_repo_idx_returns_none_for_section_title() {
        let (mut app, _tmp) = make_app();
        app.state.add_section("Work".to_string()).unwrap();
        app.selected = 0; // SectionTitle(1)
        assert_eq!(app.selected_repo_idx(), None);
    }

    #[test]
    fn repos_pane_title_default_section() {
        let (app, _tmp) = make_app();
        assert_eq!(app.repos_pane_title(), "Repositories");
    }

    #[test]
    fn repos_pane_title_named_section() {
        let (mut app, _tmp) = make_app();
        app.state.add_section("Work".to_string()).unwrap();
        app.selected = 0; // selects SectionTitle(1)
        assert_eq!(app.repos_pane_title(), "Repositories ( Work )");
    }

    // ── collapse_current_section / expand_current_section ────────────────────

    #[test]
    fn collapse_current_section_collapses_named_section() {
        let (mut app, _tmp) = make_app();
        app.state.add_section("Work".to_string()).unwrap();
        app.state.sections[1].repos.push("/w1".to_string());
        app.selected = 0; // SectionTitle(1)
        assert!(!app.state.sections[1].collapsed);
        app.collapse_current_section();
        assert!(app.state.sections[1].collapsed);
    }

    #[test]
    fn collapse_current_section_selects_section_title_when_repo_row_was_selected() {
        // Bug fix: collapsing while a repo-row is selected should move selection
        // to the section title row, not stay at the same list index.
        let (mut app, _tmp) = make_app();
        app.state.add_section("Work".to_string()).unwrap();
        app.state.sections[1].repos.push("/w1".to_string());
        app.state.sections[1].repos.push("/w2".to_string());
        // visible_rows: [SectionTitle(1), Repo(0), Repo(1)]
        // Select the second repo row (index 2).
        app.selected = 2;
        app.collapse_current_section();
        assert!(app.state.sections[1].collapsed);
        // After collapse visible_rows: [SectionTitle(1)]
        // Selection must be 0 (the section title).
        assert_eq!(app.selected, 0);
        assert_eq!(app.visible_rows()[app.selected], VisibleRow::SectionTitle(1));
    }

    #[test]
    fn collapse_current_section_noop_on_default_section() {
        let (mut app, _tmp) = make_app();
        app.state.sections[0].repos.push("/a".to_string());
        app.selected = 0; // Repo(0) → default section
        app.collapse_current_section();
        assert!(!app.state.sections[0].collapsed);
    }

    #[test]
    fn expand_current_section_uncollapses_named_section() {
        let (mut app, _tmp) = make_app();
        app.state.add_section("Work".to_string()).unwrap();
        app.state.sections[1].collapsed = true;
        app.selected = 0; // SectionTitle(1)
        app.expand_current_section();
        assert!(!app.state.sections[1].collapsed);
    }

    // ── staged_file_count ─────────────────────────────────────────────────────

    #[test]
    fn staged_file_count_no_repo_returns_zero() {
        let (app, _tmp) = make_app();
        assert_eq!(app.staged_file_count(), 0);
    }

    #[test]
    fn staged_file_count_returns_count_from_selected_repo() {
        let (mut app, _tmp) = make_app();
        app.state.sections[0].repos.push("/fake".to_string());
        let mut repo = crate::git::RepoStatus::error_entry("/fake", "");
        repo.staged = 4;
        app.repos = vec![repo];
        app.selected = 0;
        assert_eq!(app.staged_file_count(), 4);
    }

    // ── details_line_count ────────────────────────────────────────────────────

    #[test]
    fn details_line_count_empty_mode_returns_zero() {
        let (mut app, _tmp) = make_app();
        app.details_mode = DetailsMode::Empty;
        assert_eq!(app.details_line_count(), 0);
    }

    #[test]
    fn details_line_count_diff_mode_counts_content_lines() {
        let (mut app, _tmp) = make_app();
        app.details_mode = DetailsMode::Diff;
        app.details_content = "line1\nline2\nline3".to_string();
        assert_eq!(app.details_line_count(), 3);
    }

    #[test]
    fn details_line_count_commit_mode_counts_fixed_plus_body() {
        let (mut app, _tmp) = make_app();
        app.details_mode = DetailsMode::Commit;
        app.history = vec![make_commit(0)]; // body is empty
        app.history_selected = 0;
        // 4 + body.lines().count().max(1) = 4 + 1 = 5
        assert_eq!(app.details_line_count(), 5);
    }

    // ── open_repo_action_menu ─────────────────────────────────────────────────

    #[test]
    fn open_repo_action_menu_section_title_opens_section_menu() {
        let (mut app, _tmp) = make_app();
        app.state.add_section("Work".to_string()).unwrap();
        app.selected = 0; // SectionTitle(1)
        app.open_repo_action_menu();
        assert!(matches!(app.mode, AppMode::ActionMenu));
        assert!(!app.menu_items.is_empty());
    }

    #[test]
    fn open_repo_action_menu_repo_row_opens_repo_menu() {
        let (mut app, _tmp) = make_app();
        app.state.sections[0].repos.push("/fake".to_string());
        let mut repo = crate::git::RepoStatus::error_entry("/fake", "");
        repo.error = None;
        app.repos = vec![repo];
        app.selected = 0;
        app.open_repo_action_menu();
        assert!(matches!(app.mode, AppMode::ActionMenu));
        assert!(!app.menu_items.is_empty());
    }

    #[test]
    fn open_repo_action_menu_out_of_bounds_is_noop() {
        let (mut app, _tmp) = make_app();
        app.selected = 99; // no rows exist
        app.mode = AppMode::Normal;
        app.open_repo_action_menu();
        assert!(matches!(app.mode, AppMode::Normal));
    }

    // ── RepoOperation::label ──────────────────────────────────────────────────

    #[test]
    fn repo_operation_label_all_variants() {
        assert_eq!(RepoOperation::Scanning.label(), "scanning");
        assert_eq!(RepoOperation::Fetching.label(), "fetching");
        assert_eq!(RepoOperation::Pulling.label(), "pulling");
        assert_eq!(RepoOperation::Pushing.label(), "pushing");
        assert_eq!(RepoOperation::Rebasing.label(), "rebasing");
        assert_eq!(RepoOperation::Committing.label(), "committing");
        assert_eq!(RepoOperation::Working.label(), "working");
    }

    // ── LogLevel::label / LogLine::formatted ──────────────────────────────────

    #[test]
    fn log_level_label_all_variants() {
        assert_eq!(LogLevel::Debug.label(), "DEBUG");
        assert_eq!(LogLevel::Info.label(), "INFO");
        assert_eq!(LogLevel::Warn.label(), "WARN");
        assert_eq!(LogLevel::Error.label(), "ERROR");
    }

    #[test]
    fn log_line_formatted_contains_level_and_text() {
        let line = LogLine::new("hello world");
        let formatted = line.formatted();
        assert!(
            formatted.contains("INFO"),
            "formatted must contain level label"
        );
        assert!(
            formatted.contains("hello world"),
            "formatted must contain the log text"
        );
    }

    // ── next / previous (FileStatus and Details focus) ────────────────────────

    #[test]
    fn next_file_status_increments_selected() {
        let (mut app, _tmp) = make_app();
        app.state.sections[0].repos.push("/fake".to_string());
        app.repos = vec![make_repo_with_files(
            "/fake",
            vec![
                make_file_entry("a.rs", crate::git::FileStatusKind::Modified),
                make_file_entry("b.rs", crate::git::FileStatusKind::Modified),
            ],
        )];
        app.selected = 0;
        app.focus = Focus::FileStatus;
        app.file_status_selected = 0;
        app.next();
        assert_eq!(app.file_status_selected, 1);
    }

    #[test]
    fn next_file_status_clamps_at_last() {
        let (mut app, _tmp) = make_app();
        app.state.sections[0].repos.push("/fake".to_string());
        app.repos = vec![make_repo_with_files(
            "/fake",
            vec![make_file_entry("a.rs", crate::git::FileStatusKind::Modified)],
        )];
        app.selected = 0;
        app.focus = Focus::FileStatus;
        app.file_status_selected = 0;
        app.next();
        assert_eq!(app.file_status_selected, 0);
    }

    #[test]
    fn previous_file_status_decrements_selected() {
        let (mut app, _tmp) = make_app();
        app.state.sections[0].repos.push("/fake".to_string());
        app.repos = vec![make_repo_with_files(
            "/fake",
            vec![
                make_file_entry("a.rs", crate::git::FileStatusKind::Modified),
                make_file_entry("b.rs", crate::git::FileStatusKind::Modified),
            ],
        )];
        app.selected = 0;
        app.focus = Focus::FileStatus;
        app.file_status_selected = 1;
        app.previous();
        assert_eq!(app.file_status_selected, 0);
    }

    #[test]
    fn next_details_increments_scroll() {
        let (mut app, _tmp) = make_app();
        app.details_mode = DetailsMode::Diff;
        app.details_content = "line1\nline2\nline3".to_string();
        app.details_scroll = 0;
        app.focus = Focus::Details;
        app.next();
        assert_eq!(app.details_scroll, 1);
    }

    #[test]
    fn previous_details_decrements_scroll() {
        let (mut app, _tmp) = make_app();
        app.details_mode = DetailsMode::Diff;
        app.details_content = "line1\nline2\nline3".to_string();
        app.details_scroll = 2;
        app.focus = Focus::Details;
        app.previous();
        assert_eq!(app.details_scroll, 1);
    }

    // ── next_page / previous_page (FileStatus and Log focus) ──────────────────

    #[test]
    fn next_page_file_status_jumps_by_page_step() {
        let (mut app, _tmp) = make_app();
        app.state.sections[0].repos.push("/fake".to_string());
        let files: Vec<_> = (0..20)
            .map(|i| make_file_entry(&format!("f{i}.rs"), crate::git::FileStatusKind::Modified))
            .collect();
        app.repos = vec![make_repo_with_files("/fake", files)];
        app.selected = 0;
        app.focus = Focus::FileStatus;
        app.file_status_selected = 0;
        app.next_page();
        assert_eq!(app.file_status_selected, 10);
    }

    #[test]
    fn previous_page_file_status_clamps_at_zero() {
        let (mut app, _tmp) = make_app();
        app.state.sections[0].repos.push("/fake".to_string());
        let files: Vec<_> = (0..5)
            .map(|i| make_file_entry(&format!("f{i}.rs"), crate::git::FileStatusKind::Modified))
            .collect();
        app.repos = vec![make_repo_with_files("/fake", files)];
        app.selected = 0;
        app.focus = Focus::FileStatus;
        app.file_status_selected = 3;
        app.previous_page();
        assert_eq!(app.file_status_selected, 0);
    }

    #[test]
    fn next_page_log_decrements_offset_toward_tail() {
        let (mut app, _tmp) = make_app();
        for i in 0..30 {
            app.log(format!("line {i}"));
        }
        app.focus = Focus::Log;
        app.log_offset = 20;
        app.log_follow = false;
        app.next_page();
        assert_eq!(app.log_offset, 10);
    }

    #[test]
    fn previous_page_log_increments_offset_away_from_tail() {
        let (mut app, _tmp) = make_app();
        for i in 0..30 {
            app.log(format!("line {i}"));
        }
        app.focus = Focus::Log;
        app.log_offset = 5;
        app.log_follow = false;
        app.previous_page();
        assert!(app.log_offset > 5, "offset should increase away from tail");
    }

    // ── open_file_action_menu ─────────────────────────────────────────────────

    #[test]
    fn open_file_action_menu_staged_shows_commit_items() {
        let (mut app, _tmp) = make_app();
        app.state.sections[0].repos.push("/fake".to_string());
        app.repos = vec![make_repo_with_files(
            "/fake",
            vec![make_file_entry("a.rs", crate::git::FileStatusKind::Staged)],
        )];
        app.selected = 0;
        app.file_status_selected = 0;
        app.open_file_action_menu();
        assert!(matches!(app.mode, AppMode::FileActionMenu));
        let labels: Vec<_> = app.menu_items.iter().map(|m| m.label.as_str()).collect();
        assert!(labels.contains(&"Commit"), "staged file should have Commit item");
        assert!(
            labels.contains(&"Unstage File"),
            "staged file should have Unstage File item"
        );
    }

    #[test]
    fn open_file_action_menu_modified_shows_stage_items() {
        let (mut app, _tmp) = make_app();
        app.state.sections[0].repos.push("/fake".to_string());
        app.repos = vec![make_repo_with_files(
            "/fake",
            vec![make_file_entry(
                "a.rs",
                crate::git::FileStatusKind::Modified,
            )],
        )];
        app.selected = 0;
        app.file_status_selected = 0;
        app.open_file_action_menu();
        assert!(matches!(app.mode, AppMode::FileActionMenu));
        let labels: Vec<_> = app.menu_items.iter().map(|m| m.label.as_str()).collect();
        assert!(labels.contains(&"Stage File"));
        assert!(labels.contains(&"Revert File"));
    }

    #[test]
    fn open_file_action_menu_untracked_shows_stage_and_discard() {
        let (mut app, _tmp) = make_app();
        app.state.sections[0].repos.push("/fake".to_string());
        app.repos = vec![make_repo_with_files(
            "/fake",
            vec![make_file_entry(
                "a.rs",
                crate::git::FileStatusKind::Untracked,
            )],
        )];
        app.selected = 0;
        app.file_status_selected = 0;
        app.open_file_action_menu();
        assert!(matches!(app.mode, AppMode::FileActionMenu));
        let labels: Vec<_> = app.menu_items.iter().map(|m| m.label.as_str()).collect();
        assert!(labels.contains(&"Stage File"));
        assert!(labels.contains(&"Discard File"));
    }

    #[test]
    fn open_file_action_menu_conflict_shows_revert() {
        let (mut app, _tmp) = make_app();
        app.state.sections[0].repos.push("/fake".to_string());
        app.repos = vec![make_repo_with_files(
            "/fake",
            vec![make_file_entry(
                "a.rs",
                crate::git::FileStatusKind::Conflict,
            )],
        )];
        app.selected = 0;
        app.file_status_selected = 0;
        app.open_file_action_menu();
        assert!(matches!(app.mode, AppMode::FileActionMenu));
        let labels: Vec<_> = app.menu_items.iter().map(|m| m.label.as_str()).collect();
        assert!(labels.contains(&"Revert File"));
        assert!(!labels.contains(&"Stage File"), "conflict should not have Stage File");
    }

    #[test]
    fn open_file_action_menu_deleted_shows_stage_deletion() {
        let (mut app, _tmp) = make_app();
        app.state.sections[0].repos.push("/fake".to_string());
        app.repos = vec![make_repo_with_files(
            "/fake",
            vec![make_file_entry(
                "a.rs",
                crate::git::FileStatusKind::Deleted,
            )],
        )];
        app.selected = 0;
        app.file_status_selected = 0;
        app.open_file_action_menu();
        assert!(matches!(app.mode, AppMode::FileActionMenu));
        let labels: Vec<_> = app.menu_items.iter().map(|m| m.label.as_str()).collect();
        assert!(labels.contains(&"Stage Deletion"));
    }

    #[test]
    fn open_file_action_menu_patch_file_adds_apply_item() {
        let (mut app, _tmp) = make_app();
        app.state.sections[0].repos.push("/fake".to_string());
        app.repos = vec![make_repo_with_files(
            "/fake",
            vec![make_file_entry("fix.patch", crate::git::FileStatusKind::Untracked)],
        )];
        app.selected = 0;
        app.file_status_selected = 0;
        app.open_file_action_menu();
        let labels: Vec<_> = app.menu_items.iter().map(|m| m.label.as_str()).collect();
        assert!(
            labels.contains(&"Apply Patch"),
            "patch files should have Apply Patch item"
        );
    }

    #[test]
    fn open_file_action_menu_no_file_is_noop() {
        let (mut app, _tmp) = make_app();
        app.mode = AppMode::Normal;
        app.open_file_action_menu(); // no files exist
        assert!(matches!(app.mode, AppMode::Normal));
    }

    // ── open_log_action_menu / open_history_action_menu ───────────────────────

    #[test]
    fn open_log_action_menu_opens_log_menu() {
        let (mut app, _tmp) = make_app();
        app.open_log_action_menu();
        assert!(matches!(app.mode, AppMode::LogActionMenu));
        assert!(!app.menu_items.is_empty());
        let labels: Vec<_> = app.menu_items.iter().map(|m| m.label.as_str()).collect();
        assert!(labels.contains(&"Copy Log Output"));
        assert!(labels.contains(&"Clear Log"));
    }

    #[test]
    fn open_history_action_menu_on_head_commit_opens_menu() {
        let (mut app, _tmp) = make_app();
        app.history = vec![make_commit(0), make_commit(0)];
        app.history_filter = HistoryFilter::Full;
        app.history_selected = 0; // HEAD commit (index 0)
        app.open_history_action_menu();
        assert!(matches!(app.mode, AppMode::HistoryActionMenu));
        let labels: Vec<_> = app.menu_items.iter().map(|m| m.label.as_str()).collect();
        assert!(labels.contains(&"Undo Commit"));
    }

    #[test]
    fn open_history_action_menu_non_head_commit_is_noop() {
        let (mut app, _tmp) = make_app();
        app.history = vec![make_commit(0), make_commit(0)];
        app.history_filter = HistoryFilter::Full;
        app.history_selected = 1; // not HEAD
        app.mode = AppMode::Normal;
        app.open_history_action_menu();
        assert!(matches!(app.mode, AppMode::Normal));
    }

    #[test]
    fn open_history_action_menu_non_full_filter_is_noop() {
        let (mut app, _tmp) = make_app();
        app.history = vec![make_commit(0)];
        app.history_filter = HistoryFilter::AheadOf("origin/main".to_string());
        app.history_selected = 0;
        app.mode = AppMode::Normal;
        app.open_history_action_menu();
        assert!(matches!(app.mode, AppMode::Normal));
    }

    // ── clear_log ─────────────────────────────────────────────────────────────

    #[test]
    fn clear_log_empties_log_and_resets_scroll() {
        let (mut app, _tmp) = make_app();
        app.log("line 1".to_string());
        app.log("line 2".to_string());
        app.log_offset = 1;
        app.log_follow = false;
        app.clear_log();
        assert!(app.log.is_empty(), "log must be empty after clear");
        assert_eq!(app.log_offset, 0);
        assert!(app.log_follow);
    }

    // ── App::new / cancel_pick / picker_selected_path ─────────────────────────

    #[test]
    fn app_new_does_not_panic() {
        let app = App::new();
        assert!(matches!(app.mode, AppMode::Normal));
        assert!(app.repos.is_empty());
    }

    #[test]
    fn cancel_pick_clears_file_explorer_and_restores_mode() {
        let (mut app, _tmp) = make_app();
        app.show_history = false;
        app.enter_pick_mode();
        assert!(matches!(app.mode, AppMode::FilePicker));
        app.cancel_pick();
        assert!(matches!(app.mode, AppMode::Normal));
        assert!(app.file_explorer.is_none());
    }

    #[test]
    fn picker_selected_path_returns_none_when_no_explorer() {
        let (app, _tmp) = make_app();
        assert!(app.picker_selected_path().is_none());
    }

    // ── reorder_repos_to_match_sections ──────────────────────────────────────

    #[test]
    fn reorder_repos_drops_stale_entries() {
        let (mut app, _tmp) = make_app();
        // State has "/a" only; repos has "/a" and "/b"
        app.state.sections[0].repos.push("/a".to_string());
        app.repos = vec![
            crate::git::RepoStatus::error_entry("/a", ""),
            crate::git::RepoStatus::error_entry("/b", ""),
        ];
        app.reorder_repos_to_match_sections();
        assert_eq!(app.repos.len(), 1);
        assert_eq!(app.repos[0].path, "/a");
    }

    // ── selected_files ────────────────────────────────────────────────────────

    #[test]
    fn selected_files_returns_files_for_selected_repo() {
        let (mut app, _tmp) = make_app();
        app.state.sections[0].repos.push("/fake".to_string());
        app.repos = vec![make_repo_with_files(
            "/fake",
            vec![make_file_entry("a.rs", crate::git::FileStatusKind::Modified)],
        )];
        app.selected = 0;
        let files = app.selected_files();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "a.rs");
    }

    #[test]
    fn selected_files_returns_empty_when_no_repo_selected() {
        let (app, _tmp) = make_app();
        assert!(app.selected_files().is_empty());
    }

    // ── set_header_flash / tick_header_flash ──────────────────────────────────

    #[test]
    fn set_header_flash_stores_message() {
        let (mut app, _tmp) = make_app();
        app.set_header_flash("test message");
        assert!(app.header_flash.is_some());
        let (msg, _) = app.header_flash.as_ref().unwrap();
        assert_eq!(msg, "test message");
    }

    #[test]
    fn tick_header_flash_keeps_recent_flash() {
        let (mut app, _tmp) = make_app();
        app.set_header_flash("fresh message");
        app.tick_header_flash();
        assert!(
            app.header_flash.is_some(),
            "recent flash should not be cleared immediately"
        );
    }

    // ── branch_select_next / previous / selected_branch_item / close ──────────

    #[test]
    fn branch_select_next_advances_selection() {
        let (mut app, _tmp) = make_app();
        app.branch_items = vec![
            BranchItem { name: "a".to_string(), is_remote: false },
            BranchItem { name: "b".to_string(), is_remote: false },
        ];
        app.branch_selected = 0;
        app.branch_select_next();
        assert_eq!(app.branch_selected, 1);
    }

    #[test]
    fn branch_select_next_wraps_to_first() {
        let (mut app, _tmp) = make_app();
        app.branch_items = vec![
            BranchItem { name: "a".to_string(), is_remote: false },
            BranchItem { name: "b".to_string(), is_remote: false },
        ];
        app.branch_selected = 1;
        app.branch_select_next();
        assert_eq!(app.branch_selected, 0);
    }

    #[test]
    fn branch_select_previous_goes_to_last_when_at_start() {
        let (mut app, _tmp) = make_app();
        app.branch_items = vec![
            BranchItem { name: "a".to_string(), is_remote: false },
            BranchItem { name: "b".to_string(), is_remote: false },
        ];
        app.branch_selected = 0;
        app.branch_select_previous();
        assert_eq!(app.branch_selected, 1);
    }

    #[test]
    fn branch_select_previous_decrements() {
        let (mut app, _tmp) = make_app();
        app.branch_items = vec![
            BranchItem { name: "a".to_string(), is_remote: false },
            BranchItem { name: "b".to_string(), is_remote: false },
        ];
        app.branch_selected = 1;
        app.branch_select_previous();
        assert_eq!(app.branch_selected, 0);
    }

    #[test]
    fn selected_branch_item_returns_current() {
        let (mut app, _tmp) = make_app();
        app.branch_items = vec![
            BranchItem { name: "main".to_string(), is_remote: false },
            BranchItem { name: "feat".to_string(), is_remote: false },
        ];
        app.branch_selected = 1;
        let item = app.selected_branch_item().unwrap();
        assert_eq!(item.name, "feat");
    }

    #[test]
    fn close_branch_select_restores_mode() {
        let (mut app, _tmp) = make_app();
        app.show_history = false;
        app.mode = AppMode::BranchSelect;
        app.close_branch_select();
        assert!(matches!(app.mode, AppMode::Normal));
    }

    // ── check_popup_timeout ───────────────────────────────────────────────────

    #[test]
    fn check_popup_timeout_keeps_fresh_popup() {
        let (mut app, _tmp) = make_app();
        use std::time::Instant;
        app.popup_message = Some("hello".to_string());
        app.popup_show_time = Some(Instant::now());
        app.mode = AppMode::PopupMessage;
        app.check_popup_timeout();
        assert!(app.popup_message.is_some(), "fresh popup should not be dismissed");
    }

    // ── section management ────────────────────────────────────────────────────

    #[test]
    fn open_create_section_input_sets_mode() {
        let (mut app, _tmp) = make_app();
        app.open_create_section_input();
        assert!(matches!(app.mode, AppMode::SectionNameInput));
        assert!(app.section_input.is_empty());
        assert!(app.section_input_is_create);
    }

    #[test]
    fn open_rename_section_input_is_noop_for_default_section() {
        let (mut app, _tmp) = make_app();
        app.state.sections[0].repos.push("/a".to_string());
        app.selected = 0; // Repo row → default section
        app.mode = AppMode::Normal;
        app.open_rename_section_input();
        assert!(matches!(app.mode, AppMode::Normal));
    }

    #[test]
    fn open_rename_section_input_sets_mode_for_named_section() {
        let (mut app, _tmp) = make_app();
        app.state.add_section("Work".to_string()).unwrap();
        app.selected = 0; // SectionTitle(1)
        app.open_rename_section_input();
        assert!(matches!(app.mode, AppMode::SectionNameInput));
        assert_eq!(app.section_input, "Work");
        assert!(!app.section_input_is_create);
    }

    #[test]
    fn confirm_section_name_input_create_adds_section() {
        let (mut app, _tmp) = make_app();
        app.section_input = "NewSection".to_string();
        app.section_input_is_create = true;
        app.confirm_section_name_input();
        assert!(app.state.sections.iter().any(|s| s.name.as_deref() == Some("NewSection")));
    }

    #[test]
    fn confirm_section_name_input_empty_restores_mode() {
        let (mut app, _tmp) = make_app();
        app.show_history = false;
        app.section_input = "   ".to_string(); // only whitespace
        app.section_input_is_create = true;
        app.confirm_section_name_input();
        assert!(matches!(app.mode, AppMode::Normal));
    }

    #[test]
    fn open_confirm_remove_section_sets_mode() {
        let (mut app, _tmp) = make_app();
        app.state.add_section("Work".to_string()).unwrap();
        app.selected = 0; // SectionTitle(1)
        app.open_confirm_remove_section();
        assert!(matches!(app.mode, AppMode::ConfirmRemoveSection));
        assert_eq!(app.section_to_remove_idx, Some(1));
    }

    #[test]
    fn open_confirm_remove_section_noop_for_default_section() {
        let (mut app, _tmp) = make_app();
        app.state.sections[0].repos.push("/a".to_string());
        app.selected = 0; // Repo row → default section
        app.mode = AppMode::Normal;
        app.open_confirm_remove_section();
        assert!(matches!(app.mode, AppMode::Normal));
    }

    // ── next_page / previous_page for Details ─────────────────────────────────

    #[test]
    fn next_page_details_jumps_scroll() {
        let (mut app, _tmp) = make_app();
        let lines: Vec<&str> = (0..30).map(|_| "line").collect();
        app.details_content = lines.join("\n");
        app.details_mode = DetailsMode::Diff;
        app.details_scroll = 0;
        app.focus = Focus::Details;
        app.next_page();
        assert_eq!(app.details_scroll, 10);
    }

    #[test]
    fn previous_page_details_clamps_at_zero() {
        let (mut app, _tmp) = make_app();
        let lines: Vec<&str> = (0..30).map(|_| "line").collect();
        app.details_content = lines.join("\n");
        app.details_mode = DetailsMode::Diff;
        app.details_scroll = 3;
        app.focus = Focus::Details;
        app.previous_page();
        assert_eq!(app.details_scroll, 0);
    }

    // ── confirm_force_push / confirm_force_push_branch ────────────────────────

    #[test]
    fn confirm_force_push_sets_mode() {
        let (mut app, _tmp) = make_app();
        app.confirm_force_push();
        assert!(matches!(app.mode, AppMode::ConfirmForcePush));
    }

    #[test]
    fn confirm_force_push_branch_sets_mode_and_name() {
        let (mut app, _tmp) = make_app();
        app.confirm_force_push_branch("feat".to_string());
        assert!(matches!(app.mode, AppMode::ConfirmForcePushBranch));
        assert_eq!(app.branch_to_force_push, "feat");
    }

    // ── open_commit_input / commit_message_text ───────────────────────────────

    #[test]
    fn open_commit_input_sets_mode() {
        let (mut app, _tmp) = make_app();
        app.open_commit_input();
        assert!(matches!(app.mode, AppMode::CommitMessageInput));
        assert!(!app.commit_is_amend);
    }

    #[test]
    fn commit_message_text_returns_textarea_content() {
        let (mut app, _tmp) = make_app();
        app.open_commit_input();
        // Empty textarea produces empty string (single blank line).
        let text = app.commit_message_text();
        assert!(text.is_empty() || text == "\n" || text == "",
            "empty textarea should give empty-ish string, got: {text:?}");
    }

    // ── close_new_branch_input ────────────────────────────────────────────────

    #[test]
    fn close_new_branch_input_restores_mode() {
        let (mut app, _tmp) = make_app();
        app.show_history = false;
        app.mode = AppMode::NewBranchInput;
        app.close_new_branch_input();
        assert!(matches!(app.mode, AppMode::Normal));
    }

    // ── confirm_remove_section ────────────────────────────────────────────────

    #[test]
    fn confirm_remove_section_removes_named_section() {
        let (mut app, _tmp) = make_app();
        app.state.add_section("Work".to_string()).unwrap();
        app.state.sections[1].repos.push("/w1".to_string());
        app.repos = vec![crate::git::RepoStatus::error_entry("/w1", "")];
        app.section_to_remove_idx = Some(1);
        app.confirm_remove_section();
        assert!(
            app.state.sections.iter().all(|s| s.name.as_deref() != Some("Work")),
            "Work section should be removed"
        );
    }

    // ── section_select_next / previous ───────────────────────────────────────

    #[test]
    fn section_select_next_increments() {
        let (mut app, _tmp) = make_app();
        app.section_select_items = vec![(0, "Default".to_string()), (1, "Work".to_string())];
        app.section_select_selected = 0;
        app.section_select_next();
        assert_eq!(app.section_select_selected, 1);
    }

    #[test]
    fn section_select_next_stays_at_last() {
        let (mut app, _tmp) = make_app();
        app.section_select_items = vec![(0, "Default".to_string()), (1, "Work".to_string())];
        app.section_select_selected = 1;
        app.section_select_next();
        assert_eq!(app.section_select_selected, 1);
    }

    #[test]
    fn section_select_previous_decrements() {
        let (mut app, _tmp) = make_app();
        app.section_select_items = vec![(0, "Default".to_string()), (1, "Work".to_string())];
        app.section_select_selected = 1;
        app.section_select_previous();
        assert_eq!(app.section_select_selected, 0);
    }

    #[test]
    fn section_select_previous_stays_at_zero() {
        let (mut app, _tmp) = make_app();
        app.section_select_items = vec![(0, "Default".to_string()), (1, "Work".to_string())];
        app.section_select_selected = 0;
        app.section_select_previous();
        assert_eq!(app.section_select_selected, 0);
    }

    // ── execute_move_repo ─────────────────────────────────────────────────────

    #[test]
    fn execute_move_repo_moves_to_target_section() {
        let (mut app, _tmp) = make_app();
        app.state.add_section("Work".to_string()).unwrap();
        app.state.sections[0].repos.push("/repo-a".to_string());
        app.repos = vec![crate::git::RepoStatus::error_entry("/repo-a", "")];
        app.selected = 0;
        app.section_select_items = vec![(1, "Work".to_string())];
        app.section_select_selected = 0;
        app.execute_move_repo();
        assert!(
            app.state.sections[1].repos.contains(&"/repo-a".to_string()),
            "repo should be in Work section after move"
        );
    }
}
