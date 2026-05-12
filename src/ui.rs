use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Clear, FrameExt as _, Paragraph, Row, Table, TableState},
    Frame,
};

use crate::app::{App, AppMode, Focus, RepoOperation};
use crate::git::{FileStatusKind, RepoStatus};

pub fn draw(frame: &mut Frame, app: &mut App) {
    // Build a vertical layout: header / table / optional detail / optional log / help bar.
    let mut constraints: Vec<Constraint> = Vec::new();
    constraints.push(Constraint::Length(3)); // header
    constraints.push(Constraint::Min(5)); // table
    if app.show_detail {
        constraints.push(Constraint::Length(detail_panel_height(app)));
    }
    if app.show_log {
        constraints.push(Constraint::Length(log_panel_height(app)));
    }
    constraints.push(Constraint::Length(1)); // help bar

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(frame.area());

    let mut idx = 0;
    draw_header(frame, chunks[idx], app);
    idx += 1;
    draw_repo_table(frame, chunks[idx], app);
    idx += 1;
    if app.show_detail {
        draw_detail_panel(frame, chunks[idx], app);
        idx += 1;
    }
    if app.show_log {
        draw_log_panel(frame, chunks[idx], app);
        idx += 1;
    }
    draw_help_bar(frame, chunks[idx], app);

    if app.mode == AppMode::FilePicker {
        draw_file_picker(frame, app);
    }
    if app.mode == AppMode::ConfirmRemove {
        draw_confirm_remove(frame, app);
    }
    if app.mode == AppMode::ActionMenu {
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
}

/// Header — shows app title, spinner when scanning, and refresh time right-aligned.
fn draw_header(frame: &mut Frame, area: Rect, app: &App) {
    let refresh_text = match app.seconds_since_refresh() {
        Some(s) if s < 5 => "just now".to_string(),
        Some(s) => format!("{s}s ago"),
        None => "never".to_string(),
    };

    // Left side: title + optional scanning spinner.
    let mut left_spans: Vec<Span<'static>> = vec![Span::styled(
        "Gitover - Git Repository Monitor",
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )];

    if app.scanning {
        left_spans.push(Span::raw("  "));
        left_spans.push(Span::styled(
            app.spinner_frame(),
            Style::default().fg(Color::Yellow),
        ));
        left_spans.push(Span::raw(" "));
        left_spans.push(Span::styled(
            "scanning…",
            Style::default().fg(Color::Yellow),
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

    // Right paragraph (refresh indicator) — right-aligned
    let right_text = format!("refreshed: {}  ", refresh_text);
    let right_para = Paragraph::new(Line::from(Span::styled(
        right_text,
        Style::default().fg(Color::DarkGray),
    )))
    .alignment(ratatui::layout::Alignment::Right);
    frame.render_widget(right_para, inner);
}

fn draw_repo_table(frame: &mut Frame, area: Rect, app: &mut App) {
    // Scrolling: clamp before borrowing app.repos so the offset is correct
    // when ratatui renders the visible window.
    let visible_rows = table_visible_rows(area);
    clamp_offset(app, visible_rows);

    let header_cells = [
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
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
    });
    let table_header = Row::new(header_cells).height(1).bottom_margin(1);

    let spinner = app.spinner_frame().to_string();

    let rows = app.repos.iter().map(|repo| {
        if let Some(err) = &repo.error {
            return build_error_row(repo, err);
        }

        let name = repo.path.split('/').next_back().unwrap_or(&repo.path);
        let name_style = if repo.is_clean() {
            Style::default().fg(Color::Green)
        } else {
            Style::default().fg(Color::White)
        };

        let status_spans = build_status_spans(repo);
        let upstream_cell = build_ahead_behind_cell(&repo.upstream);
        let trunk_cell = build_trunk_cell(&repo.trunk);
        let activity_cell = build_activity_cell(app.repo_operation(&repo.path), &spinner);

        Row::new(vec![
            Cell::from(name).style(name_style),
            Cell::from(repo.branch.as_str()).style(Style::default().fg(Color::Cyan)),
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
            .border_style(focus_border_style(app.focus == Focus::Repos)),
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
}

/// How many data rows fit in the table area (subtract borders + header).
fn table_visible_rows(area: Rect) -> usize {
    // 2 lines for top/bottom border, 1 for header row, 1 for the header
    // bottom_margin spacer.
    let h = area.height as i32 - 2 - 1 - 1;
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
fn build_error_row(repo: &RepoStatus, err: &str) -> Row<'static> {
    let name = repo.path.split('/').next_back().unwrap_or(&repo.path);
    Row::new(vec![
        Cell::from(name.to_string()).style(Style::default().fg(Color::Red)),
        Cell::from("—").style(Style::default().fg(Color::DarkGray)),
        Cell::from(Span::styled(
            format!("⚠ {err}"),
            Style::default().fg(Color::Red),
        )),
        Cell::from("—").style(Style::default().fg(Color::DarkGray)),
        Cell::from("—").style(Style::default().fg(Color::DarkGray)),
        Cell::from("—").style(Style::default().fg(Color::DarkGray)),
    ])
}

fn build_status_spans(repo: &RepoStatus) -> Line<'static> {
    let parts: &[(usize, &str, Color)] = &[
        (repo.staged, "S", Color::Blue),
        (repo.conflict, "C", Color::Yellow),
        (repo.modified, "M", Color::Green),
        (repo.deleted, "D", Color::Red),
        (repo.added, "U", Color::Gray),
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
        Line::from(Span::styled("clean", Style::default().fg(Color::DarkGray)))
    } else {
        Line::from(spans)
    }
}

fn build_ahead_behind_cell(ab: &Option<crate::git::AheadBehind>) -> Cell<'static> {
    match ab {
        None => Cell::from("-").style(Style::default().fg(Color::DarkGray)),
        Some(ab) => {
            let text = format!("↑{} ↓{} {}", ab.ahead, ab.behind, ab.branch);
            let style = if ab.ahead > 0 || ab.behind > 0 {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            Cell::from(text).style(style)
        }
    }
}

fn build_trunk_cell(ab: &Option<crate::git::AheadBehind>) -> Cell<'static> {
    match ab {
        None => Cell::from("-").style(Style::default().fg(Color::DarkGray)),
        Some(ab) => {
            let text = format!("↑{} ↓{} {}", ab.ahead, ab.behind, ab.branch);
            let style = if ab.behind > 0 {
                Style::default().fg(Color::Red)
            } else if ab.ahead > 0 {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            Cell::from(text).style(style)
        }
    }
}

/// Per-repo busy indicator + op name. Empty cell when idle.
fn build_activity_cell(op: Option<RepoOperation>, spinner: &str) -> Cell<'static> {
    match op {
        None => Cell::from(""),
        Some(op) => Cell::from(Line::from(vec![
            Span::styled(spinner.to_string(), Style::default().fg(Color::Yellow)),
            Span::raw(" "),
            Span::styled(op.label(), Style::default().fg(Color::Yellow)),
        ])),
    }
}

// ── Detail panel ──────────────────────────────────────────────────────────

/// Decide how tall the detail panel should be (clamped to a sensible range).
fn detail_panel_height(app: &App) -> u16 {
    let count = app.selected_files().len();
    // 2 border lines + 1 header line + N file lines, clamp 5..=15
    let h = 2 + 1 + count as u16;
    h.clamp(5, 15)
}

fn draw_detail_panel(frame: &mut Frame, area: Rect, app: &mut App) {
    let title = match app.repos.get(app.selected) {
        Some(r) => format!(" Status Details — {} ", r.path),
        None => " Status Details ".to_string(),
    };

    let focused = app.focus == Focus::Detail;
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(focus_border_style(focused));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Clamp scroll before borrowing files (avoids borrow conflict).
    // We use `inner.height` which is already computed above.
    let visible = inner.height as usize;
    {
        let file_count = app.selected_files().len();
        if visible > 0 && file_count > 0 {
            if app.detail_selected < app.detail_scroll {
                app.detail_scroll = app.detail_selected;
            } else if app.detail_selected >= app.detail_scroll + visible {
                app.detail_scroll = app.detail_selected + 1 - visible;
            }
            let max_scroll = file_count.saturating_sub(visible);
            if app.detail_scroll > max_scroll {
                app.detail_scroll = max_scroll;
            }
        }
    }

    let files = app.selected_files();

    if files.is_empty() {
        let msg = Paragraph::new(Line::from(Span::styled(
            "no changes — working tree clean",
            Style::default().fg(Color::DarkGray),
        )));
        frame.render_widget(msg, inner);
        return;
    }

    let lines: Vec<Line<'static>> = files
        .iter()
        .enumerate()
        .skip(app.detail_scroll)
        .take(visible)
        .map(|(i, f)| {
            let colour = file_status_colour(&f.status);
            let selected = focused && i == app.detail_selected;
            let base_style = if selected {
                Style::default()
                    .fg(Color::Black)
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

fn file_status_colour(kind: &FileStatusKind) -> Color {
    match kind {
        FileStatusKind::Staged => Color::Blue,
        FileStatusKind::Modified => Color::Green,
        FileStatusKind::Deleted => Color::Red,
        FileStatusKind::Conflict => Color::Yellow,
        FileStatusKind::Untracked => Color::Gray,
    }
}

// ── Log panel ─────────────────────────────────────────────────────────────

fn log_panel_height(app: &App) -> u16 {
    let n = app.log.len() as u16;
    // 2 borders + at least 3 lines, at most 10
    (2 + n.min(8)).clamp(5, 10)
}

fn draw_log_panel(frame: &mut Frame, area: Rect, app: &mut App) {
    let focused = app.focus == Focus::Log;
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Output Log ")
        .border_style(focus_border_style(focused));
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
                    Style::default().fg(Color::DarkGray),
                ),
                Span::raw(l.text.clone()),
            ])
        })
        .collect();
    frame.render_widget(Paragraph::new(lines), inner);
}

// ── Help bar ──────────────────────────────────────────────────────────────

fn draw_help_bar(frame: &mut Frame, area: Rect, _app: &App) {
    let help = Line::from(vec![
        Span::styled("Q", Style::default().fg(Color::Yellow)),
        Span::raw(" quit  "),
        Span::styled("Tab", Style::default().fg(Color::Yellow)),
        Span::raw(" focus  "),
        Span::styled("j/k ↑↓", Style::default().fg(Color::Yellow)),
        Span::raw(" nav  "),
        Span::styled("PgUp/Dn", Style::default().fg(Color::Yellow)),
        Span::raw(" fast  "),
        Span::styled("A", Style::default().fg(Color::Yellow)),
        Span::raw(" add  "),
        Span::styled("D", Style::default().fg(Color::Yellow)),
        Span::raw(" remove  "),
        Span::styled("s", Style::default().fg(Color::Yellow)),
        Span::raw(" status  "),
        Span::styled("l", Style::default().fg(Color::Yellow)),
        Span::raw(" log  "),
        Span::styled("Enter", Style::default().fg(Color::Yellow)),
        Span::raw(" actions  "),
        Span::styled("r", Style::default().fg(Color::Yellow)),
        Span::raw(" refresh  "),
        Span::styled("Alt-f", Style::default().fg(Color::Yellow)),
        Span::raw(" fetch all"),
    ]);
    frame.render_widget(Paragraph::new(help), area);
}

// ── Popups ────────────────────────────────────────────────────────────────

fn draw_file_picker(frame: &mut Frame, app: &App) {
    let Some(explorer) = &app.file_explorer else {
        return;
    };

    let area = centered_rect(65, 22, frame.area());

    // Outer wrapper with title and keybinding hint
    let cwd = explorer.cwd().to_string_lossy().to_string();
    let title = format!(" Add Repository — {cwd} ");
    let outer = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .title_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .border_style(Style::default().fg(Color::Cyan));

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
        Span::styled("↑↓/jk", Style::default().fg(Color::Yellow)),
        Span::raw(" navigate  "),
        Span::styled("Enter/→/l", Style::default().fg(Color::Yellow)),
        Span::raw(" open dir  "),
        Span::styled("←/h/Bksp", Style::default().fg(Color::Yellow)),
        Span::raw(" parent  "),
        Span::styled("Space", Style::default().fg(Color::Green)),
        Span::raw(" select repo  "),
        Span::styled("Esc", Style::default().fg(Color::Yellow)),
        Span::raw(" cancel"),
    ]);
    frame.render_widget(Paragraph::new(hint), inner_chunks[1]);
}

