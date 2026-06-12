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

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, FrameExt as _, Paragraph, Row, Table, TableState},
    Frame,
};
use std::time::Instant;

use crate::app::{App, AppMode, Focus, LogLevel, RepoOperation};
use crate::git::RepoStatus;

/// Height (rows) of the header panel — used for layout and popup positioning.
pub const HEADER_HEIGHT: u16 = 3;
/// Total rows consumed by fixed panels (header only; footer was removed).
const FIXED_PANE_HEIGHT: u16 = HEADER_HEIGHT;
/// Maximum width of the action menu popup as a percentage of the owning pane width.
pub const ACTION_MENU_MAX_WIDTH_PCT: u16 = 80;
/// Popup width (percent of terminal width) for branch-select and new-branch dialogs.
pub const BRANCH_SELECT_WIDTH_PCT: u16 = 80;
/// Minimum height (rows) for the branch-select popup.
pub const BRANCH_SELECT_MIN_HEIGHT: u16 = 5;
/// Maximum height (rows) for the branch-select popup.
pub const BRANCH_SELECT_MAX_HEIGHT: u16 = 20;

/// Rectangles for each visible pane — used by mouse event handlers in main.rs.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct PaneAreas {
    pub terminal: Rect,
    pub header: Rect,
    pub repos: Rect,
    /// Set when the Branches pane is visible (occupies the same area as repos).
    pub branches: Option<Rect>,
    pub file_status: Option<Rect>,
    pub history: Option<Rect>,
    pub log: Option<Rect>,
    pub diff: Option<Rect>,
}

/// Clamp a user-supplied repos pane height to safe bounds:
/// at least 4 rows, and leaves at least 3 rows per optional pane below.
fn clamp_repos_height(override_h: u16, total_available: u16, open_panes: u16) -> u16 {
    const MIN_REPOS: u16 = 4;
    let max = total_available
        .saturating_sub(open_panes * 3)
        .max(MIN_REPOS);
    override_h.clamp(MIN_REPOS, max)
}

