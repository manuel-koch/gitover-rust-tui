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

mod app;
mod config;
mod git;
mod ops;
mod state;
mod theme;
mod ui;
mod watcher;

use anyhow::Result;
use app::{App, AppMode, Focus};
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers, MouseButton,
        MouseEvent, MouseEventKind,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ops::{spawn_op, OpRequest, OpResult};
use ratatui::{backend::CrosstermBackend, Terminal};
use ratatui_explorer::Input as ExplorerInput;
use std::{
    io,
    sync::mpsc::{self, Receiver, TryRecvError},
    time::{Duration, Instant},
};

/// Tick interval for the event loop (UI responsiveness).
const TICK: Duration = Duration::from_millis(200);

/// If a single tick takes longer than this wall-clock gap, the system likely
/// woke from sleep. Trigger a full refresh to pick up changes that happened
/// while sleeping.
const WAKE_THRESHOLD: Duration = Duration::from_secs(3);

fn main() -> Result<()> {
    if std::env::args().any(|a| a == "--version" || a == "-V") {
        println!(
            "gitover v{} (commit {}, built {})",
            env!("CARGO_PKG_VERSION"),
            env!("GIT_SHORT_HASH"),
            env!("BUILD_TIMESTAMP"),
        );
        return Ok(());
    }

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();

    // Load repos from persisted state; fall back to cwd if state is empty
    if app.state.repos.is_empty() {
        let cwd = std::env::current_dir()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        app.state.add_repo(&cwd);
        let _ = app.state.save();
    }

    refresh_repos(&mut app);

    let mut dirty_rx = watcher::start(app.repos.iter().map(|r| r.path.clone()).collect());

    // Channel for background git-operation results.
    let (op_tx, op_rx) = mpsc::channel::<OpResult>();

    let result = run_app(&mut terminal, &mut app, &mut dirty_rx, &op_tx, &op_rx);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(e) = result {
        eprintln!("Error: {e}");
    }

    Ok(())
}

fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    dirty_rx: &mut std::sync::mpsc::Receiver<String>,
    op_tx: &std::sync::mpsc::Sender<OpResult>,
    op_rx: &Receiver<OpResult>,
) -> Result<()>
where
    B::Error: Send + Sync + 'static,
{
    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|f| ui::draw(f, app))?;

        // Drain watcher notifications
        loop {
            match dirty_rx.try_recv() {
                Ok(dirty_path) => refresh_single_repo(app, &dirty_path),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => break,
            }
        }

        // Drain completed git-operation results
        loop {
            match op_rx.try_recv() {
                Ok(result) => handle_op_result(app, dirty_rx, result),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => break,
            }
        }

        // Detect wake-from-sleep
        if last_tick.elapsed() > WAKE_THRESHOLD {
            refresh_repos(app);
        }

        // Check if the automatic background fetch timer has fired.
        if app.is_auto_fetch_due() {
            app.reset_auto_fetch_timer();
            launch_all_fetch(app, op_tx);
        }

        // Check if popup message should auto-dismiss
        if matches!(app.mode, AppMode::PopupMessage) {
            app.check_popup_timeout();
        }
        last_tick = Instant::now();

        app.spinner_tick = app.spinner_tick.wrapping_add(1);

        let timeout = TICK
            .checked_sub(last_tick.elapsed())
            .unwrap_or(Duration::ZERO);

        if event::poll(timeout)? {
            let ev = event::read()?;
            if let Event::Key(key) = &ev {
                if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
                    app.should_quit = true;
                    continue;
                }
            }

            // Handle mouse events (clicks for focus, wheel for scroll)
            if let Event::Mouse(mouse) = &ev {
                handle_mouse_event(app, op_tx, mouse);
            }

            match app.mode {
                AppMode::Normal => {
                    if let Event::Key(key) = &ev {
                        if app.show_history && app.focus == app::Focus::History {
                            handle_history_key(app, op_tx, key.code, key.modifiers);
                        } else {
                            handle_normal_key(app, dirty_rx, op_tx, key.code, key.modifiers);
                        }
                    }
                }
                AppMode::FilePicker => handle_picker_event(app, dirty_rx, &ev),
                AppMode::ConfirmRemove => {
                    if let Event::Key(key) = &ev {
                        handle_confirm_remove_key(app, dirty_rx, key.code);
                    }
                }
                AppMode::ActionMenu => {
                    if let Event::Key(key) = &ev {
                        handle_menu_key(app, dirty_rx, op_tx, key.code);
                    }
                }
                AppMode::BranchSelect => {
                    if let Event::Key(key) = &ev {
                        handle_branch_select_key(app, op_tx, key.code);
                    }
                }
                AppMode::NewBranchInput => {
                    if let Event::Key(key) = &ev {
                        handle_new_branch_key(app, op_tx, key.code, key.modifiers);
                    }
                }
                AppMode::ConfirmForcePush => {
                    if let Event::Key(key) = &ev {
                        handle_confirm_force_push_key(app, op_tx, key.code);
                    }
                }
                AppMode::ConfirmDeleteBranch => {
                    if let Event::Key(key) = &ev {
                        handle_confirm_delete_branch_key(app, op_tx, key.code);
                    }
                }
                AppMode::History => {
                    if let Event::Key(key) = &ev {
                        handle_history_key(app, op_tx, key.code, key.modifiers);
                    }
                }
                AppMode::LogActionMenu => {
                    if let Event::Key(key) = &ev {
                        handle_log_menu_key(app, op_tx, key.code);
                    }
                }
                AppMode::FileActionMenu => {
                    if let Event::Key(key) = &ev {
                        handle_file_menu_key(app, op_tx, key.code);
                    }
                }
                AppMode::PopupMessage => {
                    // Any key dismisses the popup immediately
                    if let Event::Key(_) = &ev {
                        app.popup_message = None;
                        app.popup_show_time = None;
                        app.restore_base_mode();
                    }
                }
            }
        }

        if app.should_quit {
            break;
        }
    }

    Ok(())
}