fn draw_confirm_remove(frame: &mut Frame, app: &App) {
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
        .border_style(Style::default().fg(Color::Red))
        .title_style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD));
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
            Style::default().fg(Color::Yellow),
        )])),
        chunks[1],
    );
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("y/Enter", Style::default().fg(Color::Green)),
            Span::raw(" confirm    "),
            Span::styled("n/Esc", Style::default().fg(Color::Yellow)),
            Span::raw(" cancel"),
        ])),
        chunks[3],
    );
}

// ── Action menu popup ─────────────────────────────────────────────────────

fn draw_action_menu(frame: &mut Frame, app: &App) {
    let repo_name = app
        .repos
        .get(app.selected)
        .map(|r| r.path.split('/').next_back().unwrap_or(&r.path).to_string())
        .unwrap_or_default();
    let title = format!(" Actions — {repo_name} ");
    let height = (app.menu_items.len() as u16 + 4).min(frame.area().height);
    let area = centered_rect(40, height, frame.area());
    frame.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(Style::default().fg(Color::Cyan));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let rows: Vec<Row<'_>> = app
        .menu_items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let selected = i == app.menu_selected;
            let style = if selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            Row::new(vec![
                Cell::from(format!(" {} ", item.key)).style(if selected {
                    style
                } else {
                    Style::default().fg(Color::Yellow)
                }),
                Cell::from(item.label.clone()).style(style),
            ])
        })
        .collect();

    let table = Table::new(rows, [Constraint::Length(4), Constraint::Fill(1)])
        .block(Block::default().borders(Borders::NONE));
    frame.render_widget(table, inner);
}