/// Compute the layout rectangles for all visible panes.
/// This mirrors the layout logic in `draw()` so main.rs can use these
/// rectangles for mouse-click focus detection.
pub fn pane_areas(app: &App, total: Rect) -> PaneAreas {
    let total_available = total.height.saturating_sub(FIXED_PANE_HEIGHT);

    let show_file_status = app.show_file_status && !app.show_branches;
    let open_panes = [show_file_status, app.show_history, app.show_log]
        .into_iter()
        .filter(|&p| p)
        .count() as u16;

    let mut constraints: Vec<Constraint> = Vec::new();
    constraints.push(Constraint::Length(HEADER_HEIGHT));

    if open_panes == 0 {
        constraints.push(Constraint::Length(total_available));
    } else if let Some(override_h) = app.repos_height_override {
        let repo_height = clamp_repos_height(override_h, total_available, open_panes);
        constraints.push(Constraint::Length(repo_height));
        if show_file_status {
            constraints.push(Constraint::Fill(1));
        }
        if app.show_history {
            constraints.push(Constraint::Fill(1));
        }
        if app.show_log {
            constraints.push(Constraint::Fill(1));
        }
    } else {
        let base_share = total_available / (open_panes + 1);
        let remainder = total_available % (open_panes + 1);
        constraints.push(Constraint::Length(base_share + remainder));
        if show_file_status {
            constraints.push(Constraint::Length(base_share));
        }
        if app.show_history {
            constraints.push(Constraint::Length(base_share));
        }
        if app.show_log {
            constraints.push(Constraint::Length(base_share));
        }
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(total);

    let mut idx = 0;
    let header = chunks[idx];
    idx += 1;
    let repos = chunks[idx];
    idx += 1;

    let mut file_status: Option<Rect> = None;
    if show_file_status {
        file_status = Some(chunks[idx]);
        idx += 1;
    }

    let mut history: Option<Rect> = None;
    if app.show_history {
        history = Some(chunks[idx]);
        idx += 1;
    }

    let mut log = None;
    if app.show_log {
        log = Some(chunks[idx]);
        idx += 1;
    }
    let _ = idx;

    // When diff is shown, shrink the FileStatus and History panes to the left
    // half and expose a single right-half diff area spanning their combined height.
    let mut diff: Option<Rect> = None;
    if app.show_details && (file_status.is_some() || history.is_some()) {
        let top_area = file_status.or(history).unwrap();
        let bottom_area = history.or(file_status).unwrap();
        let combined_y = top_area.y;
        let combined_height = bottom_area.y + bottom_area.height - combined_y;
        let full_w = total.width;
        const DETAILS_MIN_W: u16 = 15;
        let half_w = app
            .details_width_override
            .unwrap_or(full_w / 2)
            .clamp(DETAILS_MIN_W, full_w.saturating_sub(DETAILS_MIN_W));
        let left_w = full_w - half_w;
        if let Some(ref mut r) = file_status {
            r.width = left_w;
        }
        if let Some(ref mut r) = history {
            r.width = left_w;
        }
        diff = Some(Rect {
            x: total.x + left_w,
            y: combined_y,
            width: half_w,
            height: combined_height,
        });
    }

    let branches = if app.show_branches { Some(repos) } else { None };

    PaneAreas {
        terminal: total,
        header,
        repos,
        branches,
        file_status,
        history,
        log,
        diff,
    }
}

pub fn draw(frame: &mut Frame, app: &mut App) {
    // Cache pane areas for mouse click detection
    app.cached_pane_areas = Some(pane_areas(app, frame.area()));

    // Compute remaining vertical space after fixed-height panels.
    let total_available = frame.area().height.saturating_sub(FIXED_PANE_HEIGHT);

    // Count open optional panes (File Status, History, Log).
    let show_file_status = app.show_file_status && !app.show_branches;
    let open_panes = [show_file_status, app.show_history, app.show_log]
        .into_iter()
        .filter(|&p| p)
        .count() as u16;

    let mut constraints: Vec<Constraint> = Vec::new();
    constraints.push(Constraint::Length(HEADER_HEIGHT));

    if open_panes == 0 {
        constraints.push(Constraint::Length(total_available));
    } else if let Some(override_h) = app.repos_height_override {
        let repo_height = clamp_repos_height(override_h, total_available, open_panes);
        constraints.push(Constraint::Length(repo_height));
        if show_file_status {
            constraints.push(Constraint::Fill(1));
        }
        if app.show_history {
            constraints.push(Constraint::Fill(1));
        }
        if app.show_log {
            constraints.push(Constraint::Fill(1));
        }
    } else {
        // Distribute available space evenly among Repositories + open panes.
        let base_share = total_available / (open_panes + 1);
        let remainder = total_available % (open_panes + 1);
        constraints.push(Constraint::Length(base_share + remainder));
        if show_file_status {
            constraints.push(Constraint::Length(base_share));
        }
        if app.show_history {
            constraints.push(Constraint::Length(base_share));
        }
        if app.show_log {
            constraints.push(Constraint::Length(base_share));
        }
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(frame.area());

    let mut idx = 0;
    let header_area = chunks[idx];
    idx += 1;
    let repos_area = chunks[idx];
    idx += 1;

    let mut file_status_area: Option<Rect> = None;
    if show_file_status {
        file_status_area = Some(chunks[idx]);
        idx += 1;
    }

    let mut history_area: Option<Rect> = None;
    if app.show_history {
        history_area = Some(chunks[idx]);
        idx += 1;
    }

    let mut log_area: Option<Rect> = None;
    if app.show_log {
        log_area = Some(chunks[idx]);
        idx += 1;
    }
    let _ = idx;

    // When diff is shown, shrink the FileStatus/History panes to the left half
    // and expose a single right-half panel spanning their combined height.
    let mut diff_area: Option<Rect> = None;
    if app.show_details && (file_status_area.is_some() || history_area.is_some()) {
        let top_area = file_status_area.or(history_area).unwrap();
        let bottom_area = history_area.or(file_status_area).unwrap();
        let combined_y = top_area.y;
        let combined_height = bottom_area.y + bottom_area.height - combined_y;
        let full_w = frame.area().width;
        const DETAILS_MIN_W: u16 = 15;
        let half_w = app
            .details_width_override
            .unwrap_or(full_w / 2)
            .clamp(DETAILS_MIN_W, full_w.saturating_sub(DETAILS_MIN_W));
        let left_w = full_w - half_w;
        if let Some(ref mut r) = file_status_area {
            r.width = left_w;
        }
        if let Some(ref mut r) = history_area {
            r.width = left_w;
        }
        diff_area = Some(Rect {
            x: frame.area().x + left_w,
            y: combined_y,
            width: half_w,
            height: combined_height,
        });
    }

    draw_header(frame, header_area, app);
    if app.show_branches {
        draw_branches_panel(frame, repos_area, app);
    } else {
        draw_repo_table(frame, repos_area, app);
    }
    if let Some(area) = file_status_area {
        draw_file_status_panel(frame, area, app);
    }
    if let Some(area) = history_area {
        draw_history_panel(frame, area, app);
    }
    if let Some(area) = diff_area {
        draw_details_panel(frame, area, app);
    }
    if let Some(area) = log_area {
        draw_log_panel(frame, area, app);
    }

    if app.mode == AppMode::FilePicker {
        draw_file_picker(frame, app);
    }
    if app.mode == AppMode::ConfirmRemove {
        draw_confirm_remove(frame, app);
    }
    if matches!(
        app.mode,
        AppMode::ActionMenu
            | AppMode::LogActionMenu
            | AppMode::FileActionMenu
            | AppMode::BranchActionMenu
    ) {
        draw_action_menu(frame, app);
    }
    if app.mode == AppMode::BranchSelect {
        draw_branch_select(frame, app, false);
    }
    if app.mode == AppMode::NewBranchInput {
        draw_new_branch_input(frame, app);
    }
    if app.mode == AppMode::CommitMessageInput {
        draw_commit_message_input(frame, app);
    }
    if app.mode == AppMode::ConfirmForcePush {
        draw_confirm_force_push(frame, app);
    }
    if app.mode == AppMode::ConfirmForcePushBranch {
        draw_confirm_force_push_branch(frame, app);
    }

    if app.mode == AppMode::ConfirmDeleteLocalBranch {
        draw_confirm_delete_local_branch(frame, app);
    }
    if app.mode == AppMode::PopupMessage {
        draw_popup_message(frame, app);
    }
    if app.mode == AppMode::HelpOverlay {
        draw_help_overlay(frame, app);
    }
}

/// Header — shows app title, spinner when scanning, auto-fetch timer, and refresh time right-aligned.
fn draw_header(frame: &mut Frame, area: Rect, app: &App) {
    let theme = app.theme();
    let refresh_text = match app.seconds_since_refresh() {
        Some(s) if s < 5 => "just now".to_string(),
        Some(s) => format!("{s}s ago"),
        None => "never".to_string(),
    };

    // Calculate auto-fetch information
    let auto_fetch_info = if app.config.general.auto_fetch_interval().as_secs() == 0 {
        "auto-fetch disabled".to_string()
    } else if let Some(next) = app.next_auto_fetch {
        let duration_until = next - Instant::now();
        let secs = duration_until.as_secs();
        if secs <= 1 {
            "fetching all now".to_string()
        } else if secs < 60 {
            format!("fetching all in {}s", secs)
        } else {
            format!("fetching all in {}m", secs / 60)
        }
    } else {
        match app.config.general.auto_fetch_interval().as_secs() {
            0 => "auto-fetch disabled".to_string(),
            interval if interval <= 60 => format!("fetching all in {}s", interval),
            interval => format!("fetching all in {}m", interval / 60),
        }
    };

    // Left side: title + optional scanning spinner + help hint.
    let mut left_spans: Vec<Span<'static>> = vec![Span::styled(
        concat!("Git Repository Overview (v", env!("CARGO_PKG_VERSION"), ")"),
        Style::default()
            .fg(theme.title)
            .add_modifier(Modifier::BOLD),
    )];

    if app.scanning {
        left_spans.push(Span::raw("  "));
        left_spans.push(Span::styled(
            app.spinner_frame(),
            Style::default().fg(theme.spinner),
        ));
        left_spans.push(Span::raw(" "));
        left_spans.push(Span::styled(
            "scanning…",
            Style::default().fg(theme.spinner),
        ));
    }

    left_spans.push(Span::styled("   ?", Style::default().fg(theme.help_key)));
    left_spans.push(Span::styled(
        " help",
        Style::default().fg(theme.refresh_info),
    ));

    // Right side: "refreshed: Xs ago" — right-aligned inside the block inner area.
    // We render two paragraphs: one left-aligned, one right-aligned.
    let block = Block::default().borders(Borders::ALL);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Left paragraph (title + spinner + help hint)
    let left_para = Paragraph::new(Line::from(left_spans));
    frame.render_widget(left_para, inner);

    // Right paragraph (refresh indicator + auto-fetch hint or flash) — right-aligned.
    // Flash replaces the auto-fetch slot so no text shifts on the right side.
    let right_text = format!("refreshed: {}  ", refresh_text);
    let right_info: Span = if let Some((msg, _)) = &app.header_flash {
        Span::styled(
            msg.as_str(),
            Style::default()
                .fg(theme.spinner)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled(&auto_fetch_info, Style::default().fg(theme.auto_fetch_info))
    };
    let right_spans = vec![
        Span::styled(right_text, Style::default().fg(theme.refresh_info)),
        right_info,
    ];
    let right_para =
        Paragraph::new(Line::from(right_spans)).alignment(ratatui::layout::Alignment::Right);
    frame.render_widget(right_para, inner);
}

fn draw_repo_table(frame: &mut Frame, area: Rect, app: &mut App) {
    // Scrolling: clamp before borrowing app.repos so the offset is correct
    // when ratatui renders the visible window.
    let visible_rows = table_visible_rows(area);
    clamp_offset(app, visible_rows);

    let theme = app.theme();
    let header_cells: Vec<Cell<'static>> = [
        "Repository",
        "Branch",
        "Status",
        "Activity",
        "↑↓ Upstream",
        "↑↓ Trunk",
    ]
    .iter()
    .map(|h| {
        Cell::from(*h).style(
            Style::default()
                .fg(theme.table_header)
                .add_modifier(Modifier::BOLD),
        )
    })
    .collect();
    let table_header = Row::new(header_cells).height(1);

    let spinner = app.spinner_frame().to_string();

    let rows = app.repos.iter().map(|repo| {
        if repo.error.is_some() {
            return build_error_row(repo, theme);
        }

        let name = repo.path.split('/').next_back().unwrap_or(&repo.path);
        let name_style = if repo.is_clean() {
            Style::default().fg(theme.repo_clean)
        } else {
            Style::default().fg(theme.repo_dirty)
        };

        let status_spans = build_status_spans(repo, theme);
        let upstream_cell = build_ahead_behind_cell(&repo.upstream, theme);
        let trunk_cell = build_trunk_cell(&repo.trunk, theme);
        let activity_cell = build_activity_cell(app.repo_operation(&repo.path), &spinner, theme);

        Row::new(vec![
            Cell::from(name).style(name_style),
            Cell::from(repo.branch.as_str()).style(Style::default().fg(theme.branch)),
            Cell::from(status_spans),
            activity_cell,
            upstream_cell,
            trunk_cell,
        ])
    });

    let table = Table::new(
        rows,
        [
            Constraint::Fill(3), // repo name
            Constraint::Fill(4), // branch
            Constraint::Fill(2), // status
            Constraint::Fill(2), // activity
            Constraint::Fill(5), // upstream
            Constraint::Fill(5), // trunk
        ],
    )
    .header(table_header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title("Repositories")
            .border_style(focus_border_style(
                app.focus == Focus::Repos || app.dragging_repos_divider || app.hover_repos_divider,
                app.theme(),
            )),
    )
    .row_highlight_style(
        Style::default()
            .bg(theme.selection_row_bg)
            .add_modifier(Modifier::BOLD),
    )
    .highlight_symbol("> ");

    // Scrolling: clamp the table_offset so the selected row stays visible.
    let mut table_state = TableState::default()
        .with_selected(Some(app.selected))
        .with_offset(app.table_offset);
    frame.render_stateful_widget(table, area, &mut table_state);

    // If the table state's auto-scroll adjusted the offset, mirror it back so
    // subsequent ticks render the same window. ratatui's Table widget rewrites
    // the offset in-place when the selection moves outside the viewport.
    app.table_offset = table_state.offset();

    draw_error_overlays(frame, area, app);

    // Scroll indicators: ▲ top-right when rows above, ▼ bottom-right when rows below.
    let has_more_above = app.table_offset > 0;
    let has_more_below = app.table_offset + visible_rows < app.repos.len();
    let t = app.theme();
    let focused = app.focus == Focus::Repos;
    let inner_x = area.x + 1;
    let inner_y = area.y + 1;
    let inner_w = area.width.saturating_sub(2);
    let inner_h = area.height.saturating_sub(2);
    let ind_color = if focused {
        t.border_focused
    } else {
        t.border_unfocused
    };
    if has_more_above && inner_w > 2 {
        frame.render_widget(
            Paragraph::new(Span::styled("▲ ", Style::default().fg(ind_color))),
            Rect {
                x: inner_x + inner_w - 2,
                y: inner_y + 1,
                width: 2,
                height: 1,
            },
        );
    }
    if has_more_below && inner_h > 1 && inner_w > 2 {
        frame.render_widget(
            Paragraph::new(Span::styled("▼ ", Style::default().fg(ind_color))),
            Rect {
                x: inner_x + inner_w - 2,
                y: inner_y + inner_h - 1,
                width: 2,
                height: 1,
            },
        );
    }

    // Drag-handle indicator on the bottom border when hovering or dragging.
    if (app.hover_repos_divider || app.dragging_repos_divider) && area.width > 4 {
        let indicator = Span::styled(
            " ↕ ",
            Style::default()
                .fg(t.border_focused)
                .add_modifier(Modifier::BOLD),
        );
        frame.render_widget(
            Paragraph::new(indicator),
            Rect {
                x: area.x + area.width / 2 - 1,
                y: area.y + area.height - 1,
                width: 3,
                height: 1,
            },
        );
    }
}