/// Handle mouse events: clicks set pane focus, wheel scrolls the current pane.
fn handle_mouse_event(
    app: &mut App,
    op_tx: &std::sync::mpsc::Sender<OpResult>,
    mouse: &MouseEvent,
) {
    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            // ActionMenu mode:
            // - click on item executes it
            // - click outside closes menu
            if matches!(app.mode, AppMode::ActionMenu | AppMode::FileActionMenu) {
                let terminal_area = app
                    .cached_pane_areas
                    .as_ref()
                    .map(|a| a.terminal)
                    .unwrap_or_default();
                if let Some(item_idx) = menu_item_under_mouse(app, mouse, terminal_area) {
                    if let Some(item) = app.menu_items.get(item_idx).cloned() {
                        if matches!(app.mode, AppMode::FileActionMenu) {
                            dispatch_file_menu_action(app, op_tx, item.key);
                        } else {
                            dispatch_menu_action(app, op_tx, item.key);
                        }
                    }
                } else {
                    // Click outside menu closes it
                    app.close_menu();
                }
                return;
            }

            // BranchSelect popup: click on a branch entry to select and checkout.
            if matches!(app.mode, AppMode::BranchSelect) {
                // Determine the popup geometry similar to draw_branch_select.
                let term_area = app
                    .cached_pane_areas
                    .as_ref()
                    .map(|a| a.terminal)
                    .unwrap_or_default();
                // Height calculation matches UI.
                let height = (app.branch_items.len() as u16 + 4)
                    .clamp(5, 20)
                    .min(term_area.height);
                let popup = ui::centered_rect(55, height, term_area);
                let inner = ratatui::widgets::Block::default()
                    .borders(ratatui::widgets::Borders::ALL)
                    .title("")
                    .inner(popup);
                // If click inside the inner rect, determine which row.
                if mouse.column >= inner.x
                    && mouse.column < inner.x + inner.width
                    && mouse.row >= inner.y
                    && mouse.row < inner.y + inner.height
                {
                    let row_offset = (mouse.row - inner.y) as usize;
                    // Visible rows start at inner.y, no header.
                    // Determine start index as UI does.
                    let visible = inner.height as usize;
                    let start = if app.branch_selected >= visible {
                        app.branch_selected + 1 - visible
                    } else {
                        0
                    };
                    let idx = start + row_offset;
                    if idx < app.branch_items.len() {
                        app.branch_selected = idx;
                    }
                    // Perform checkout of the selected branch.
                    if let Some(item) = app.selected_branch_item().cloned() {
                        app.close_branch_select();
                        launch_op(
                            app,
                            op_tx,
                            OpRequest::CheckoutBranch {
                                name: item.name,
                                is_remote: item.is_remote,
                            },
                        );
                    }
                } else {
                    // Click outside popup just closes it.
                    app.close_branch_select();
                }
                return;
            }

            let now = std::time::Instant::now();
            let pos = (mouse.column, mouse.row);
            let is_double_click = app
                .last_click_time
                .map(|t| now.duration_since(t) < std::time::Duration::from_millis(300))
                .unwrap_or(false)
                && app.last_click_pos == Some(pos);

            if is_double_click {
                if matches!(app.focus, Focus::Repos) {
                    app.open_repo_action_menu();
                } else if matches!(app.focus, Focus::FileStatus) {
                    app.open_file_action_menu();
                }
            } else {
                handle_mouse_click(app, mouse);
            }

            app.last_click_time = Some(now);
            app.last_click_pos = Some(pos);
        }
        MouseEventKind::ScrollUp => {
            if matches!(app.mode, AppMode::ActionMenu) {
                app.menu_previous();
            } else {
                app.previous();
                app.reload_history_if_open();
            }
        }
        MouseEventKind::ScrollDown => {
            if matches!(app.mode, AppMode::ActionMenu) {
                app.menu_next();
            } else {
                app.next();
                app.reload_history_if_open();
            }
        }
        _ => {}
    }
}

