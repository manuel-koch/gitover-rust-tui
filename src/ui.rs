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

use crate::app::{App, AppMode, Focus, RepoOperation};
use crate::git::RepoStatus;

/// Popup width (percent of terminal width) for the repo/log action menu.
pub const ACTION_MENU_WIDTH_PCT: u16 = 60;
/// Popup width (percent of terminal width) for the file action menu.
pub const FILE_ACTION_MENU_WIDTH_PCT: u16 = 80;
/// Height (rows) of the header panel — used for layout and popup positioning.
pub const HEADER_HEIGHT: u16 = 3;
/// Height (rows) of the help bar at the bottom of the screen.
const HELP_BAR_HEIGHT: u16 = 1;
/// Total rows consumed by fixed panels (header + help bar).
const FIXED_PANE_HEIGHT: u16 = HEADER_HEIGHT + HELP_BAR_HEIGHT;
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
    pub file_status: Option<Rect>,
    pub history: Option<Rect>,
    pub log: Option<Rect>,
    pub help_bar: Rect,
}

/// Compute the layout rectangles for all visible panes.
/// This mirrors the layout logic in `draw()` so main.rs can use these
/// rectangles for mouse-click focus detection.
pub fn pane_areas(app: &App, total: Rect) -> PaneAreas {
    let total_available = total.height.saturating_sub(FIXED_PANE_HEIGHT);

    let open_panes = [app.show_file_status, app.show_history, app.show_log]
        .into_iter()
        .filter(|&p| p)
        .count();

    let base_share = total_available / (open_panes as u16 + 1);
    let remainder = total_available % (open_panes as u16 + 1);
    let repo_height = base_share + remainder;
    let pane_height = base_share;

    let mut constraints: Vec<Constraint> = Vec::new();
    constraints.push(Constraint::Length(HEADER_HEIGHT));
    constraints.push(Constraint::Length(repo_height));

    if app.show_file_status {
        constraints.push(Constraint::Length(pane_height));
    }
    if app.show_history {
        constraints.push(Constraint::Length(pane_height));
    }
    if app.show_log {
        constraints.push(Constraint::Length(pane_height));
    }
    constraints.push(Constraint::Length(HELP_BAR_HEIGHT));

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(total);

    let mut idx = 0;
    let header = chunks[idx];
    idx += 1;
    let repos = chunks[idx];
    idx += 1;

    let mut file_status = None;
    if app.show_file_status {
        file_status = Some(chunks[idx]);
        idx += 1;
    }

    let mut history = None;
    if app.show_history {
        history = Some(chunks[idx]);
        idx += 1;
    }

    let mut log = None;
    if app.show_log {
        log = Some(chunks[idx]);
        idx += 1;
    }

    let help_bar = chunks[idx];

    PaneAreas {
        terminal: total,
        header,
        repos,
        file_status,
        history,
        log,
        help_bar,
    }
}