/// Render ▲/▼ scroll indicators at the top-right / bottom-right of `inner`.
/// Uses the focused/unfocused border colour so the indicator matches the pane border.
fn draw_scroll_indicators(
    frame: &mut Frame,
    inner: Rect,
    has_more_above: bool,
    has_more_below: bool,
    focused: bool,
    t: &crate::theme::Theme,
) {
    let color = if focused {
        t.border_focused
    } else {
        t.border_unfocused
    };
    if has_more_above && inner.width > 2 {
        frame.render_widget(
            Paragraph::new(Span::styled("▲ ", Style::default().fg(color))),
            Rect {
                x: inner.x + inner.width - 2,
                y: inner.y,
                width: 2,
                height: 1,
            },
        );
    }
    if has_more_below && inner.height > 1 && inner.width > 2 {
        frame.render_widget(
            Paragraph::new(Span::styled("▼ ", Style::default().fg(color))),
            Rect {
                x: inner.x + inner.width - 2,
                y: inner.y + inner.height - 1,
                width: 2,
                height: 1,
            },
        );
    }
}

/// How many data rows fit in the table area (subtract borders + header).
fn table_visible_rows(area: Rect) -> usize {
    // 2 lines for top/bottom border, 1 for header row.
    let h = area.height as i32 - 2 - 1;
    h.max(0) as usize
}

/// Adjust `app.table_offset` so the selected row is inside the viewport.
fn clamp_offset(app: &mut App, visible: usize) {
    if visible == 0 || app.repos.is_empty() {
        app.table_offset = 0;
        return;
    }
    if app.selected < app.table_offset {
        app.table_offset = app.selected;
    } else if app.selected >= app.table_offset + visible {
        app.table_offset = app.selected + 1 - visible;
    }
    let max_offset = app.repos.len().saturating_sub(visible);
    if app.table_offset > max_offset {
        app.table_offset = max_offset;
    }
}

/// Inline-error row for repos that failed to scan / have invalid paths.
/// Only the repo name is placed in col 0; the error message is rendered as a
/// full-width overlay by `draw_error_overlays` so it can use all remaining
/// horizontal space instead of being clipped to a single narrow column.
fn build_error_row(repo: &RepoStatus, theme: &crate::theme::Theme) -> Row<'static> {
    let name = repo.path.split('/').next_back().unwrap_or(&repo.path);
    Row::new(vec![
        Cell::from(name.to_string()).style(Style::default().fg(theme.error)),
        Cell::from(""),
        Cell::from(""),
        Cell::from(""),
        Cell::from(""),
        Cell::from(""),
    ])
}

/// Overdraw the error message for every visible error row, starting right after
/// the repo-name column and spanning to the right edge of the table.
///
/// This runs after the table has been rendered so the table's selection-highlight
/// (Modifier::REVERSED) is already written into the buffer; we reproduce the same
/// modifier for our spans so the highlighting looks uniform across the whole row.
fn draw_error_overlays(frame: &mut Frame, area: Rect, app: &App) {
    let theme = app.theme();

    let inner_x = area.x + 1;
    let inner_y = area.y + 1;
    let inner_w = area.width.saturating_sub(2);
    let inner_h = area.height.saturating_sub(2);
    if inner_w < 4 || inner_h < 3 {
        return;
    }

    // Data rows start after the header row (1).
    let data_y = inner_y + 1;
    let visible = inner_h.saturating_sub(1) as usize;

    // The table uses highlight_symbol "> " (2 chars).  Col 0 is Fill(3) out of
    // the 21 total fill units; the highlight symbol width is subtracted first.
    let highlight_w: u16 = 2;
    let col_space = inner_w.saturating_sub(highlight_w);
    let col0_w = (u32::from(col_space) * 3 / 21) as u16;

    let err_x = inner_x + highlight_w + col0_w;
    let err_w = inner_w.saturating_sub(highlight_w + col0_w);
    if err_w == 0 {
        return;
    }

    for (i, repo) in app.repos.iter().enumerate() {
        if i < app.table_offset {
            continue;
        }
        let row_i = i - app.table_offset;
        if row_i >= visible {
            break;
        }
        let Some(err) = &repo.error else { continue };

        let y = data_y + row_i as u16;
        let (warn_style, err_style) = if i == app.selected {
            (
                Style::default()
                    .fg(theme.placeholder)
                    .bg(theme.selection_row_bg),
                Style::default().fg(theme.error).bg(theme.selection_row_bg),
            )
        } else {
            (
                Style::default().fg(theme.placeholder),
                Style::default().fg(theme.error),
            )
        };

        let line = Line::from(vec![
            Span::styled("⚠ ", warn_style),
            Span::styled(err.to_string(), err_style),
        ]);

        frame.render_widget(
            Paragraph::new(line),
            Rect {
                x: err_x,
                y,
                width: err_w,
                height: 1,
            },
        );
    }
}

fn build_status_spans(repo: &RepoStatus, theme: &crate::theme::Theme) -> Line<'static> {
    let parts: &[(usize, &str, Color)] = &[
        (repo.staged, "S", theme.status_staged),
        (repo.conflict, "C", theme.status_conflict),
        (repo.modified, "M", theme.status_modified),
        (repo.deleted, "D", theme.status_deleted),
        (repo.added, "U", theme.status_untracked),
    ];

    let mut spans: Vec<Span<'static>> = Vec::new();
    for &(count, code, colour) in parts {
        if count > 0 {
            if !spans.is_empty() {
                spans.push(Span::raw(" "));
            }
            spans.push(Span::styled(
                format!("{count}-{code}"),
                Style::default().fg(colour),
            ));
        }
    }

    if spans.is_empty() {
        Line::from(Span::styled(
            "clean",
            Style::default().fg(theme.status_clean_text),
        ))
    } else {
        Line::from(spans)
    }
}

fn build_ahead_behind_cell(
    ab: &Option<crate::git::AheadBehind>,
    theme: &crate::theme::Theme,
) -> Cell<'static> {
    match ab {
        None => Cell::from("-").style(Style::default().fg(theme.placeholder)),
        Some(ab) => {
            let text = format!("↑{} ↓{} {}", ab.ahead, ab.behind, ab.branch);
            let style = if ab.ahead > 0 || ab.behind > 0 {
                Style::default().fg(theme.sync_warning)
            } else {
                Style::default().fg(theme.sync_ok)
            };
            Cell::from(text).style(style)
        }
    }
}

fn build_trunk_cell(
    ab: &Option<crate::git::AheadBehind>,
    theme: &crate::theme::Theme,
) -> Cell<'static> {
    match ab {
        None => Cell::from("-").style(Style::default().fg(theme.placeholder)),
        Some(ab) => {
            let text = format!("↑{} ↓{} {}", ab.ahead, ab.behind, ab.branch);
            let style = if ab.behind > 0 {
                Style::default().fg(theme.trunk_behind)
            } else if ab.ahead > 0 {
                Style::default().fg(theme.sync_warning)
            } else {
                Style::default().fg(theme.sync_ok)
            };
            Cell::from(text).style(style)
        }
    }
}

/// Per-repo busy indicator + op name. Empty cell when idle.
fn build_activity_cell(
    op: Option<RepoOperation>,
    spinner: &str,
    theme: &crate::theme::Theme,
) -> Cell<'static> {
    match op {
        None => Cell::from(""),
        Some(op) => Cell::from(Line::from(vec![
            Span::styled(spinner.to_string(), Style::default().fg(theme.activity)),
            Span::raw(" "),
            Span::styled(op.label(), Style::default().fg(theme.activity)),
        ])),
    }
}

// ── Branches panel ───────────────────────────────────────────────────────────