/// Determine which pane a mouse click landed on and set focus accordingly.
/// Also updates the selection index if clicking in the repos table.
fn handle_mouse_click(app: &mut App, mouse: &MouseEvent) {
    // Only handle clicks in Normal or History mode (not during popups)
    if !matches!(app.mode, AppMode::Normal | AppMode::History) {
        return;
    }

    // Use the cached pane areas from the last draw call
    let Some(areas) = &app.cached_pane_areas else {
        return;
    };

    let click = (mouse.column, mouse.row);

    // Check each visible pane in priority order
    if let Some(file_status_area) = &areas.file_status {
        if in_rect(click, *file_status_area) {
            app.focus = Focus::FileStatus;
            // Also select the file under the mouse, if any
            if let Some(row) = file_status_row_under_mouse(app, mouse, *file_status_area) {
                app.file_status_selected = row;
                app.file_status_scroll = 0; // reset scroll so selection is visible
            }
            return;
        }
    }

    if let Some(history_area) = &areas.history {
        if in_rect(click, *history_area) {
            app.focus = Focus::History;
            // Also select the commit/change under the mouse, if any
            if let Some(row) = history_row_under_mouse(app, mouse, *history_area) {
                app.history_selected = row;
                app.history_scroll = 0;
            }
            return;
        }
    }

    if let Some(log_area) = &areas.log {
        if in_rect(click, *log_area) {
            app.focus = Focus::Log;
            return;
        }
    }

    if in_rect(click, areas.repos) {
        app.focus = Focus::Repos;
        // If clicking in the repos table, also update the selection
        if let Some(selected_row) = row_under_mouse(app, mouse, areas.repos) {
            app.selected = selected_row;
            app.file_status_selected = 0;
            app.file_status_scroll = 0;
        }
    }
}

/// Returns true if (col, row) is inside the given rect.
fn in_rect((col, row): (u16, u16), rect: ratatui::layout::Rect) -> bool {
    col >= rect.x && col < rect.x + rect.width && row >= rect.y && row < rect.y + rect.height
}

/// Given a mouse click in the repos table area, return the row index (0-based)
/// that corresponds to the click, or None if the click is on a border/header.
fn row_under_mouse(
    app: &App,
    mouse: &MouseEvent,
    table_area: ratatui::layout::Rect,
) -> Option<usize> {
    // The table block has borders (1 on each side) and a header row with
    // bottom_margin(1) — visually the header occupies 2 lines (content + margin).
    // Inner area starts at table_area.y + 1 (top border).
    // First data row: y + 1 (border) + 1 (header) + 1 (bottom_margin) = y + 3.
    let inner_top = table_area.y + 3;
    let inner_bottom = table_area.y + table_area.height - 1; // -1 for bottom border
    let row = mouse.row;

    if row < inner_top || row >= inner_bottom {
        return None;
    }

    let row_index = (row - inner_top) as usize;

    // Account for table_offset (scrolled rows)
    let offset_row = row_index + app.table_offset;

    // Clamp to available repos
    if offset_row < app.repos.len() {
        Some(offset_row)
    } else {
        None
    }
}

