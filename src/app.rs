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
use ratatui_explorer::FileExplorer;
use std::collections::HashMap;
use std::path::PathBuf;
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
    pub commit_message: String,
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

        let state = match state_path {
            Some(p) => State::load_from_path(p),
            None => State::load(),
        };
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
            commit_message: String::new(),
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
                if !self.repos.is_empty() {
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
                if !self.repos.is_empty() {
                    let last = self.repos.len() - 1;
                    self.selected = (self.selected + PAGE_STEP).min(last);
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
        self.state.repos.clone()
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

        let added = self.state.add_repo(path);
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
    pub fn open_repo_action_menu(&mut self) {
        if self.repos.is_empty() {
            return;
        }
        let repo = &self.repos[self.selected];
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

            // Custom repo commands from config, separated from built-in actions
            let cmds = self.config.repo_commands.clone();
            if !cmds.is_empty() {
                items.push(MenuItem::separator());
                for (i, rc) in cmds.iter().enumerate() {
                    // Assign digit keys '1'–'9', then '0' for the 10th
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
        if self.repos.is_empty() {
            return;
        }
        let path = self.repos[self.selected].path.clone();
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
        self.repos.get(self.selected).map(|r| r.staged).unwrap_or(0)
    }

    /// Open the commit message dialog for a fresh commit.
    pub fn open_commit_input(&mut self) {
        self.commit_message = String::new();
        self.commit_is_amend = false;
        self.mode = AppMode::CommitMessageInput;
    }

    /// Open the commit message dialog pre-filled with the HEAD commit message for amending.
    pub fn open_amend_input(&mut self) {
        let path = match self.repos.get(self.selected) {
            Some(r) => r.path.clone(),
            None => return,
        };
        self.commit_message = crate::git::get_head_commit_message(&path)
            .map(|m| m.trim_end().to_string())
            .unwrap_or_default();
        self.commit_head_file_count = crate::git::get_head_commit_file_count(&path);
        self.commit_is_amend = true;
        self.mode = AppMode::CommitMessageInput;
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
        if self.repos.is_empty() {
            return;
        }
        let repo = &self.repos[self.selected];
        if repo.error.is_some() {
            return;
        }
        let path = repo.path.clone();
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
        let current_path = match self.repos.get(self.selected) {
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
            let trunk_ref = self.repos[self.selected]
                .trunk
                .as_ref()
                .map(|t| t.branch.clone());
            let upstream_ref = self.repos[self.selected]
                .upstream
                .as_ref()
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
        if self.repos.is_empty() {
            return;
        }
        let repo = &self.repos[self.selected];
        if repo.error.is_some() {
            return;
        }
        let path = repo.path.clone();
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
            if let Some(repo) = self.repos.get(self.selected) {
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
        let current_path = match self.repos.get(self.selected) {
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
        if self.repos.is_empty() {
            return;
        }
        let repo = &self.repos[self.selected];
        if repo.error.is_some() {
            return;
        }
        let path = repo.path.clone();
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
        let repo = match self.repos.get(self.selected) {
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
        match self.repos.get(self.selected) {
            Some(r) => &r.files,
            None => &[],
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

    /// Sort the repo list by absolute path.
    /// Uses case-insensitive comparison unless `general.case_sensitive_path_sorting` is set.
    pub fn sort_repos(&mut self) {
        if self.config.general.case_sensitive_path_sorting {
            self.repos.sort_by(|a, b| a.path.cmp(&b.path));
        } else {
            self.repos
                .sort_by(|a, b| a.path.to_lowercase().cmp(&b.path.to_lowercase()));
        }
    }
}

/// Format the current local wall-clock time as `HH:MM:SS`.
fn current_hms() -> String {
    use chrono::Local;
    Local::now().format("%H:%M:%S").to_string()
}