fn draw_branches_panel(frame: &mut Frame, area: Rect, app: &mut App) {
    let t = app.theme();
    let focused = app.focus == Focus::Branches;

    let repo_path = app
        .repos
        .get(app.selected)
        .map(|r| r.path.as_str())
        .unwrap_or("");
    let title = format!(" Branches — {repo_path} ");

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(focus_border_style(focused, t));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if app.branch_info_list.is_empty() {
        frame.render_widget(
            Paragraph::new(Span::styled(
                "no branches found",
                Style::default().fg(t.placeholder),
            )),
            inner,
        );
        return;
    }

    let visible = inner.height.saturating_sub(1) as usize; // -1 for header row
    let n = app.branch_info_list.len();

    // Clamp scroll so selected row stays in viewport.
    if app.branches_pane_selected < app.branches_pane_scroll {
        app.branches_pane_scroll = app.branches_pane_selected;
    } else if app.branches_pane_selected >= app.branches_pane_scroll + visible {
        app.branches_pane_scroll = app.branches_pane_selected + 1 - visible;
    }
    let max_scroll = n.saturating_sub(visible);
    if app.branches_pane_scroll > max_scroll {
        app.branches_pane_scroll = max_scroll;
    }

    let rows: Vec<Row<'static>> = app
        .branch_info_list
        .iter()
        .enumerate()
        .skip(app.branches_pane_scroll)
        .take(visible)
        .map(|(i, branch)| {
            let selected = focused && i == app.branches_pane_selected;
            let row_style = if selected {
                Style::default()
                    .bg(t.selection_row_bg)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let marker = match (branch.is_current, branch.is_merged) {
                (true, true) => "*✓ ",
                (true, false) => "*  ",
                (false, true) => "✓  ",
                (false, false) => "   ",
            };
            let name_style = if branch.is_current {
                Style::default().fg(t.branch).add_modifier(Modifier::BOLD)
            } else if branch.is_merged {
                Style::default().fg(t.placeholder)
            } else if branch.is_remote_only {
                Style::default().fg(t.branch_remote)
            } else {
                Style::default()
            };

            let upstream_text = if branch.is_remote_only {
                "remote only".to_string()
            } else {
                match &branch.upstream {
                    None => "-".to_string(),
                    Some(ab) => format!("↑{} ↓{} {}", ab.ahead, ab.behind, ab.branch),
                }
            };
            let upstream_style = if branch.is_remote_only {
                Style::default().fg(t.placeholder)
            } else {
                match &branch.upstream {
                    Some(ab) if ab.ahead > 0 || ab.behind > 0 => {
                        Style::default().fg(t.sync_warning)
                    }
                    Some(_) => Style::default().fg(t.sync_ok),
                    None => Style::default().fg(t.placeholder),
                }
            };

            let trunk_text = if branch.is_trunk {
                "is trunk".to_string()
            } else {
                match &branch.trunk {
                    None => "-".to_string(),
                    Some(ab) => format!("↑{} ↓{} {}", ab.ahead, ab.behind, ab.branch),
                }
            };
            let trunk_style = if branch.is_trunk {
                Style::default().fg(t.placeholder)
            } else if branch.is_merged {
                Style::default().fg(t.sync_ok)
            } else {
                match &branch.trunk {
                    Some(ab) if ab.behind > 0 => Style::default().fg(t.trunk_behind),
                    Some(ab) if ab.ahead > 0 => Style::default().fg(t.sync_warning),
                    Some(_) => Style::default().fg(t.sync_ok),
                    None => Style::default().fg(t.placeholder),
                }
            };

            Row::new(vec![
                Cell::from(format!("{}{}", marker, branch.name)).style(name_style),
                Cell::from(upstream_text).style(upstream_style),
                Cell::from(trunk_text).style(trunk_style),
            ])
            .style(row_style)
        })
        .collect();

    let header = Row::new(vec![
        Cell::from("Branch").style(
            Style::default()
                .fg(t.table_header)
                .add_modifier(Modifier::BOLD),
        ),
        Cell::from("↑↓ Upstream").style(
            Style::default()
                .fg(t.table_header)
                .add_modifier(Modifier::BOLD),
        ),
        Cell::from("↑↓ Trunk").style(
            Style::default()
                .fg(t.table_header)
                .add_modifier(Modifier::BOLD),
        ),
    ])
    .height(1);

    let table = Table::new(
        rows,
        [
            Constraint::Fill(4),
            Constraint::Fill(5),
            Constraint::Fill(5),
        ],
    )
    .header(header);
    frame.render_widget(table, inner);

    let indicators_area = Rect {
        y: inner.y + 1,
        height: inner.height.saturating_sub(1),
        ..inner
    };
    draw_scroll_indicators(
        frame,
        indicators_area,
        app.branches_pane_scroll > 0,
        app.branches_pane_scroll + visible < n,
        focused,
        t,
    );
}

// ── File Status panel ────────────────────────────────────────────────────────

#[allow(dead_code)]
/// Decide how tall the file status panel should be (clamped to a sensible range).
fn file_status_panel_height(app: &App) -> u16 {
    let count = app.selected_files().len();
    // 2 border lines + 1 header line + N file lines, clamp 5..=15
    let h = 2 + 1 + count as u16;
    h.clamp(5, 15)
}

fn draw_file_status_panel(frame: &mut Frame, area: Rect, app: &mut App) {
    let theme = app.theme();
    let title = match app.repos.get(app.selected) {
        Some(repo) => format!(" File Status — {} ", repo.path),
        None => " File Status ".to_string(),
    };

    let focused = app.focus == Focus::FileStatus;
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(focus_border_style(focused, app.theme()));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Clamp scroll before borrowing files (avoids borrow conflict).
    // We use `inner.height` which is already computed above.
    let visible = inner.height as usize;
    {
        let file_count = app.selected_files().len();
        if visible > 0 && file_count > 0 {
            if app.file_status_selected < app.file_status_scroll {
                app.file_status_scroll = app.file_status_selected;
            } else if app.file_status_selected >= app.file_status_scroll + visible {
                app.file_status_scroll = app.file_status_selected + 1 - visible;
            }
            let max_scroll = file_count.saturating_sub(visible);
            if app.file_status_scroll > max_scroll {
                app.file_status_scroll = max_scroll;
            }
        }
    }

    let files = app.selected_files();

    if files.is_empty() {
        let msg = Paragraph::new(Line::from(Span::styled(
            "no changes — working tree clean",
            Style::default().fg(theme.placeholder),
        )));
        frame.render_widget(msg, inner);
        return;
    }

    let line_width = inner.width as usize;
    let lines: Vec<Line<'static>> = files
        .iter()
        .enumerate()
        .skip(app.file_status_scroll)
        .take(visible)
        .map(|(i, f)| {
            let colour = theme.file_status_colour(&f.status);
            let selected = focused && i == app.file_status_selected;
            let line_style = if selected {
                Style::default()
                    .bg(theme.selection_row_bg)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            let code = format!(" {} ", f.status.code());
            let content_len = code.len() + 1 + f.path.len();
            let pad = " ".repeat(line_width.saturating_sub(content_len));
            Line::from(vec![
                Span::styled(
                    code,
                    Style::default().fg(colour).add_modifier(Modifier::BOLD),
                ),
                Span::raw(" "),
                Span::raw(format!("{}{}", f.path, pad)),
            ])
            .style(line_style)
        })
        .collect();

    let para = Paragraph::new(lines);
    frame.render_widget(para, inner);

    draw_scroll_indicators(
        frame,
        inner,
        app.file_status_scroll > 0,
        app.file_status_scroll + visible < files.len(),
        focused,
        theme,
    );
}

// ── Log panel ─────────────────────────────────────────────────────────────

#[allow(dead_code)]
fn log_panel_height(app: &App) -> u16 {
    let n = app.log.len() as u16;
    // 2 borders + at least 3 lines, at most 10
    (2 + n.min(8)).clamp(5, 10)
}

fn draw_log_panel(frame: &mut Frame, area: Rect, app: &mut App) {
    let theme = app.theme();
    let focused = app.focus == Focus::Log;
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Output Log ")
        .border_style(focus_border_style(focused, app.theme()));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if app.log.is_empty() {
        return;
    }

    let visible = inner.height as usize;
    let n = app.log.len();

    // log_offset is lines-from-tail (0 = tail visible).
    // When not focused OR follow is enabled, always show the tail (offset 0).
    let effective_offset = if !focused || app.log_follow {
        0
    } else {
        app.log_offset.min(n.saturating_sub(1))
    };

    // Convert lines-from-tail to an absolute start index.
    // tail_abs = last full-page start. We go back `effective_offset` more lines.
    let tail_abs = n.saturating_sub(visible);
    let start = tail_abs.saturating_sub(effective_offset);

    // If user has scrolled back to (or past) the tail, re-enable follow.
    if effective_offset == 0 {
        app.log_follow = true;
    }

    let lines: Vec<Line<'static>> = app.log[start..]
        .iter()
        .take(visible)
        .map(|l| {
            let text_color = match l.level {
                LogLevel::Debug => theme.log_debug,
                LogLevel::Info => theme.log_info,
                LogLevel::Warn => theme.log_warn,
                LogLevel::Error => theme.log_error,
            };
            Line::from(vec![
                Span::styled(
                    format!("[{} ", l.timestamp),
                    Style::default().fg(theme.log_message),
                ),
                Span::styled(
                    format!("{:>5}", l.level.label()),
                    Style::default().fg(text_color),
                ),
                Span::styled("] ", Style::default().fg(theme.log_message)),
                Span::styled(l.text.clone(), Style::default().fg(theme.log_message)),
            ])
        })
        .collect();
    frame.render_widget(Paragraph::new(lines), inner);

    draw_scroll_indicators(frame, inner, start > 0, start + visible < n, focused, theme);
}

// ── Help bar ──────────────────────────────────────────────────────────────

fn draw_history_panel(frame: &mut Frame, area: Rect, app: &mut App) {
    let t = app.theme();
    let focused = app.focus == Focus::History;
    let filter_label = app.history_filter.label();

    // Position indicator computed up front so it can go into the title.
    let commit_idx = app
        .history_row_at(app.history_selected)
        .map(|(ci, _)| ci + 1)
        .unwrap_or(1)
        .min(app.history.len().max(1));
    let pos = format!("{}/{}", commit_idx, app.history.len());

    let title = match (filter_label.is_empty(), app.history.is_empty()) {
        (true, true) => " Commit History ".to_string(),
        (true, false) => format!(" Commit History [{pos}] "),
        (false, true) => format!(" Commit History ({filter_label}) "),
        (false, false) => format!(" Commit History [{pos}] ({filter_label}) "),
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(focus_border_style(focused, t));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if app.history.is_empty() {
        frame.render_widget(
            Paragraph::new("No commits found.").style(Style::default().fg(t.history_scroll_info)),
            inner,
        );
        return;
    }

    // Two-column layout: hash | (timestamp  author  summary)
    // File sub-rows occupy col 1 only, so they align with the timestamp column.
    let total = inner.width as usize;
    let hash_w = 10usize;
    let ts_w = 20usize;
    // measure widest author name (capped at 20)
    let author_w = app
        .history
        .iter()
        .map(|c| c.author.len())
        .max()
        .unwrap_or(8)
        .min(20);
    // col 1 width = total minus hash col and the 1-char column_spacing gap
    let rest_w = total.saturating_sub(hash_w + 1);
    let summary_w = rest_w.saturating_sub(ts_w + 1 + author_w + 1);

    let visible = inner.height as usize;

    // Scroll: ensure selected row is visible.
    if app.history_selected >= app.history_scroll + visible {
        app.history_scroll = app.history_selected - visible + 1;
    }
    if app.history_selected < app.history_scroll {
        app.history_scroll = app.history_selected;
    }

    // Build flat row list for the visible window.
    let mut rows: Vec<Row<'static>> = Vec::new();
    let mut flat_idx = 0usize;

    'outer: for commit in &app.history {
        // Commit header row
        if flat_idx >= app.history_scroll {
            let selected = flat_idx == app.history_selected;
            let row_style = if selected {
                Style::default()
                    .bg(t.selection_row_bg)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            let hash_cell =
                Cell::from(commit.short_hash.clone()).style(Style::default().fg(t.history_hash));
            // Build col-1 as a styled Line: timestamp  author  summary
            let ts_text: String = format!(
                "{:<ts_w$}",
                commit.timestamp.chars().take(ts_w).collect::<String>()
            );
            let author_text: String = format!(
                "{:<author_w$}",
                commit.author.chars().take(author_w).collect::<String>()
            );
            let summary_text: String = commit.summary.chars().take(summary_w).collect();
            let rest_line = Line::from(vec![
                Span::styled(ts_text, Style::default().fg(t.history_timestamp)),
                Span::raw(" "),
                Span::styled(author_text, Style::default().fg(t.history_author)),
                Span::raw(" "),
                Span::raw(summary_text),
            ]);
            let row = Row::new(vec![hash_cell, Cell::from(rest_line)]).style(row_style);
            rows.push(row);
            if rows.len() >= visible {
                break 'outer;
            }
        }
        flat_idx += 1;

        // File sub-rows — col 0 empty, col 1 starts at the timestamp position
        for file_delta in &commit.files {
            if flat_idx >= app.history_scroll {
                let selected = flat_idx == app.history_selected;
                let row_style = if selected {
                    Style::default()
                        .bg(t.selection_row_bg)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                let file_text: String = format!("{}  {}", file_delta.kind.code(), file_delta.path)
                    .chars()
                    .take(rest_w)
                    .collect();
                let file_span = Span::styled(
                    file_text,
                    Style::default().fg(t.delta_colour(&file_delta.kind)),
                );
                let row = Row::new(vec![
                    Cell::from(""),
                    Cell::from(Line::from(vec![file_span])),
                ])
                .style(row_style);
                rows.push(row);
                if rows.len() >= visible {
                    break 'outer;
                }
            }
            flat_idx += 1;
        }
    }

    let widths = [Constraint::Length(hash_w as u16), Constraint::Min(4)];
    let table = Table::new(rows, widths).column_spacing(1);
    frame.render_widget(table, inner);

    draw_scroll_indicators(
        frame,
        inner,
        app.history_scroll > 0,
        app.history_scroll + visible < app.history_row_count(),
        focused,
        t,
    );
}

fn draw_details_panel(frame: &mut Frame, area: Rect, app: &mut App) {
    use crate::app::{DetailsMode, DetailsSource};
    let t = app.theme();
    let focused = app.focus == Focus::Details;

    match app.details_mode {
        DetailsMode::Empty => {
            let block = Block::default()
                .borders(Borders::ALL)
                .title(" Details ")
                .border_style(focus_border_style(focused, t));
            let inner = block.inner(area);
            frame.render_widget(block, area);
            frame.render_widget(
                Paragraph::new(Span::styled(
                    "Select file or commit for details.",
                    Style::default().fg(t.placeholder),
                )),
                inner,
            );
        }

        DetailsMode::Diff => {
            let title = match app.details_source {
                DetailsSource::FileStatus => {
                    let name = app
                        .selected_files()
                        .get(app.file_status_selected)
                        .map(|f| f.path.split('/').next_back().unwrap_or(&f.path).to_string())
                        .unwrap_or_default();
                    if name.is_empty() {
                        " Diff ".to_string()
                    } else {
                        format!(" Diff — {name} ")
                    }
                }
                DetailsSource::HistoryFile | DetailsSource::HistoryCommit => {
                    let name = app
                        .history_row_at(app.history_selected)
                        .and_then(|(ci, fi)| {
                            let fi = fi?;
                            app.history.get(ci)?.files.get(fi).map(|f| {
                                f.path.split('/').next_back().unwrap_or(&f.path).to_string()
                            })
                        })
                        .unwrap_or_default();
                    if name.is_empty() {
                        " Diff ".to_string()
                    } else {
                        format!(" Diff — {name} ")
                    }
                }
            };

            let block = Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(focus_border_style(focused, t));
            let inner = block.inner(area);
            frame.render_widget(block, area);

            if app.details_content.is_empty() {
                frame.render_widget(
                    Paragraph::new(Span::styled("no diff", Style::default().fg(t.placeholder))),
                    inner,
                );
                return;
            }

            let visible = inner.height as usize;
            let total_lines = app.details_content.lines().count();
            let max_scroll = total_lines.saturating_sub(visible);
            if app.details_scroll > max_scroll {
                app.details_scroll = max_scroll;
            }

            let lines: Vec<Line<'static>> = app
                .details_content
                .lines()
                .skip(app.details_scroll)
                .take(visible)
                .map(|raw| diff_line_to_ratatui(raw, t))
                .collect();

            frame.render_widget(Paragraph::new(lines), inner);
            draw_scroll_indicators(
                frame,
                inner,
                app.details_scroll > 0,
                app.details_scroll < max_scroll,
                focused,
                t,
            );
        }

        DetailsMode::Commit => {
            let (commit_idx, _) = match app.history_row_at(app.history_selected) {
                Some((ci, None)) => (ci, ()),
                _ => return,
            };
            let commit = match app.history.get(commit_idx) {
                Some(c) => c.clone(),
                None => return,
            };

            let title = format!(" Commit [{}/{}] ", commit_idx + 1, app.history.len());
            let block = Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(focus_border_style(focused, t));
            let inner = block.inner(area);
            frame.render_widget(block, area);

            // Build the change summary exactly like build_status_spans: "N-A N-M N-D N-R"
            let delta_counts: &[(crate::git::DeltaKind, &str, Color)] = &[
                (crate::git::DeltaKind::Added, "A", t.delta_added),
                (crate::git::DeltaKind::Modified, "M", t.delta_modified),
                (crate::git::DeltaKind::Deleted, "D", t.delta_deleted),
                (crate::git::DeltaKind::Renamed, "R", t.delta_modified),
            ];
            let mut change_spans: Vec<Span<'static>> = Vec::new();
            for (kind, code, colour) in delta_counts {
                let count = commit.files.iter().filter(|f| &f.kind == kind).count();
                if count > 0 {
                    if !change_spans.is_empty() {
                        change_spans.push(Span::raw(" "));
                    }
                    change_spans.push(Span::styled(
                        format!("{count}-{code}"),
                        Style::default().fg(*colour),
                    ));
                }
            }
            if change_spans.is_empty() {
                change_spans.push(Span::styled(
                    "no changes",
                    Style::default().fg(t.placeholder),
                ));
            }

            let width = inner.width as usize;

            let mut lines: Vec<Line<'static>> = vec![
                Line::from(vec![
                    Span::styled(
                        commit.short_hash.clone(),
                        Style::default().fg(t.history_hash),
                    ),
                    Span::raw("  "),
                    Span::styled(
                        commit.timestamp.clone(),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]),
                Line::from(change_spans),
                Line::from(Span::styled(
                    format!("{} <{}>", commit.author, commit.author_email),
                    Style::default().fg(t.history_author),
                )),
                Line::raw(""),
            ];

            for wrapped in word_wrap(&commit.summary, width) {
                lines.push(Line::from(Span::styled(
                    wrapped,
                    Style::default().add_modifier(Modifier::BOLD),
                )));
            }

            if !commit.body.is_empty() {
                lines.push(Line::raw(""));
                for body_line in commit.body.lines() {
                    if body_line.trim().is_empty() {
                        lines.push(Line::raw(""));
                    } else {
                        for wrapped in word_wrap(body_line, width) {
                            lines.push(Line::from(wrapped));
                        }
                    }
                }
            }

            let total_lines = lines.len();
            let visible = inner.height as usize;
            let max_scroll = total_lines.saturating_sub(visible);
            if app.details_scroll > max_scroll {
                app.details_scroll = max_scroll;
            }
            let scroll = app.details_scroll;

            let display: Vec<Line<'static>> =
                lines.into_iter().skip(scroll).take(visible).collect();
            frame.render_widget(Paragraph::new(display), inner);
            draw_scroll_indicators(frame, inner, scroll > 0, scroll < max_scroll, focused, t);
        }
    }

    // Drag-handle indicator on the left border when hovering or dragging.
    if (app.hover_details_divider || app.dragging_details_divider) && area.height > 2 {
        let indicator = Span::styled(
            "↔",
            Style::default()
                .fg(t.border_focused)
                .add_modifier(Modifier::BOLD),
        );
        frame.render_widget(
            Paragraph::new(indicator),
            Rect {
                x: area.x,
                y: area.y + area.height / 2,
                width: 1,
                height: 1,
            },
        );
    }
}

/// Word-wrap `text` to fit within `width` columns.
/// Returns at least one element; preserves empty input as `[""]`.
fn word_wrap(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![text.to_string()];
    }
    if text.trim().is_empty() {
        return vec![String::new()];
    }
    let mut out: Vec<String> = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        if line.is_empty() {
            line.push_str(word);
        } else if line.len() + 1 + word.len() <= width {
            line.push(' ');
            line.push_str(word);
        } else {
            out.push(std::mem::take(&mut line));
            line.push_str(word);
        }
    }
    if !line.is_empty() {
        out.push(line);
    }
    out
}

/// Map one raw patch line to a styled ratatui `Line`.
fn diff_line_to_ratatui(line: &str, t: &crate::theme::Theme) -> Line<'static> {
    let (style, text) = if line.starts_with('+') && !line.starts_with("+++") {
        (Style::default().fg(t.delta_added), line.to_string())
    } else if line.starts_with('-') && !line.starts_with("---") {
        (Style::default().fg(t.delta_deleted), line.to_string())
    } else if line.starts_with("@@") {
        (Style::default().fg(t.history_author), line.to_string())
    } else if line.starts_with("diff ")
        || line.starts_with("index ")
        || line.starts_with("+++")
        || line.starts_with("---")
        || line.starts_with("Binary ")
    {
        (Style::default().fg(t.placeholder), line.to_string())
    } else {
        (Style::default(), line.to_string())
    };
    Line::from(Span::styled(text, style))
}

fn draw_help_overlay(frame: &mut Frame, app: &mut App) {
    let t = app.theme();
    let height = 46_u16.min(frame.area().height.saturating_sub(HEADER_HEIGHT));
    let area = top_centered_rect(62, height, HEADER_HEIGHT, frame.area());
    app.help_overlay_area = Some(area);

    frame.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Application Help ")
        .title_style(
            Style::default()
                .fg(t.popup_border)
                .add_modifier(Modifier::BOLD),
        )
        .border_style(Style::default().fg(t.popup_border));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let sec = Style::default().fg(t.title).add_modifier(Modifier::BOLD);
    let key_sty = Style::default().fg(t.help_key).add_modifier(Modifier::BOLD);
    let desc_sty = Style::default().fg(Color::DarkGray);

    // Build one section: blank line + header + entries aligned to the widest key in that section.
    // Uses chars().count() so Unicode arrows (↑↓) measure as one display column each.
    let section =
        |title: &'static str, entries: &[(&'static str, &'static str)]| -> Vec<Line<'static>> {
            let key_width = entries
                .iter()
                .map(|(k, _)| k.chars().count())
                .max()
                .unwrap_or(0);
            let mut section_lines: Vec<Line<'static>> = vec![
                Line::raw(""),
                Line::from(Span::styled(format!(" {title}"), sec)),
            ];
            for &(k, v) in entries {
                section_lines.push(Line::from(vec![
                    Span::styled(format!("  {k:<key_width$}  "), key_sty),
                    Span::styled(v, desc_sty),
                ]));
            }
            section_lines
        };

    let build_info_sty = Style::default().fg(Color::DarkGray);
    let mut lines: Vec<Line> = vec![Line::from(Span::styled(
        format!(
            "  gitover v{} (commit {}, built {})",
            env!("CARGO_PKG_VERSION"),
            env!("GIT_SHORT_HASH"),
            env!("BUILD_TIMESTAMP"),
        ),
        build_info_sty,
    ))];

    lines.extend(section(
        "Navigation",
        &[
            ("Tab / Shift-Tab", "cycle focus"),
            ("↑ / ↓", "move up / down"),
            ("Shift-↑ / Shift-↓", "prev / next commit (History pane)"),
            (", / .", "prev / next commit (History pane)"),
            ("PgUp / PgDn", "move up / down fast"),
        ],
    ));
    lines.extend(section(
        "Toggle Panes",
        &[
            ("s", "File Status"),
            ("b", "Branches"),
            ("h", "Commit History"),
            ("d", "Details"),
            ("l", "Output Log"),
        ],
    ));
    lines.extend(section(
        "Repositories",
        &[
            ("f", "Fetch"),
            ("p", "Pull Branch"),
            ("P", "Push Branch"),
            ("F", "Force Push Branch"),
            ("c", "Checkout Branch"),
            ("n", "Create Branch"),
            ("Enter", "Action Menu"),
            ("A", "Add Repository"),
            ("D", "Remove Repository"),
            ("r", "Refresh Repository Info"),
        ],
    ));
    lines.extend(section(
        "Branches Pane",
        &[
            ("c", "Checkout selected Branch"),
            ("p", "Pull Branch (fast-forward)"),
            ("P", "Push Branch"),
            ("F", "Force Push Branch"),
            ("n", "Create Branch"),
            ("Enter", "Branch Action Menu"),
            ("Esc / b", "Close Branches Pane"),
        ],
    ));
    lines.extend(section(
        "Global",
        &[
            ("Alt-f", "Fetch All Repositories"),
            ("Ctrl-C", "Quit Application"),
        ],
    ));

    lines.push(Line::raw(""));
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::styled("Esc", key_sty),
        Span::styled("  or  ", desc_sty),
        Span::styled("Enter", key_sty),
        Span::styled("   close help popup", desc_sty),
    ]));

    let total_lines = lines.len();
    let visible = inner.height as usize;
    let max_scroll = total_lines.saturating_sub(visible);
    app.help_overlay_max_scroll = max_scroll;
    let scroll = app.help_overlay_scroll.min(max_scroll) as u16;

    frame.render_widget(Paragraph::new(lines).scroll((scroll, 0)), inner);
    draw_scroll_indicators(
        frame,
        inner,
        scroll > 0,
        (scroll as usize) + visible < total_lines,
        true,
        t,
    );
}