/// Given a mouse click in the File Status pane area, return the file index
/// (0-based in the full file list) under the mouse, or None.
fn file_status_row_under_mouse(
    app: &App,
    mouse: &MouseEvent,
    pane_area: ratatui::layout::Rect,
) -> Option<usize> {
    // The block has borders (1 on each side) and no header row.
    // Inner area starts at pane_area.y + 1 (top border).
    let inner_top = pane_area.y + 1;
    let inner_bottom = pane_area.y + pane_area.height - 1; // -1 for bottom border
    let row = mouse.row;

    if row < inner_top || row >= inner_bottom {
        return None;
    }

    let row_index = (row - inner_top) as usize;
    let files = app.selected_files();
    let offset_row = row_index;

    if offset_row < files.len() {
        Some(offset_row)
    } else {
        None
    }
}

/// Given a mouse click in the History pane area, return the flat row index
/// (0-based across commits + file sub-rows) under the mouse, or None.
fn history_row_under_mouse(
    app: &App,
    mouse: &MouseEvent,
    pane_area: ratatui::layout::Rect,
) -> Option<usize> {
    // The block has borders (1 on each side) and no header row.
    // Inner area starts at pane_area.y + 1 (top border).
    let inner_top = pane_area.y + 1;
    let inner_bottom = pane_area.y + pane_area.height - 1;
    let row = mouse.row;

    if row < inner_top || row >= inner_bottom {
        return None;
    }

    let row_index = (row - inner_top) as usize;
    let offset_row = row_index;

    if offset_row < app.history.len() {
        Some(offset_row)
    } else {
        None
    }
}

pub fn menu_item_under_mouse(
    app: &App,
    mouse: &MouseEvent,
    terminal_area: ratatui::layout::Rect,
) -> Option<usize> {
    // Mirror the geometry used in draw_action_menu:
    // height = menu_items.len() + 4 (title + top/bottom borders + blank line at bottom)
    let width = if matches!(app.mode, AppMode::FileActionMenu) {
        66
    } else {
        40
    };
    let height = (app.menu_items.len() as u16 + 4).min(terminal_area.height);
    let area = ui::top_centered_rect(width, height, 3, terminal_area);
    let (col, row) = (mouse.column, mouse.row);

    // Check if click is inside the menu area
    if col < area.x || col >= area.x + area.width || row < area.y || row >= area.y + area.height {
        return None;
    }

    // Content starts after the border (1 line top border)
    let inner_top = area.y + 1;
    let inner_bottom = area.y + area.height - 1; // -1 for bottom border

    if row < inner_top || row >= inner_bottom {
        return None;
    }

    let item_index = (row - inner_top) as usize;
    if item_index >= app.menu_items.len() {
        return None;
    }

    Some(item_index)
}

fn handle_normal_key(
    app: &mut App,
    _dirty_rx: &mut std::sync::mpsc::Receiver<String>,
    op_tx: &std::sync::mpsc::Sender<OpResult>,
    key: KeyCode,
    modifiers: KeyModifiers,
) {
    // Alt-f: fetch all tracked repos
    if modifiers.contains(KeyModifiers::ALT) && key == KeyCode::Char('f') {
        app.reset_auto_fetch_timer();
        launch_all_fetch(app, op_tx);
        return;
    }

    match key {
        KeyCode::Tab => app.cycle_focus(),
        KeyCode::BackTab => app.cycle_focus_reverse(),
        KeyCode::Down => {
            app.next();
            app.reload_history_if_open();
        }
        KeyCode::Up => {
            app.previous();
            app.reload_history_if_open();
        }
        KeyCode::PageDown => {
            app.next_page();
            app.reload_history_if_open();
        }
        KeyCode::PageUp => {
            app.previous_page();
            app.reload_history_if_open();
        }
        KeyCode::Char('r') => refresh_repos(app),
        KeyCode::Char('A') => app.enter_pick_mode(),
        KeyCode::Char('D') => app.request_remove_selected(),
        KeyCode::Char('s') => app.toggle_file_status(),
        KeyCode::Char('l') => app.toggle_log(),
        // Enter opens context-sensitive action menu
        KeyCode::Enter => {
            if app.focus == Focus::Log && app.show_log {
                app.open_log_action_menu();
            } else if app.focus == Focus::FileStatus && app.show_file_status {
                app.open_file_action_menu();
            } else {
                app.open_repo_action_menu();
            }
        }
        // Direct shortcuts (bypass menu)
        KeyCode::Char('f') => launch_op(app, op_tx, OpRequest::Fetch),
        KeyCode::Char('p') => launch_op(app, op_tx, OpRequest::Pull),
        KeyCode::Char('P') => launch_op(app, op_tx, OpRequest::Push),
        KeyCode::Char('c') => app.open_branch_select(),
        KeyCode::Char('h') => app.open_history(app::HistoryFilter::Full),
        KeyCode::Char('T') => app.next_theme(),
        _ => {}
    }
}