// ── Branch select popup ───────────────────────────────────────────────────

fn draw_branch_select(frame: &mut Frame, app: &App, delete_mode: bool) {
    let title = if delete_mode {
        " Delete Branch — select branch ".to_string()
    } else {
        " Checkout Branch — select branch ".to_string()
    };
    let height = (app.branch_items.len() as u16 + 4)
        .clamp(5, 20)
        .min(frame.area().height);
    let area = centered_rect(55, height, frame.area());
    frame.render_widget(Clear, area);

    let border_color = if delete_mode { Color::Red } else { Color::Cyan };
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
                Style::default().fg(Color::DarkGray),
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
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            let tag = if item.is_remote {
                Span::styled(" remote ", Style::default().fg(Color::Yellow))
            } else {
                Span::styled(" local  ", Style::default().fg(Color::DarkGray))
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
    let area = centered_rect(55, 7, frame.area());
    frame.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Create New Branch ")
        .border_style(Style::default().fg(Color::Cyan));
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
        Paragraph::new(Span::styled(display, Style::default().fg(Color::Cyan))),
        chunks[1],
    );
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("Enter", Style::default().fg(Color::Green)),
            Span::raw(" confirm    "),
            Span::styled("Esc", Style::default().fg(Color::Yellow)),
            Span::raw(" cancel"),
        ])),
        chunks[3],
    );
}

// ── Confirm force-push popup ───────────────────────────────────────────────

fn draw_confirm_force_push(frame: &mut Frame, app: &App) {
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
        .border_style(Style::default().fg(Color::Red))
        .title_style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD));
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
        Paragraph::new(Span::styled(target, Style::default().fg(Color::Yellow))),
        chunks[1],
    );
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("y/Enter", Style::default().fg(Color::Red)),
            Span::raw(" confirm    "),
            Span::styled("n/Esc", Style::default().fg(Color::Yellow)),
            Span::raw(" cancel"),
        ])),
        chunks[3],
    );
}

fn centered_rect(percent_x: u16, height: u16, area: Rect) -> Rect {
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
/// Focused: bright Cyan. Unfocused: DarkGray.
fn focus_border_style(focused: bool) -> Style {
    if focused {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    }
}