// ── Popups ────────────────────────────────────────────────────────────────

fn draw_file_picker(frame: &mut Frame, app: &App) {
    let Some(explorer) = &app.file_explorer else {
        return;
    };

    let t = app.theme();
    let area = centered_rect(65, 22, frame.area());

    // Outer wrapper with title and keybinding hint
    let cwd = explorer.cwd().to_string_lossy().to_string();
    let title = format!(" Add Repository — {cwd} ");
    let outer = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .title_style(
            Style::default()
                .fg(t.popup_border)
                .add_modifier(Modifier::BOLD),
        )
        .border_style(Style::default().fg(t.popup_border));

    // Help line at bottom — 1 row, inside the popup
    let inner_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(outer.inner(area));

    frame.render_widget(Clear, area);
    frame.render_widget(&outer, area);

    // File explorer widget — pass by value since widget() returns impl WidgetRef
    let explorer_widget = explorer.widget();
    let explorer_area = Block::default()
        .borders(Borders::NONE)
        .inner(inner_chunks[0]);
    frame.render_widget_ref(explorer_widget, explorer_area);

    // Keybinding hint bar — drop leftmost groups when width is tight.
    // Group widths: navigate=16, open-dir=20, parent=17, Space+Esc=29.
    let w = inner_chunks[1].width;
    let ks = Style::default().fg(t.help_key);
    let kc = Style::default().fg(t.help_key_confirm);
    let mut spans: Vec<Span> = Vec::new();
    if w >= 82 {
        spans.push(Span::styled("↑↓/jk", ks));
        spans.push(Span::raw(" navigate  "));
    }
    if w >= 66 {
        spans.push(Span::styled("Enter/→/l", ks));
        spans.push(Span::raw(" open dir  "));
    }
    if w >= 46 {
        spans.push(Span::styled("←/h/Bksp", ks));
        spans.push(Span::raw(" parent  "));
    }
    spans.push(Span::styled("Space", kc));
    spans.push(Span::raw(" add repo  "));
    spans.push(Span::styled("Esc", ks));
    spans.push(Span::raw(" cancel"));
    frame.render_widget(Paragraph::new(Line::from(spans)), inner_chunks[1]);
}