/// Fetch all tracked repos in parallel.
fn launch_all_fetch(app: &mut App, op_tx: &std::sync::mpsc::Sender<OpResult>) {
    let git_bin = app
        .config
        .general
        .git
        .clone()
        .unwrap_or_else(|| "git".to_string());

    let paths: Vec<String> = app
        .repos
        .iter()
        .filter(|r| r.error.is_none())
        .map(|r| r.path.clone())
        .collect();

    if paths.is_empty() {
        return;
    }

    app.log(format!("fetching all {} repos…", paths.len()));

    for path in paths {
        app.operations
            .insert(path.clone(), app::RepoOperation::Fetching);
        spawn_op(path, OpRequest::Fetch, git_bin.clone(), op_tx.clone());
    }
}

/// Dispatch a git operation for the currently selected repo.
fn launch_op(app: &mut App, op_tx: &std::sync::mpsc::Sender<OpResult>, request: OpRequest) {
    if app.repos.is_empty() {
        return;
    }
    let repo = &app.repos[app.selected];
    if repo.error.is_some() {
        return;
    }
    let path = repo.path.clone();
    let git_bin = app
        .config
        .general
        .git
        .clone()
        .unwrap_or_else(|| "git".to_string());
    let label = request.label();
    app.operations.insert(
        path.clone(),
        match &request {
            OpRequest::Fetch => app::RepoOperation::Fetching,
            OpRequest::Pull => app::RepoOperation::Pulling,
            OpRequest::Push | OpRequest::ForcePush => app::RepoOperation::Pushing,
            _ => app::RepoOperation::Fetching,
        },
    );
    app.log(format!("{label} {path}"));
    spawn_op(path, request, git_bin, op_tx.clone());
}

/// Handle a completed op result: log output, clear busy indicator, refresh.
fn handle_op_result(
    app: &mut App,
    dirty_rx: &mut std::sync::mpsc::Receiver<String>,
    result: OpResult,
) {
    app.operations.remove(&result.repo_path);
    let status = if result.success { "ok" } else { "FAILED" };
    app.log(format!(
        "{} {} — {status}",
        result.op_label, result.repo_path
    ));
    for line in &result.lines {
        app.log(format!("  {line}"));
    }
    // Auto-show output log on failure so the user sees what went wrong.
    if !result.success && !app.show_log {
        app.toggle_log();
    }
    refresh_single_repo(app, &result.repo_path);
    app.refresh_history_for_repo(&result.repo_path.clone());
    *dirty_rx = watcher::start(app.repos.iter().map(|r| r.path.clone()).collect());
}

fn handle_menu_key(
    app: &mut App,
    _dirty_rx: &mut std::sync::mpsc::Receiver<String>,
    op_tx: &std::sync::mpsc::Sender<OpResult>,
    key: KeyCode,
) {
    match key {
        KeyCode::Down => app.menu_next(),
        KeyCode::Up => app.menu_previous(),
        KeyCode::Esc => app.close_menu(),
        KeyCode::Enter => {
            if let Some(item) = app.menu_items.get(app.menu_selected).cloned() {
                dispatch_menu_action(app, op_tx, item.key);
            }
        }
        // Also handle direct key shortcuts inside the menu
        k => {
            if let KeyCode::Char(c) = k {
                dispatch_menu_action(app, op_tx, c);
            }
        }
    }
}

/// Handle key events for the log action menu.
fn handle_log_menu_key(app: &mut App, _op_tx: &std::sync::mpsc::Sender<OpResult>, key: KeyCode) {
    match key {
        KeyCode::Down => app.menu_next(),
        KeyCode::Up => app.menu_previous(),
        KeyCode::Esc => app.close_menu(),
        KeyCode::Enter => app.copy_log_to_clipboard(), // copy_log_to_clipboard sets mode to PopupMessage
        _ => {}
    }
}