pub fn draw(frame: &mut Frame, app: &mut App) {
    // Cache pane areas for mouse click detection
    app.cached_pane_areas = Some(pane_areas(app, frame.area()));

    // Compute remaining vertical space after fixed-height panels.
    let total_available = frame.area().height.saturating_sub(FIXED_PANE_HEIGHT);

    // Count open optional panes (File Status, History, Log).
    let open_panes = [app.show_file_status, app.show_history, app.show_log]
        .into_iter()
        .filter(|&p| p)
        .count();

    // Distribute available space evenly among Repositories + open panes.
    // Any remaining lines go to Repositories so it is always the biggest.
    let base_share = total_available / (open_panes as u16 + 1);
    let remainder = total_available % (open_panes as u16 + 1);
    // Repositories gets `base_share + remainder`; each optional pane gets `base_share`.
    let repo_height = base_share + remainder;
    let pane_height = base_share;

    // Build constraints in fixed order: header / table / file status / history / log / help bar
    let mut constraints: Vec<Constraint> = Vec::new();
    constraints.push(Constraint::Length(HEADER_HEIGHT)); // header
    constraints.push(Constraint::Length(repo_height)); // Repositories table — gets all extra space

    if app.show_file_status {
        constraints.push(Constraint::Length(pane_height));
    }
    if app.show_history {
        constraints.push(Constraint::Length(pane_height));
    }
    if app.show_log {
        constraints.push(Constraint::Length(pane_height));
    }
    constraints.push(Constraint::Length(HELP_BAR_HEIGHT)); // help bar

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(frame.area());

    let mut idx = 0;

    // Always present panes (order fixed)
    draw_header(frame, chunks[idx], app);
    idx += 1;
    draw_repo_table(frame, chunks[idx], app);
    idx += 1;

    // Optional panes in fixed order: File Status -> History -> Log
    if app.show_file_status {
        draw_file_status_panel(frame, chunks[idx], app);
        idx += 1;
    }
    if app.show_history {
        draw_history_panel(frame, chunks[idx], app);
        idx += 1;
    }
    if app.show_log {
        draw_log_panel(frame, chunks[idx], app);
        idx += 1;
    }

    // Always present help bar
    draw_help_bar(frame, chunks[idx], app);

    if app.mode == AppMode::FilePicker {
        draw_file_picker(frame, app);
    }
    if app.mode == AppMode::ConfirmRemove {
        draw_confirm_remove(frame, app);
    }
    if matches!(
        app.mode,
        AppMode::ActionMenu | AppMode::LogActionMenu | AppMode::FileActionMenu
    ) {
        draw_action_menu(frame, app);
    }
    if app.mode == AppMode::BranchSelect {
        draw_branch_select(frame, app, false);
    }
    if app.mode == AppMode::NewBranchInput {
        draw_new_branch_input(frame, app);
    }
    if app.mode == AppMode::ConfirmForcePush {
        draw_confirm_force_push(frame, app);
    }
    if app.mode == AppMode::ConfirmDeleteBranch {
        draw_branch_select(frame, app, true);
    }
    if app.mode == AppMode::PopupMessage {
        draw_popup_message(frame, app);
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

    // Left side: title + optional scanning spinner.
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

    // Right side: "refreshed: Xs ago" — right-aligned inside the block inner area.
    // We render two paragraphs: one left-aligned, one right-aligned.
    let block = Block::default().borders(Borders::ALL);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Left paragraph (title + spinner)
    let left_para = Paragraph::new(Line::from(left_spans));
    frame.render_widget(left_para, inner);

    // Right paragraph (refresh indicator + auto-fetch hint) — right-aligned
    let right_text = format!("refreshed: {}  ", refresh_text);
    let right_spans = vec![
        Span::styled(right_text, Style::default().fg(theme.refresh_info)),
        Span::styled(&auto_fetch_info, Style::default().fg(theme.auto_fetch_info)),
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
            .border_style(focus_border_style(app.focus == Focus::Repos, app.theme())),
    )
    .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED))
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
    let inner_x = area.x + 1;
    let inner_y = area.y + 1;
    let inner_w = area.width.saturating_sub(2);
    let inner_h = area.height.saturating_sub(2);
    if has_more_above && inner_w > 2 {
        frame.render_widget(
            Paragraph::new(Span::styled("▲ ", Style::default().fg(t.help_key))),
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
            Paragraph::new(Span::styled("▼ ", Style::default().fg(t.help_key))),
            Rect {
                x: inner_x + inner_w - 2,
                y: inner_y + inner_h - 1,
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
        let modifier = if i == app.selected {
            Modifier::REVERSED
        } else {
            Modifier::empty()
        };

        let line = Line::from(vec![
            Span::styled(
                "⚠ ",
                Style::default()
                    .fg(theme.placeholder)
                    .add_modifier(modifier),
            ),
            Span::styled(
                err.to_string(),
                Style::default().fg(theme.error).add_modifier(modifier),
            ),
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

    let lines: Vec<Line<'static>> = files
        .iter()
        .enumerate()
        .skip(app.file_status_scroll)
        .take(visible)
        .map(|(i, f)| {
            let colour = theme.file_status_colour(&f.status);
            let selected = focused && i == app.file_status_selected;
            let base_style = if selected {
                Style::default()
                    .fg(theme.selection_fg)
                    .bg(colour)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            let code_style = if selected {
                base_style
            } else {
                Style::default().fg(colour).add_modifier(Modifier::BOLD)
            };
            Line::from(vec![
                Span::styled(format!(" {} ", f.status.code()), code_style),
                Span::raw(" "),
                Span::styled(f.path.clone(), base_style),
            ])
        })
        .collect();

    let para = Paragraph::new(lines);
    frame.render_widget(para, inner);
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
            Line::from(vec![
                Span::styled(
                    format!("[{}] ", l.timestamp),
                    Style::default().fg(theme.log_timestamp),
                ),
                Span::raw(l.text.clone()),
            ])
        })
        .collect();
    frame.render_widget(Paragraph::new(lines), inner);
}

// ── Help bar ──────────────────────────────────────────────────────────────

fn draw_history_panel(frame: &mut Frame, area: Rect, app: &mut App) {
    let t = app.theme();
    let focused = app.focus == Focus::History;
    let filter_label = app.history_filter.label();
    let title = if filter_label.is_empty() {
        " Commit History ".to_string()
    } else {
        format!(" Commit History ({filter_label}) ")
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

    // Column widths — distribute proportionally.
    // hash:10  timestamp:20  author:dynamic  summary:rest
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
    let sep = 2usize; // " │ " between cols is 3 chars; account for 3 separators
    let summary_w = total.saturating_sub(hash_w + ts_w + author_w + sep * 3 + 4);

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
                    .fg(t.selection_fg)
                    .bg(t.selection_bg)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            let hash_cell =
                Cell::from(commit.short_hash.clone()).style(Style::default().fg(t.history_hash));
            let ts_cell = Cell::from(commit.timestamp.clone())
                .style(Style::default().fg(t.history_timestamp));
            let author_cell = Cell::from(commit.author.chars().take(author_w).collect::<String>())
                .style(Style::default().fg(t.history_author));
            let summary_cell =
                Cell::from(commit.summary.chars().take(summary_w).collect::<String>());
            let row =
                Row::new(vec![hash_cell, ts_cell, author_cell, summary_cell]).style(row_style);
            rows.push(row);
            if rows.len() >= visible {
                break 'outer;
            }
        }
        flat_idx += 1;

        // File sub-rows
        for file_delta in &commit.files {
            if flat_idx >= app.history_scroll {
                let selected = flat_idx == app.history_selected;
                let row_style = if selected {
                    Style::default().fg(t.selection_fg).bg(t.selection_bg)
                } else {
                    Style::default()
                };
                // empty hash / ts / author; file info in summary column
                let file_text = format!("  {} {}", file_delta.kind.code(), file_delta.path);
                let file_span = Span::styled(
                    file_text
                        .chars()
                        .take(summary_w + author_w + ts_w)
                        .collect::<String>(),
                    Style::default().fg(t.delta_colour(&file_delta.kind)),
                );
                let row = Row::new(vec![
                    Cell::from(""),
                    Cell::from(""),
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

    // Scroll indicator: show commit N of M (not flat row index).
    let commit_idx = app
        .history_row_at(app.history_selected)
        .map(|(ci, _)| ci + 1)
        .unwrap_or(1)
        .min(app.history.len());
    let scroll_info = format!("{}/{}", commit_idx, app.history.len());
    let info_width = scroll_info.len() as u16 + 2;
    if inner.width > info_width {
        let info_area = Rect {
            x: inner.x + inner.width - info_width,
            y: inner.y,
            width: info_width,
            height: 1,
        };
        frame.render_widget(
            Paragraph::new(scroll_info).style(Style::default().fg(t.history_scroll_info)),
            info_area,
        );
    }

    let widths = [
        Constraint::Length(hash_w as u16),
        Constraint::Length(ts_w as u16),
        Constraint::Length(author_w as u16),
        Constraint::Min(4),
    ];
    let table = Table::new(rows, widths).column_spacing(1);
    frame.render_widget(table, inner);
}

fn draw_help_bar(frame: &mut Frame, area: Rect, app: &App) {
    let t = app.theme();
    let mut spans: Vec<Span<'static>> = Vec::new();

    // Navigation hints — shown only when the full help text fits.
    // Built first so we can measure both sections together.
    let nav: [Span<'static>; 6] = [
        Span::styled("Tab", Style::default().fg(t.help_key)),
        Span::raw(" focus  "),
        Span::styled("↑↓", Style::default().fg(t.help_key)),
        Span::raw(" nav  "),
        Span::styled("PgUp/Dn", Style::default().fg(t.help_key)),
        Span::raw(" fast  "),
    ];
    let nav_width: usize = nav.iter().map(|s| s.width()).sum();

    // Action keys in grouped order: A, D, r, Alt-f, s, h, l, c, Enter
    let actions: [Span<'static>; 18] = [
        Span::styled("A", Style::default().fg(t.help_key)),
        Span::raw(" add  "),
        Span::styled("D", Style::default().fg(t.help_key)),
        Span::raw(" remove  "),
        Span::styled("r", Style::default().fg(t.help_key)),
        Span::raw(" refresh  "),
        Span::styled("Alt-f", Style::default().fg(t.help_key)),
        Span::raw(" fetch all  "),
        Span::styled("s", Style::default().fg(t.help_key)),
        Span::raw(" status  "),
        Span::styled("h", Style::default().fg(t.help_key)),
        Span::raw(" history  "),
        Span::styled("l", Style::default().fg(t.help_key)),
        Span::raw(" log  "),
        Span::styled("c", Style::default().fg(t.help_key)),
        Span::raw(" checkout  "),
        Span::styled("Enter", Style::default().fg(t.help_key)),
        Span::raw(" actions"),
    ];
    let actions_width: usize = actions.iter().map(|s| s.width()).sum();

    // Only include nav hints if everything fits without clipping
    if (nav_width + actions_width) as u16 <= area.width {
        spans.extend(nav);
    }
    spans.extend(actions);

    let help = Line::from(spans);
    frame.render_widget(Paragraph::new(help), area);
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

    // Keybinding hint bar
    let hint = Line::from(vec![
        Span::styled("↑↓/jk", Style::default().fg(t.help_key)),
        Span::raw(" navigate  "),
        Span::styled("Enter/→/l", Style::default().fg(t.help_key)),
        Span::raw(" open dir  "),
        Span::styled("←/h/Bksp", Style::default().fg(t.help_key)),
        Span::raw(" parent  "),
        Span::styled("Space", Style::default().fg(t.help_key_confirm)),
        Span::raw(" select repo  "),
        Span::styled("Esc", Style::default().fg(t.help_key)),
        Span::raw(" cancel"),
    ]);
    frame.render_widget(Paragraph::new(hint), inner_chunks[1]);
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

// ── Action menu popup ─────────────────────────────────────────────────────

fn draw_action_menu(frame: &mut Frame, app: &mut App) {
    let t = app.theme();
    let title = match app.mode {
        AppMode::LogActionMenu => " Output Log ".to_string(),
        AppMode::FileActionMenu => {
            let file_name = app
                .selected_files()
                .get(app.file_status_selected)
                .map(|f| f.path.split('/').next_back().unwrap_or(&f.path).to_string())
                .unwrap_or_default();
            format!(" File Actions — {file_name} ")
        }
        _ => {
            let repo_name = app
                .repos
                .get(app.selected)
                .map(|r| r.path.split('/').next_back().unwrap_or(&r.path).to_string())
                .unwrap_or_default();
            format!(" Actions — {repo_name} ")
        }
    };
    let width = if app.mode == AppMode::FileActionMenu {
        FILE_ACTION_MENU_WIDTH_PCT
    } else {
        ACTION_MENU_WIDTH_PCT
    };
    let n = app.menu_items.len();
    // Cap height to available screen space; +2 for top/bottom borders.
    let max_height = frame.area().height.saturating_sub(HEADER_HEIGHT);
    let height = ((n as u16 + 2).min(max_height)).max(3);
    let area = top_centered_rect(width, height, HEADER_HEIGHT, frame.area());
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
            Paragraph::new(Span::styled("▲ ", Style::default().fg(t.help_key))),
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
            Paragraph::new(Span::styled("▼ ", Style::default().fg(t.help_key))),
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

// ── New branch input popup ─────────────────────────────────────────────────

fn draw_new_branch_input(frame: &mut Frame, app: &App) {
    let t = app.theme();
    let area = centered_rect(BRANCH_SELECT_WIDTH_PCT, 7, frame.area());
    frame.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Create New Branch ")
        .border_style(Style::default().fg(t.popup_border));
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

    frame.render_widget(Paragraph::new("Branch name:"), chunks[0]);
    // Show the input with a trailing cursor
    let display = format!("{}▍", app.branch_input);
    frame.render_widget(
        Paragraph::new(Span::styled(display, Style::default().fg(t.input_text))),
        chunks[1],
    );
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("Enter", Style::default().fg(t.popup_confirm)),
            Span::raw(" confirm    "),
            Span::styled("Esc", Style::default().fg(t.popup_cancel)),
            Span::raw(" cancel"),
        ])),
        chunks[3],
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