fn draw_confirm_remove(frame: &mut Frame, app: &App) {
    let t = app.theme();
    let target = app
        .repos
        .get(app.selected)
        .map(|r| r.path.clone())
        .unwrap_or_default();

    let area = centered_rect(60, 7, frame.area());
    frame.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Remove Repository? ")
        .border_style(Style::default().fg(t.popup_border_danger))
        .title_style(
            Style::default()
                .fg(t.popup_border_danger)
                .add_modifier(Modifier::BOLD),
        );
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(inner);

    frame.render_widget(
        Paragraph::new(Line::from(vec![Span::raw(
            "Stop tracking this repository?",
        )])),
        chunks[0],
    );
    frame.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(
            target,
            Style::default().fg(t.popup_target),
        )])),
        chunks[1],
    );
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("y/Enter", Style::default().fg(t.popup_confirm)),
            Span::raw(" confirm    "),
            Span::styled("n/Esc", Style::default().fg(t.popup_cancel)),
            Span::raw(" cancel"),
        ])),
        chunks[3],
    );
}

// ── Confirm delete local branch dialog ───────────────────────────────────

fn draw_confirm_delete_local_branch(frame: &mut Frame, app: &App) {
    let t = app.theme();
    let area = centered_rect(60, 7, frame.area());
    frame.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Delete Branch? ")
        .border_style(Style::default().fg(t.popup_border_danger))
        .title_style(
            Style::default()
                .fg(t.popup_border_danger)
                .add_modifier(Modifier::BOLD),
        );
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(inner);

    frame.render_widget(Paragraph::new("Delete this local branch?"), chunks[0]);
    frame.render_widget(
        Paragraph::new(Span::styled(
            app.branch_to_delete.clone(),
            Style::default().fg(t.popup_target),
        )),
        chunks[1],
    );
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("y/Enter", Style::default().fg(t.popup_confirm_danger)),
            Span::raw(" delete    "),
            Span::styled("n/Esc", Style::default().fg(t.popup_cancel)),
            Span::raw(" cancel"),
        ])),
        chunks[3],
    );
}