fn dispatch_menu_action(app: &mut App, op_tx: &std::sync::mpsc::Sender<OpResult>, key: char) {
    match key {
        'f' => {
            app.close_menu();
            launch_op(app, op_tx, OpRequest::Fetch);
        }
        'p' => {
            app.close_menu();
            launch_op(app, op_tx, OpRequest::Pull);
        }
        'P' => {
            app.close_menu();
            launch_op(app, op_tx, OpRequest::Push);
        }
        'F' => {
            app.close_menu();
            app.confirm_force_push();
        }
        'c' => {
            app.close_menu();
            app.open_branch_select();
        }
        'n' => {
            app.close_menu();
            app.open_new_branch_input();
        }
        'x' => {
            app.close_menu();
            app.open_delete_branch_select();
        }
        'h' => {
            app.close_menu();
            app.open_history(app::HistoryFilter::Full);
        }
        'u' => {
            let branch = app
                .repos
                .get(app.selected)
                .and_then(|r| r.upstream.as_ref())
                .map(|u| u.branch.clone())
                .unwrap_or_default();
            app.close_menu();
            if !branch.is_empty() {
                app.open_history(app::HistoryFilter::AheadOf(branch));
            }
        }
        'U' => {
            let branch = app
                .repos
                .get(app.selected)
                .and_then(|r| r.upstream.as_ref())
                .map(|u| u.branch.clone())
                .unwrap_or_default();
            app.close_menu();
            if !branch.is_empty() {
                app.open_history(app::HistoryFilter::BehindOf(branch));
            }
        }
        't' => {
            let branch = app
                .repos
                .get(app.selected)
                .and_then(|r| r.trunk.as_ref())
                .map(|t| t.branch.clone())
                .unwrap_or_default();
            app.close_menu();
            if !branch.is_empty() {
                app.open_history(app::HistoryFilter::AheadOf(branch));
            }
        }
        'T' => {
            let branch = app
                .repos
                .get(app.selected)
                .and_then(|r| r.trunk.as_ref())
                .map(|t| t.branch.clone())
                .unwrap_or_default();
            app.close_menu();
            if !branch.is_empty() {
                app.open_history(app::HistoryFilter::BehindOf(branch));
            }
        }
        _ => {}
    }
}

fn handle_file_menu_key(app: &mut App, op_tx: &std::sync::mpsc::Sender<OpResult>, key: KeyCode) {
    match key {
        KeyCode::Down => app.menu_next(),
        KeyCode::Up => app.menu_previous(),
        KeyCode::Esc => app.close_menu(),
        KeyCode::Enter => {
            if let Some(item) = app.menu_items.get(app.menu_selected).cloned() {
                dispatch_file_menu_action(app, op_tx, item.key);
            }
        }
        k => {
            if let KeyCode::Char(c) = k {
                dispatch_file_menu_action(app, op_tx, c);
            }
        }
    }
}

fn dispatch_file_menu_action(app: &mut App, op_tx: &std::sync::mpsc::Sender<OpResult>, key: char) {
    let files = app.selected_files().to_vec();
    let file = match files.get(app.file_status_selected) {
        Some(f) => f.clone(),
        None => {
            app.close_menu();
            return;
        }
    };

    match key {
        's' => {
            app.close_menu();
            launch_op(app, op_tx, OpRequest::StageFile(file.path));
        }
        'u' => {
            app.close_menu();
            launch_op(app, op_tx, OpRequest::UnstageFile(file.path));
        }
        'r' => {
            let is_conflict = file.status == git::FileStatusKind::Conflict;
            app.close_menu();
            launch_op(
                app,
                op_tx,
                OpRequest::RevertFile {
                    file_path: file.path,
                    is_conflict,
                },
            );
        }
        'd' => {
            app.close_menu();
            launch_op(app, op_tx, OpRequest::DiscardFile(file.path));
        }
        _ => {}
    }
}

fn handle_branch_select_key(
    app: &mut App,
    op_tx: &std::sync::mpsc::Sender<OpResult>,
    key: KeyCode,
) {
    match key {
        KeyCode::Down => app.branch_select_next(),
        KeyCode::Up => app.branch_select_previous(),
        KeyCode::Esc => app.close_branch_select(),
        KeyCode::Enter => {
            if let Some(item) = app.selected_branch_item().cloned() {
                app.close_branch_select();
                launch_op(
                    app,
                    op_tx,
                    OpRequest::CheckoutBranch {
                        name: item.name,
                        is_remote: item.is_remote,
                    },
                );
            }
        }
        _ => {}
    }
}

fn handle_new_branch_key(
    app: &mut App,
    op_tx: &std::sync::mpsc::Sender<OpResult>,
    key: KeyCode,
    _modifiers: KeyModifiers,
) {
    match key {
        KeyCode::Esc => app.close_new_branch_input(),
        KeyCode::Enter => {
            let name = app.sanitised_branch_name();
            if !name.is_empty() {
                app.close_new_branch_input();
                launch_op(app, op_tx, OpRequest::CreateBranch(name));
            }
        }
        KeyCode::Backspace => {
            app.branch_input.pop();
        }
        KeyCode::Char(c) => app.branch_input.push(c),
        _ => {}
    }
}

fn handle_confirm_force_push_key(
    app: &mut App,
    op_tx: &std::sync::mpsc::Sender<OpResult>,
    key: KeyCode,
) {
    match key {
        KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
            app.restore_base_mode();
            launch_op(app, op_tx, OpRequest::ForcePush);
        }
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
            app.restore_base_mode();
        }
        _ => {}
    }
}

fn handle_confirm_delete_branch_key(
    app: &mut App,
    op_tx: &std::sync::mpsc::Sender<OpResult>,
    key: KeyCode,
) {
    match key {
        KeyCode::Down => app.branch_select_next(),
        KeyCode::Up => app.branch_select_previous(),
        KeyCode::Esc => {
            app.restore_base_mode();
        }
        KeyCode::Enter => {
            if let Some(item) = app.selected_branch_item().cloned() {
                app.branch_to_delete = item.name.clone();
                app.restore_base_mode();
                launch_op(app, op_tx, OpRequest::DeleteBranch(item.name));
            }
        }
        _ => {}
    }
}

fn handle_confirm_remove_key(
    app: &mut App,
    dirty_rx: &mut std::sync::mpsc::Receiver<String>,
    key: KeyCode,
) {
    match key {
        KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
            if let Some(removed) = app.remove_selected() {
                app.log(format!("removed repo {removed}"));
                refresh_repos(app);
                *dirty_rx = watcher::start(app.repos.iter().map(|r| r.path.clone()).collect());
            }
        }
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
            app.cancel_remove();
        }
        _ => {}
    }
}

fn handle_picker_event(
    app: &mut App,
    dirty_rx: &mut std::sync::mpsc::Receiver<String>,
    ev: &Event,
) {
    if let Event::Key(key) = ev {
        match key.code {
            // Esc cancels
            KeyCode::Esc => {
                app.cancel_pick();
            }

            // Enter: always navigate into the directory
            KeyCode::Enter => {
                if let Some(explorer) = app.file_explorer.as_mut() {
                    let _ = explorer.handle(ExplorerInput::Right);
                }
            }

            // Space: select the current directory as a repo
            KeyCode::Char(' ') => {
                if let Some(path) = app.picker_selected_path() {
                    match app.add_repo_path(&path) {
                        Ok(Some(new_path)) => {
                            add_repo_to_app(app, &new_path, dirty_rx);
                        }
                        Ok(None) => {
                            // Already tracked — close picker
                        }
                        Err(e) => {
                            // Not a valid git repo — show nothing, stay open
                            let _ = e;
                        }
                    }
                }
            }

            // Navigate down
            KeyCode::Down => {
                if let Some(e) = app.file_explorer.as_mut() {
                    let _ = e.handle(ExplorerInput::Down);
                }
            }
            // Navigate up
            KeyCode::Up => {
                if let Some(e) = app.file_explorer.as_mut() {
                    let _ = e.handle(ExplorerInput::Up);
                }
            }
            // Go into dir
            KeyCode::Right => {
                if let Some(e) = app.file_explorer.as_mut() {
                    let _ = e.handle(ExplorerInput::Right);
                }
            }
            // Go to parent
            KeyCode::Left | KeyCode::Backspace => {
                if let Some(e) = app.file_explorer.as_mut() {
                    let _ = e.handle(ExplorerInput::Left);
                }
            }
            KeyCode::Home => {
                if let Some(e) = app.file_explorer.as_mut() {
                    let _ = e.handle(ExplorerInput::Home);
                }
            }
            KeyCode::End => {
                if let Some(e) = app.file_explorer.as_mut() {
                    let _ = e.handle(ExplorerInput::End);
                }
            }
            KeyCode::PageUp => {
                if let Some(e) = app.file_explorer.as_mut() {
                    let _ = e.handle(ExplorerInput::PageUp);
                }
            }
            KeyCode::PageDown => {
                if let Some(e) = app.file_explorer.as_mut() {
                    let _ = e.handle(ExplorerInput::PageDown);
                }
            }
            _ => {}
        }
    }
}