// ── Action menu popup ─────────────────────────────────────────────────────

/// Build the title string for the action menu based on the current app mode.
fn action_menu_title(app: &App) -> String {
    match app.mode {
        AppMode::LogActionMenu => " Output Log ".to_string(),
        AppMode::FileActionMenu => {
            let file_name = app
                .selected_files()
                .get(app.file_status_selected)
                .map(|f| f.path.split('/').next_back().unwrap_or(&f.path).to_string())
                .unwrap_or_default();
            format!(" File Actions — {file_name} ")
        }
        AppMode::BranchActionMenu => {
            let branch_name = app
                .branch_info_list
                .get(app.branches_pane_selected)
                .map(|b| b.name.clone())
                .unwrap_or_default();
            format!(" Branch Actions — {branch_name} ")
        }
        _ => {
            let repo_name = app
                .repos
                .get(app.selected)
                .map(|r| r.path.split('/').next_back().unwrap_or(&r.path).to_string())
                .unwrap_or_default();
            format!(" Actions — {repo_name} ")
        }
    }
}

/// Return the pane rect the action menu should be anchored to.
fn action_menu_pane(app: &App) -> Rect {
    let areas = app.cached_pane_areas.as_ref();
    match app.mode {
        AppMode::LogActionMenu => areas
            .and_then(|a| a.log)
            .or_else(|| areas.map(|a| a.repos))
            .unwrap_or_default(),
        AppMode::FileActionMenu => areas
            .and_then(|a| a.file_status)
            .or_else(|| areas.map(|a| a.repos))
            .unwrap_or_default(),
        _ => areas.map(|a| a.repos).unwrap_or_default(),
    }
}

/// Compute the full `Rect` for the action menu popup, centered on the current pane.
/// Width is derived from content (title + menu items), clamped to 80% of the pane width.
pub fn action_menu_area(app: &App) -> Rect {
    let title = action_menu_title(app);
    let title_display_width = title.chars().count() as u16;
    let max_label_width = app
        .menu_items
        .iter()
        .filter(|i| !i.is_separator)
        .map(|i| i.label.chars().count() as u16)
        .max()
        .unwrap_or(0);
    // 4 = key column (Constraint::Length(4)), 2 = left+right borders
    let natural_width = (title_display_width + 2).max(max_label_width + 6);
    let pane = action_menu_pane(app);
    let max_allowed = (pane.width * ACTION_MENU_MAX_WIDTH_PCT / 100).max(10);
    let width = natural_width.min(max_allowed).max(10);
    let n = app.menu_items.len() as u16;
    // Height: allow the popup to extend below the originating pane down to the
    // terminal bottom, so long menus are never truncated by pane boundaries.
    let terminal = app
        .cached_pane_areas
        .as_ref()
        .map(|a| a.terminal)
        .unwrap_or(pane);
    let available_height = (terminal.y + terminal.height).saturating_sub(pane.y);
    let height = (n + 2).min(available_height).max(3);
    pane_top_rect(width, height, pane)
}

/// Position a popup of `width × height` at the top edge of `pane`, horizontally centered.
/// The caller is responsible for clamping `height` to the available vertical space.
pub fn pane_top_rect(width: u16, height: u16, pane: Rect) -> Rect {
    let x = pane.x + (pane.width.saturating_sub(width)) / 2;
    Rect {
        x,
        y: pane.y,
        width: width.min(pane.width),
        height,
    }
}

fn draw_action_menu(frame: &mut Frame, app: &mut App) {
    let t = app.theme();
    let title = action_menu_title(app);
    let n = app.menu_items.len();
    let area = action_menu_area(app);
    frame.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(Style::default().fg(t.popup_border));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let visible = inner.height as usize;

    // Clamp scroll so menu_selected is always in the viewport.
    if n > 0 && visible > 0 {
        if app.menu_selected < app.menu_scroll {
            app.menu_scroll = app.menu_selected;
        } else if app.menu_selected >= app.menu_scroll + visible {
            app.menu_scroll = app.menu_selected + 1 - visible;
        }
        let max_scroll = n.saturating_sub(visible);
        if app.menu_scroll > max_scroll {
            app.menu_scroll = max_scroll;
        }
    }

    let scroll = app.menu_scroll;
    let has_more_above = scroll > 0;
    let has_more_below = scroll + visible < n;

    let rows: Vec<Row<'_>> = app
        .menu_items
        .iter()
        .enumerate()
        .skip(scroll)
        .take(visible)
        .map(|(i, item)| {
            if item.is_separator {
                return Row::new(vec![
                    Cell::from(""),
                    Cell::from("─────").style(Style::default().fg(t.border_unfocused)),
                ]);
            }
            let selected = i == app.menu_selected;
            let style = if selected {
                Style::default()
                    .fg(t.selection_fg)
                    .bg(t.selection_bg)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            Row::new(vec![
                Cell::from(format!(" {} ", item.key)).style(if selected {
                    style
                } else {
                    Style::default().fg(t.help_key)
                }),
                Cell::from(item.label.clone()).style(style),
            ])
        })
        .collect();

    let table = Table::new(rows, [Constraint::Length(4), Constraint::Fill(1)])
        .block(Block::default().borders(Borders::NONE));
    frame.render_widget(table, inner);

    // Overflow indicators: ▲ top-right when scrolled down, ▼ bottom-right when more below.
    if has_more_above && inner.width > 2 {
        let ind = Rect {
            x: inner.x + inner.width - 2,
            y: inner.y,
            width: 2,
            height: 1,
        };
        frame.render_widget(
            Paragraph::new(Span::styled("▲ ", Style::default().fg(t.border_focused))),
            ind,
        );
    }
    if has_more_below && inner.height > 0 && inner.width > 2 {
        let ind = Rect {
            x: inner.x + inner.width - 2,
            y: inner.y + inner.height - 1,
            width: 2,
            height: 1,
        };
        frame.render_widget(
            Paragraph::new(Span::styled("▼ ", Style::default().fg(t.border_focused))),
            ind,
        );
    }
}

// ── Branch select popup ───────────────────────────────────────────────────

fn draw_branch_select(frame: &mut Frame, app: &App, delete_mode: bool) {
    let t = app.theme();
    let title = if delete_mode {
        " Delete Branch — select branch ".to_string()
    } else {
        " Checkout Branch — select branch ".to_string()
    };
    let height = (app.branch_items.len() as u16 + 4)
        .clamp(BRANCH_SELECT_MIN_HEIGHT, BRANCH_SELECT_MAX_HEIGHT)
        .min(frame.area().height);
    let area = centered_rect(BRANCH_SELECT_WIDTH_PCT, height, frame.area());
    frame.render_widget(Clear, area);

    let border_color = if delete_mode {
        t.popup_border_danger
    } else {
        t.popup_border
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(Style::default().fg(border_color));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if app.branch_items.is_empty() {
        frame.render_widget(
            Paragraph::new(Span::styled(
                "no other branches available",
                Style::default().fg(t.popup_empty),
            )),
            inner,
        );
        return;
    }

    let visible = inner.height as usize;
    let start = if app.branch_selected >= visible {
        app.branch_selected + 1 - visible
    } else {
        0
    };

    let rows: Vec<Row<'_>> = app
        .branch_items
        .iter()
        .enumerate()
        .skip(start)
        .take(visible)
        .map(|(i, item)| {
            let selected = i == app.branch_selected;
            let base = if selected {
                Style::default()
                    .fg(t.selection_fg)
                    .bg(t.selection_bg)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            let tag = if item.is_remote {
                Span::styled(" remote ", Style::default().fg(t.branch_remote))
            } else {
                Span::styled(" local  ", Style::default().fg(t.branch_local))
            };
            Row::new(vec![
                Cell::from(Line::from(vec![tag])),
                Cell::from(item.name.clone()).style(base),
            ])
        })
        .collect();

    let table = Table::new(rows, [Constraint::Length(9), Constraint::Fill(1)])
        .block(Block::default().borders(Borders::NONE));
    frame.render_widget(table, inner);
}

// ── Commit message input popup ─────────────────────────────────────────────

fn draw_commit_message_input(frame: &mut Frame, app: &App) {
    let t = app.theme();
    let staged = app.staged_file_count();
    let title = if app.commit_is_amend {
        format!(
            " Amend Commit ({staged} staged + {} from HEAD) ",
            app.commit_head_file_count
        )
    } else {
        format!(" Commit ({staged} staged) ")
    };

    let height = 14_u16.min(frame.area().height.saturating_sub(HEADER_HEIGHT + 2));
    let area = centered_rect(BRANCH_SELECT_WIDTH_PCT, height, frame.area());
    frame.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(Style::default().fg(t.popup_border));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // "Commit message:" label
            Constraint::Min(2),    // text area
            Constraint::Length(1), // key hints
        ])
        .split(inner);

    frame.render_widget(Paragraph::new("Commit message:"), chunks[0]);

    // Split message into display lines and append cursor to the last one.
    let text_area_height = chunks[1].height as usize;
    let raw_lines: Vec<&str> = app.commit_message.split('\n').collect();
    let total_lines = raw_lines.len();
    // Scroll so the cursor (last line) is always visible.
    let scroll_offset = total_lines.saturating_sub(text_area_height);
    let visible: Vec<Line> = raw_lines
        .iter()
        .enumerate()
        .skip(scroll_offset)
        .take(text_area_height)
        .map(|(i, line)| {
            if i == total_lines - 1 {
                Line::from(Span::styled(
                    format!("{line}▍"),
                    Style::default().fg(t.input_text),
                ))
            } else {
                Line::from(Span::styled(
                    line.to_string(),
                    Style::default().fg(t.input_text),
                ))
            }
        })
        .collect();
    frame.render_widget(Paragraph::new(visible), chunks[1]);

    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("Enter", Style::default().fg(t.popup_confirm)),
            Span::raw(" confirm    "),
            Span::styled("Shift-↵ / Alt-↵", Style::default().fg(t.popup_confirm)),
            Span::raw(" newline    "),
            Span::styled("Esc", Style::default().fg(t.popup_cancel)),
            Span::raw(" cancel"),
        ])),
        chunks[2],
    );
}