/// Load the git status for a newly added path, discover submodules, restart watcher.
fn add_repo_to_app(
    app: &mut App,
    new_path: &str,
    dirty_rx: &mut std::sync::mpsc::Receiver<String>,
) {
    if let Ok(status) = git::get_repo_status(new_path) {
        app.repos.push(status);
        app.sort_repos();

        // Discover and add submodules
        if let Ok(repo) = git2::Repository::open(new_path) {
            if let Ok(submodules) = repo.submodules() {
                for sub in submodules {
                    if let Some(sub_path) = sub.path().to_str() {
                        let full = format!("{}/{}", new_path, sub_path);
                        if app.state.add_repo(&full) {
                            if let Ok(s) = git::get_repo_status(&full) {
                                app.repos.push(s);
                            }
                        }
                    }
                }
                app.sort_repos();
                let _ = app.state.save();
            }
        }

        *dirty_rx = watcher::start(app.repos.iter().map(|r| r.path.clone()).collect());
    }

    // Refresh recents list in app
    app.recent_repos = app
        .state
        .recent
        .iter()
        .map(|r| (r.path.clone(), r.name.clone()))
        .collect();
}

fn refresh_repos(app: &mut App) {
    let paths = app.tracked_paths();
    app.scanning = true;
    let started = Instant::now();
    app.log(format!("scanning {} repo(s)", paths.len()));

    app.repos = paths
        .iter()
        .map(|p| {
            git::get_repo_status(p)
                .unwrap_or_else(|e| git::RepoStatus::error_entry(p, format!("{e}")))
        })
        .collect();
    app.sort_repos();
    app.last_refreshed = Some(Instant::now());
    app.scanning = false;

    let n = app.repos.len();
    let errs = app.repos.iter().filter(|r| r.error.is_some()).count();
    let ms = started.elapsed().as_millis();
    if errs > 0 {
        app.log(format!(
            "scan complete — {n} repos, {errs} error(s) ({ms} ms)"
        ));
    } else {
        app.log(format!("scan complete — {n} repos ({ms} ms)"));
    }

    if app.selected >= app.repos.len() && !app.repos.is_empty() {
        app.selected = app.repos.len() - 1;
    }
}

fn refresh_single_repo(app: &mut App, path: &str) {
    if let Some(repo) = app.repos.iter_mut().find(|r| r.path == path) {
        match git::get_repo_status(path) {
            Ok(fresh) => *repo = fresh,
            Err(e) => *repo = git::RepoStatus::error_entry(path, format!("{e}")),
        }
    }
    app.last_refreshed = Some(Instant::now());
}

fn handle_history_key(
    app: &mut App,
    op_tx: &std::sync::mpsc::Sender<OpResult>,
    key: KeyCode,
    modifiers: KeyModifiers,
) {
    // Alt-f: fetch all tracked repos (global shortcut, works from any pane)
    if modifiers.contains(KeyModifiers::ALT) && key == KeyCode::Char('f') {
        app.reset_auto_fetch_timer();
        launch_all_fetch(app, op_tx);
        return;
    }

    match key {
        KeyCode::Char('h') => app.close_history(),
        KeyCode::Tab => app.cycle_focus(),
        KeyCode::BackTab => app.cycle_focus_reverse(),
        KeyCode::Down => app.next(),
        KeyCode::Up => app.previous(),
        KeyCode::PageDown => app.next_page(),
        KeyCode::PageUp => app.previous_page(),
        KeyCode::Enter => {
            if app.focus == Focus::Log && app.show_log {
                app.open_log_action_menu();
            } else if app.focus == Focus::FileStatus && app.show_file_status {
                app.open_file_action_menu();
            } else if app.focus == Focus::Repos {
                app.open_repo_action_menu();
            }
        }
        // Global keys that must work from any pane
        KeyCode::Char('s') => app.toggle_file_status(),
        KeyCode::Char('l') => app.toggle_log(),
        KeyCode::Char('r') => refresh_repos(app),
        KeyCode::Char('T') => app.next_theme(),
        KeyCode::Char('A') => app.enter_pick_mode(),
        KeyCode::Char('D') => app.request_remove_selected(),
        KeyCode::Char('c') => app.open_branch_select(),
        KeyCode::Char('f') => launch_op(app, op_tx, OpRequest::Fetch),
        KeyCode::Char('p') => launch_op(app, op_tx, OpRequest::Pull),
        KeyCode::Char('P') => launch_op(app, op_tx, OpRequest::Push),
        _ => {}
    }
}