// ── New branch input popup ─────────────────────────────────────────────────

fn draw_new_branch_input(frame: &mut Frame, app: &App) {
    let t = app.theme();
    let has_base = !app.branch_input_base.is_empty();
    let height = if has_base { 9 } else { 7 };
    let area = centered_rect(BRANCH_SELECT_WIDTH_PCT, height, frame.area());
    frame.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Create New Branch ")
        .border_style(Style::default().fg(t.popup_border));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut constraints = vec![
        Constraint::Length(1), // "Branch name:" label
        Constraint::Length(1), // text input
        Constraint::Min(0),    // spacer
    ];
    if has_base {
        constraints.push(Constraint::Length(1)); // "from: <base>" line
        constraints.push(Constraint::Length(1)); // blank separator
    }
    constraints.push(Constraint::Length(1)); // key hints

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(inner);

    frame.render_widget(Paragraph::new("Branch name:"), chunks[0]);
    let display = format!("{}▍", app.branch_input);
    frame.render_widget(
        Paragraph::new(Span::styled(display, Style::default().fg(t.input_text))),
        chunks[1],
    );

    let hint_idx = if has_base {
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("from: ", Style::default().fg(t.placeholder)),
                Span::styled(
                    app.branch_input_base.clone(),
                    Style::default().fg(t.input_text),
                ),
            ])),
            chunks[3],
        );
        5
    } else {
        3
    };

    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("Enter", Style::default().fg(t.popup_confirm)),
            Span::raw(" confirm    "),
            Span::styled("Esc", Style::default().fg(t.popup_cancel)),
            Span::raw(" cancel"),
        ])),
        chunks[hint_idx],
    );
}

// ── Confirm force-push popup ───────────────────────────────────────────────

fn draw_confirm_force_push(frame: &mut Frame, app: &App) {
    let t = app.theme();
    let target = app
        .repos
        .get(app.selected)
        .map(|r| r.path.clone())
        .unwrap_or_default();
    let area = centered_rect(60, 7, frame.area());
    frame.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Force Push? ")
        .border_style(Style::default().fg(t.popup_border_danger))
        .title_style(
            Style::default()
                .fg(t.popup_border_danger)
                .add_modifier(Modifier::BOLD),
        );
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(inner);

    frame.render_widget(Paragraph::new("Force-push current branch?"), chunks[0]);
    frame.render_widget(
        Paragraph::new(Span::styled(target, Style::default().fg(t.popup_target))),
        chunks[1],
    );
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("y/Enter", Style::default().fg(t.popup_confirm_danger)),
            Span::raw(" confirm    "),
            Span::styled("n/Esc", Style::default().fg(t.popup_cancel)),
            Span::raw(" cancel"),
        ])),
        chunks[3],
    );
}

fn draw_confirm_force_push_branch(frame: &mut Frame, app: &App) {
    let t = app.theme();
    let branch = &app.branch_to_force_push;
    let area = centered_rect(60, 7, frame.area());
    frame.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Force Push Branch? ")
        .border_style(Style::default().fg(t.popup_border_danger))
        .title_style(
            Style::default()
                .fg(t.popup_border_danger)
                .add_modifier(Modifier::BOLD),
        );
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(inner);

    frame.render_widget(Paragraph::new("Force-push branch to origin?"), chunks[0]);
    frame.render_widget(
        Paragraph::new(Span::styled(
            branch.clone(),
            Style::default().fg(t.popup_target),
        )),
        chunks[1],
    );
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("y/Enter", Style::default().fg(t.popup_confirm_danger)),
            Span::raw(" confirm    "),
            Span::styled("n/Esc", Style::default().fg(t.popup_cancel)),
            Span::raw(" cancel"),
        ])),
        chunks[3],
    );
}

/// Draw a transient popup message (toast notification) that auto-dismisses.
fn draw_popup_message(frame: &mut Frame, app: &mut App) {
    let Some(ref msg) = app.popup_message else {
        return;
    };
    let t = app.theme();
    let area = frame.area();

    // Calculate popup size based on message content
    let msg_len = msg.len() as u16;
    let width = (msg_len + 4).clamp(30, area.width.saturating_sub(4));
    let height: u16 = 5; // Fixed height for the popup box

    let popup = centered_rect((width * 100 / area.width).max(30), height, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Notification ")
        .border_style(Style::default().fg(t.popup_border));
    let inner = block.inner(popup);
    frame.render_widget(Clear, popup);
    frame.render_widget(block, popup);

    // Center the message text vertically and horizontally
    let text_area = Rect {
        x: inner.x,
        y: inner.y + inner.height / 2 - 1,
        width: inner.width,
        height: 3,
    };

    let para = Paragraph::new(msg.clone())
        .style(Style::default().fg(t.popup_target))
        .alignment(ratatui::layout::Alignment::Center);
    frame.render_widget(para, text_area);
}

/// Like `centered_rect` but anchors the popup's top edge at `top_offset` rows from `area.y`
/// instead of deriving the y position from the popup height.
pub fn top_centered_rect(percent_x: u16, height: u16, top_offset: u16, area: Rect) -> Rect {
    let popup_width = area.width * percent_x / 100;
    let x = area.x + (area.width.saturating_sub(popup_width)) / 2;
    let y = area.y + top_offset;
    let max_height = area.height.saturating_sub(top_offset);
    Rect {
        x,
        y,
        width: popup_width,
        height: height.min(max_height),
    }
}

pub fn centered_rect(percent_x: u16, height: u16, area: Rect) -> Rect {
    let popup_width = area.width * percent_x / 100;
    let x = area.x + (area.width.saturating_sub(popup_width)) / 2;
    let y = area.y + area.height.saturating_sub(height) / 3;
    Rect {
        x,
        y,
        width: popup_width,
        height: height.min(area.height),
    }
}

/// Return a border style that highlights when the pane is focused.
/// Uses theme colors: border_focused (bright) when focused, border_unfocused when not.
fn focus_border_style(focused: bool, theme: &crate::theme::Theme) -> Style {
    if focused {
        Style::default()
            .fg(theme.border_focused)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.border_unfocused)
    }
}
