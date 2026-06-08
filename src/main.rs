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
mod utils;
mod watcher;

use anyhow::Result;
use app::{App, AppMode, Focus, HistoryFilter};
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
    sync::{
        mpsc::{self, Receiver, TryRecvError},
        Mutex,
    },
    time::{Duration, Instant},
};

pub(crate) static DEBUG_LOG: Mutex<Option<std::fs::File>> = Mutex::new(None);

/// Single point of control for writing a log line to the debug log file.
pub(crate) fn write_debug_log(line: &app::LogLine) {
    if let Ok(mut guard) = DEBUG_LOG.lock() {
        if let Some(f) = guard.as_mut() {
            use std::io::Write as _;
            let _ = writeln!(f, "{}", line.formatted());
        }
    }
}

#[macro_export]
macro_rules! dlog {
    ($($arg:tt)*) => {{
        $crate::write_debug_log(&$crate::app::LogLine::new_at(
            $crate::app::LogLevel::Debug,
            format!($($arg)*),
        ));
    }};
}

/// Tick interval for the event loop (UI responsiveness).
const TICK: Duration = Duration::from_millis(200);

/// If a single tick takes longer than this wall-clock gap, the system likely
/// woke from sleep. Trigger a full refresh to pick up changes that happened
/// while sleeping.
const WAKE_THRESHOLD: Duration = Duration::from_secs(3);

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    if args.iter().any(|a| a == "--version" || a == "-V") {
        println!(
            "gitover v{} (commit {}, built {})",
            env!("CARGO_PKG_VERSION"),
            env!("GIT_SHORT_HASH"),
            env!("BUILD_TIMESTAMP"),
        );
        return Ok(());
    }

    let config_override = parse_path_flag(&args, "--config");
    let state_override = parse_path_flag(&args, "--state");

    // Resolve debug-log path: CLI flag takes precedence over config file option.
    let config_for_log = config::Config::load_from(&config::find_config_path());
    let raw_log_path: Option<String> = args
        .windows(2)
        .find(|w| w[0] == "--debug-log")
        .map(|w| w[1].clone())
        .or_else(|| config_for_log.general.debug_log.clone());

    let log_path = raw_log_path
        .map(|s| {
            let (path, missing) = utils::expand_path(&s);
            if !missing.is_empty() {
                for name in &missing {
                    eprintln!("error: debug-log path: unknown variable ${{{name}}}");
                }
                Err(anyhow::anyhow!(
                    "debug-log path contains unresolvable variables"
                ))
            } else {
                Ok(path)
            }
        })
        .transpose()?;

    if let Some(ref log_path) = log_path {
        match std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path)
        {
            Ok(f) => {
                if let Ok(mut guard) = DEBUG_LOG.lock() {
                    *guard = Some(f);
                }
                dlog!("gitover started");
            }
            Err(e) => eprintln!("warning: cannot open debug log {log_path:?}: {e}"),
        }
    }

    // Restore terminal state even if a thread panics so the shell isn't left
    // in raw mode with mouse capture enabled (which prints escape sequences as text).
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = crossterm::terminal::disable_raw_mode();
        let _ = crossterm::execute!(
            std::io::stderr(),
            crossterm::terminal::LeaveAlternateScreen,
            crossterm::event::DisableMouseCapture,
        );
        default_hook(info);
    }));

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new_with_overrides(config_override, state_override);

    // On first launch (empty state), seed the repo list with CWD if it is a git repo.
    if app.state.repos.is_empty() {
        if let Ok(cwd) = std::env::current_dir() {
            if git2::Repository::open(&cwd).is_ok() {
                app.state.add_repo(&cwd.to_string_lossy());
                let _ = app.state.save();
            }
        }
    }

    refresh_repos(&mut app);
    app.reload_history_if_open(false);
    app.refresh_details();

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
                Ok(dirty_path) => {
                    refresh_single_repo(app, &dirty_path);
                    app.refresh_branches_for_repo(&dirty_path);
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => break,
            }
        }

        // Drain completed git-operation results
        loop {
            match op_rx.try_recv() {
                Ok(result) => handle_op_result(app, result),
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
        app.tick_header_flash();
        last_tick = Instant::now();

        app.spinner_tick = app.spinner_tick.wrapping_add(1);

        let timeout = TICK
            .checked_sub(last_tick.elapsed())
            .unwrap_or(Duration::ZERO);

        if event::poll(timeout)? {
            let ev = event::read()?;
            if let Event::Key(key) = &ev {
                dlog!(
                    "key: code={:?} modifiers={:?} mode={:?}",
                    key.code,
                    key.modifiers,
                    app.mode
                );
                if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
                    app.should_quit = true;
                    continue;
                }
                // Alt-f / Option-f: global shortcut — fetch all repos from any mode.
                // macOS Terminal sends ƒ (U+0192) with no modifier instead of ALT+'f'.
                let is_alt_f = (key.modifiers.contains(KeyModifiers::ALT)
                    && key.code == KeyCode::Char('f'))
                    || key.code == KeyCode::Char('ƒ');
                if is_alt_f {
                    dlog!("alt-f matched → launch_all_fetch");
                    app.reset_auto_fetch_timer();
                    launch_all_fetch(app, op_tx);
                    continue;
                }
                dlog!(
                    "alt-f NOT matched (modifiers={:?} code={:?})",
                    key.modifiers,
                    key.code
                );
            }

            // Handle mouse events (clicks for focus, wheel for scroll)
            // Skip while a modal overlay is open so the underlying panes don't scroll.
            if let Event::Mouse(mouse) = &ev {
                if matches!(app.mode, AppMode::HelpOverlay) {
                    match mouse.kind {
                        MouseEventKind::ScrollUp => {
                            app.help_overlay_scroll = app.help_overlay_scroll.saturating_sub(1);
                        }
                        MouseEventKind::ScrollDown => {
                            app.help_overlay_scroll = app
                                .help_overlay_scroll
                                .saturating_add(1)
                                .min(app.help_overlay_max_scroll);
                        }
                        MouseEventKind::Down(MouseButton::Left) => {
                            let outside = app.help_overlay_area.is_none_or(|r| {
                                mouse.column < r.x
                                    || mouse.column >= r.x + r.width
                                    || mouse.row < r.y
                                    || mouse.row >= r.y + r.height
                            });
                            if outside {
                                app.help_overlay_scroll = 0;
                                app.restore_base_mode();
                            }
                        }
                        _ => {}
                    }
                } else {
                    handle_mouse_event(app, op_tx, mouse);
                }
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

                AppMode::ConfirmDeleteLocalBranch => {
                    if let Event::Key(key) = &ev {
                        handle_confirm_delete_local_branch_key(app, op_tx, key.code);
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
                AppMode::BranchActionMenu => {
                    if let Event::Key(key) = &ev {
                        handle_branch_menu_key(app, op_tx, key.code);
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
                AppMode::HelpOverlay => {
                    if let Event::Key(key) = &ev {
                        match key.code {
                            KeyCode::Down => {
                                app.help_overlay_scroll = app
                                    .help_overlay_scroll
                                    .saturating_add(1)
                                    .min(app.help_overlay_max_scroll);
                            }
                            KeyCode::Up => {
                                app.help_overlay_scroll = app.help_overlay_scroll.saturating_sub(1);
                            }
                            KeyCode::PageDown => {
                                app.help_overlay_scroll = app
                                    .help_overlay_scroll
                                    .saturating_add(10)
                                    .min(app.help_overlay_max_scroll);
                            }
                            KeyCode::PageUp => {
                                app.help_overlay_scroll =
                                    app.help_overlay_scroll.saturating_sub(10);
                            }
                            KeyCode::Esc | KeyCode::Enter | KeyCode::Char('?') => {
                                app.help_overlay_scroll = 0;
                                app.restore_base_mode();
                            }
                            _ => {}
                        }
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
            if matches!(
                app.mode,
                AppMode::ActionMenu | AppMode::FileActionMenu | AppMode::BranchActionMenu
            ) {
                if let Some(item_idx) = menu_item_under_mouse(app, mouse) {
                    if let Some(item) = app.menu_items.get(item_idx).cloned() {
                        if !item.is_separator {
                            if matches!(app.mode, AppMode::FileActionMenu) {
                                dispatch_file_menu_action(app, op_tx, item.key);
                            } else if matches!(app.mode, AppMode::BranchActionMenu) {
                                dispatch_branch_menu_action(app, op_tx, item.key);
                            } else {
                                activate_menu_item(app, op_tx, &item);
                            }
                        }
                    }
                } else {
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
                    .clamp(ui::BRANCH_SELECT_MIN_HEIGHT, ui::BRANCH_SELECT_MAX_HEIGHT)
                    .min(term_area.height);
                let popup = ui::centered_rect(ui::BRANCH_SELECT_WIDTH_PCT, height, term_area);
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
                        let (name, is_remote) = if item.is_remote {
                            (format!("origin/{}", item.name), true)
                        } else {
                            (item.name, false)
                        };
                        launch_op(app, op_tx, OpRequest::CheckoutBranch { name, is_remote });
                    }
                } else {
                    // Click outside popup just closes it.
                    app.close_branch_select();
                }
                return;
            }

            // Repos-divider drag: start drag when clicking on the bottom border
            // of the repos pane (only when optional panes are open below it).
            if matches!(app.mode, AppMode::Normal | AppMode::History)
                && is_repos_divider(app, mouse)
            {
                app.dragging_repos_divider = true;
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
                app.reload_history_if_open(false);
                app.refresh_details();
            }

            app.last_click_time = Some(now);
            app.last_click_pos = Some(pos);
        }
        MouseEventKind::Drag(MouseButton::Left) => {
            if app.dragging_repos_divider {
                update_repos_height_from_drag(app, mouse);
            }
        }
        MouseEventKind::Up(MouseButton::Left) => {
            if app.dragging_repos_divider {
                update_repos_height_from_drag(app, mouse);
                app.dragging_repos_divider = false;
            }
        }
        // NOTE: Moved events (no button held) require the terminal to honour the
        // `?1003h` any-event mouse mode that crossterm requests via EnableMouseCapture.
        // ZED and kitty send them correctly; iTerm2 does not, so the hover indicator
        // on the repos divider is never triggered there. Drag (button-held motion) uses
        // `?1002h` which iTerm2 does support, so drag itself works fine in iTerm2.
        MouseEventKind::Moved => {
            app.hover_repos_divider = matches!(app.mode, AppMode::Normal | AppMode::History)
                && is_repos_divider(app, mouse);
        }
        MouseEventKind::ScrollUp => {
            if matches!(app.mode, AppMode::ActionMenu) {
                app.menu_previous();
            } else {
                focus_pane_under_mouse(app, mouse);
                app.previous();
                app.reload_history_if_open(false);
                app.refresh_details();
            }
        }
        MouseEventKind::ScrollDown => {
            if matches!(app.mode, AppMode::ActionMenu) {
                app.menu_next();
            } else {
                focus_pane_under_mouse(app, mouse);
                app.next();
                app.reload_history_if_open(false);
                app.refresh_details();
            }
        }
        _ => {}
    }
}

/// Returns true when the mouse is on the bottom border row of the repos pane
/// and at least one optional pane is visible below (making the divider draggable).
fn is_repos_divider(app: &App, mouse: &MouseEvent) -> bool {
    let Some(areas) = &app.cached_pane_areas else {
        return false;
    };
    let has_panes_below =
        areas.file_status.is_some() || areas.history.is_some() || areas.log.is_some();
    if !has_panes_below {
        return false;
    }
    mouse.row == areas.repos.y + areas.repos.height.saturating_sub(1)
}

/// Recompute `repos_height_override` so the bottom of the repos pane tracks the mouse row.
fn update_repos_height_from_drag(app: &mut App, mouse: &MouseEvent) {
    let Some(areas) = app.cached_pane_areas.clone() else {
        return;
    };
    let open_panes = [
        areas.file_status.is_some(),
        areas.history.is_some(),
        areas.log.is_some(),
    ]
    .into_iter()
    .filter(|&p| p)
    .count() as u16;
    if open_panes == 0 {
        return;
    }
    let total_available = (areas.terminal.y + areas.terminal.height).saturating_sub(areas.repos.y);
    let new_height = mouse.row.saturating_sub(areas.repos.y) + 1;
    const MIN_REPOS: u16 = 4;
    let max_h = total_available
        .saturating_sub(open_panes * 3)
        .max(MIN_REPOS);
    app.repos_height_override = Some(new_height.clamp(MIN_REPOS, max_h));
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
            }
            app.refresh_details();
            return;
        }
    }

    if let Some(history_area) = &areas.history {
        if in_rect(click, *history_area) {
            app.focus = Focus::History;
            // Also select the commit/change under the mouse, if any
            if let Some(row) = history_row_under_mouse(app, mouse, *history_area) {
                app.history_selected = row;
            }
            app.refresh_details();
            return;
        }
    }

    if let Some(diff_area) = &areas.diff {
        if in_rect(click, *diff_area) {
            app.focus = Focus::Details;
            return;
        }
    }

    if let Some(log_area) = &areas.log {
        if in_rect(click, *log_area) {
            app.focus = Focus::Log;
            return;
        }
    }

    if let Some(branches_area) = &areas.branches {
        if in_rect(click, *branches_area) {
            app.focus = Focus::Branches;
            if let Some(row) = branches_row_under_mouse(app, mouse, *branches_area) {
                if row != app.branches_pane_selected {
                    app.branches_pane_selected = row;
                    app.reload_history_from_branches();
                }
            }
            return;
        }
    }

    if in_rect(click, areas.repos) {
        app.focus = Focus::Repos;
        if let Some(selected_row) = row_under_mouse(app, mouse, areas.repos) {
            if selected_row != app.selected {
                app.selected = selected_row;
                app.file_status_selected = 0;
                app.file_status_scroll = 0;
                app.reload_history_if_open(false);
                app.refresh_details();
            }
        }
    }
}

/// Set focus to whichever pane the mouse cursor is currently over.
/// Used by scroll events so scrolling an unfocused pane works intuitively.
fn focus_pane_under_mouse(app: &mut App, mouse: &MouseEvent) {
    if !matches!(app.mode, AppMode::Normal | AppMode::History) {
        return;
    }
    let Some(areas) = &app.cached_pane_areas else {
        return;
    };
    let pos = (mouse.column, mouse.row);
    if let Some(a) = areas.file_status {
        if in_rect(pos, a) {
            app.focus = Focus::FileStatus;
            return;
        }
    }
    if let Some(a) = areas.history {
        if in_rect(pos, a) {
            app.focus = Focus::History;
            return;
        }
    }
    if let Some(a) = areas.diff {
        if in_rect(pos, a) {
            app.focus = Focus::Details;
            return;
        }
    }
    if let Some(a) = areas.log {
        if in_rect(pos, a) {
            app.focus = Focus::Log;
            return;
        }
    }
    if let Some(a) = areas.branches {
        if in_rect(pos, a) {
            app.focus = Focus::Branches;
            return;
        }
    }
    if in_rect(pos, areas.repos) {
        app.focus = Focus::Repos;
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
    // 1 top border + 1 header row = first data row at y + 2.
    let inner_top = table_area.y + 2;
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

/// Given a mouse click in the Branches pane, return the branch index under the mouse.
fn branches_row_under_mouse(
    app: &App,
    mouse: &MouseEvent,
    pane_area: ratatui::layout::Rect,
) -> Option<usize> {
    // 1 border + 1 header row = data starts at y + 2.
    let inner_top = pane_area.y + 2;
    let inner_bottom = pane_area.y + pane_area.height - 1;
    let row = mouse.row;
    if row < inner_top || row >= inner_bottom {
        return None;
    }
    let row_index = (row - inner_top) as usize;
    let offset_row = row_index + app.branches_pane_scroll;
    if offset_row < app.branch_info_list.len() {
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
    let offset_row = row_index + app.file_status_scroll;

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
    let offset_row = row_index + app.history_scroll;

    if offset_row < app.history_row_count() {
        Some(offset_row)
    } else {
        None
    }
}

pub fn menu_item_under_mouse(app: &App, mouse: &MouseEvent) -> Option<usize> {
    let area = ui::action_menu_area(app);
    let (col, row) = (mouse.column, mouse.row);

    if col < area.x || col >= area.x + area.width || row < area.y || row >= area.y + area.height {
        return None;
    }

    let inner_top = area.y + 1;
    let inner_bottom = area.y + area.height - 1;

    if row < inner_top || row >= inner_bottom {
        return None;
    }

    let item_index = (row - inner_top) as usize + app.menu_scroll;
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
    _modifiers: KeyModifiers,
) {
    match key {
        KeyCode::Tab => {
            app.cycle_focus();
            app.refresh_details();
        }
        KeyCode::BackTab => {
            app.cycle_focus_reverse();
            app.refresh_details();
        }
        KeyCode::Down => {
            app.next();
            if app.focus == Focus::Branches {
                app.reload_history_from_branches();
            } else {
                app.reload_history_if_open(false);
            }
            app.refresh_details();
        }
        KeyCode::Up => {
            app.previous();
            if app.focus == Focus::Branches {
                app.reload_history_from_branches();
            } else {
                app.reload_history_if_open(false);
            }
            app.refresh_details();
        }
        KeyCode::PageDown => {
            app.next_page();
            if app.focus == Focus::Branches {
                app.reload_history_from_branches();
            } else {
                app.reload_history_if_open(false);
            }
            app.refresh_details();
        }
        KeyCode::PageUp => {
            app.previous_page();
            if app.focus == Focus::Branches {
                app.reload_history_from_branches();
            } else {
                app.reload_history_if_open(false);
            }
            app.refresh_details();
        }
        KeyCode::Char('r') => refresh_repos(app),
        KeyCode::Char('A') => app.enter_pick_mode(),
        KeyCode::Char('D') => app.request_remove_selected(),
        KeyCode::Char('s') => app.toggle_file_status(),
        KeyCode::Char('l') => app.toggle_log(),
        KeyCode::Char('d') => app.toggle_details(),
        KeyCode::Char('b') => {
            if app.show_branches {
                app.close_branches_pane();
            } else {
                app.open_branches_pane();
            }
        }
        // Enter opens context-sensitive action menu
        KeyCode::Enter => {
            if app.focus == Focus::Branches {
                app.open_branch_action_menu();
            } else if app.focus == Focus::Log && app.show_log {
                app.open_log_action_menu();
            } else if app.focus == Focus::FileStatus && app.show_file_status {
                app.open_file_action_menu();
            } else {
                app.open_repo_action_menu();
            }
        }
        // Direct shortcuts (bypass menu)
        KeyCode::Char('f') => launch_op(app, op_tx, OpRequest::Fetch),
        KeyCode::Char('p') => {
            if let Some(op) = branch_pull_op(app) {
                launch_op(app, op_tx, op);
            } else {
                launch_op(app, op_tx, OpRequest::Pull);
            }
        }
        KeyCode::Char('P') => launch_op(app, op_tx, OpRequest::Push),
        KeyCode::Char('c') => {
            if app.focus == Focus::Branches {
                // Direct checkout of the selected branch (bypasses dialog)
                if let Some(b) = app.selected_branch_info().cloned() {
                    if !b.is_current {
                        let (name, is_remote) = if b.is_remote_only {
                            (format!("origin/{}", b.name), true)
                        } else {
                            (b.name, false)
                        };
                        launch_op(app, op_tx, OpRequest::CheckoutBranch { name, is_remote });
                    }
                }
            } else {
                app.open_branch_select();
            }
        }
        KeyCode::Char('h') => app.open_history(app::HistoryFilter::Full),
        KeyCode::Char('T') => app.next_theme(),
        KeyCode::Char('?') => app.mode = AppMode::HelpOverlay,
        KeyCode::Esc => {
            if app.focus == Focus::Branches {
                app.close_branches_pane();
            }
        }
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

    let total = app.repos.len();
    let paths: Vec<String> = app
        .repos
        .iter()
        .filter(|r| r.error.is_none())
        .map(|r| r.path.clone())
        .collect();

    dlog!(
        "launch_all_fetch: total_repos={} fetchable={}",
        total,
        paths.len()
    );
    for r in &app.repos {
        dlog!("  repo path={:?} error={:?}", r.path, r.error);
    }

    if paths.is_empty() {
        dlog!("launch_all_fetch: no fetchable repos, returning early");
        return;
    }

    app.set_header_flash(format!("↻ fetching {} repos…", paths.len()));
    app.log(format!("fetching all {} repos…", paths.len()));

    for path in paths {
        app.operations
            .insert(path.clone(), app::RepoOperation::Fetching);
        spawn_op(path, OpRequest::Fetch, git_bin.clone(), op_tx.clone());
    }
}

/// Return a `PullBranch` op if the branches pane is focused and the selected branch
/// is a non-current branch that can be fast-forwarded, otherwise `None`.
fn branch_pull_op(app: &App) -> Option<OpRequest> {
    if app.focus != Focus::Branches {
        return None;
    }
    let b = app.selected_branch_info()?;
    if b.is_current {
        return None;
    }
    let up = b.upstream.as_ref()?;
    if up.behind == 0 || up.ahead != 0 {
        return None;
    }
    Some(OpRequest::PullBranch {
        name: b.name.clone(),
        upstream: up.branch.clone(),
    })
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
            OpRequest::Pull | OpRequest::PullBranch { .. } => app::RepoOperation::Pulling,
            OpRequest::Push | OpRequest::ForcePush => app::RepoOperation::Pushing,
            _ => app::RepoOperation::Fetching,
        },
    );
    app.log(format!("run '{label}' in {path}"));
    spawn_op(path, request, git_bin, op_tx.clone());
}

/// Expand `${ROOT}` / `${BRANCH}` and env vars in a repo command string and spawn it.
fn launch_repo_cmd(
    app: &mut App,
    op_tx: &std::sync::mpsc::Sender<OpResult>,
    name: &str,
    raw_cmd: &str,
    background: bool,
) {
    if app.repos.is_empty() {
        return;
    }
    let repo = &app.repos[app.selected];
    if repo.error.is_some() {
        return;
    }
    let root = repo.path.clone();
    let branch = repo.branch.clone();

    // Step 1: replace repo-dependent vars (${ROOT}, ${BRANCH}).
    // Unresolved names are left as-is and will be tried as env vars in step 2.
    let (s1, _) = utils::expand_vars(raw_cmd, &[("ROOT", &root), ("BRANCH", &branch)]);

    // Step 2: replace remaining env vars (${HOME}, etc.).
    // Any name still unresolved at this point is a genuine error.
    let (cmd, missing) = utils::expand_env_vars(&s1);
    for var in &missing {
        app.log_error(format!(
            "repo command '{name}': unknown variable ${{{var}}}"
        ));
    }
    if !missing.is_empty() {
        return;
    }
    let name = name.to_string();
    let git_bin = app
        .config
        .general
        .git
        .clone()
        .unwrap_or_else(|| "git".to_string());
    app.log(format!("run '{name}' in {root}"));
    spawn_op(
        root,
        OpRequest::RunRepoCommand {
            name,
            cmd,
            background,
        },
        git_bin,
        op_tx.clone(),
    );
}

/// Handle a completed op result: log output, clear busy indicator, refresh.
fn handle_op_result(app: &mut App, result: OpResult) {
    app.operations.remove(&result.repo_path);
    if !result.success {
        app.log_error(format!(
            "'{}' failed in {}",
            result.op_label, result.repo_path
        ));
    }
    for line in &result.lines {
        app.log(format!("  {line}"));
    }
    // Auto-show output log on failure so the user sees what went wrong.
    if !result.success && !app.show_log {
        app.toggle_log();
    }
    refresh_single_repo(app, &result.repo_path);
    app.reload_history_if_open(true);
    app.refresh_branches_for_repo(&result.repo_path.clone());
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
        KeyCode::PageDown => app.menu_next_page(),
        KeyCode::PageUp => app.menu_previous_page(),
        KeyCode::Esc => app.close_menu(),
        KeyCode::Enter => {
            if let Some(item) = app.menu_items.get(app.menu_selected).cloned() {
                if !item.is_separator {
                    activate_menu_item(app, op_tx, &item);
                }
            }
        }
        k => {
            if let KeyCode::Char(c) = k {
                // Check for a repo-command with this key first; fall back to built-in dispatch.
                let rc = app
                    .menu_items
                    .iter()
                    .find(|i| !i.is_separator && i.key == c && i.repo_cmd.is_some())
                    .cloned();
                if let Some(item) = rc {
                    activate_menu_item(app, op_tx, &item);
                } else {
                    dispatch_menu_action(app, op_tx, c);
                }
            }
        }
    }
}

/// Execute the action for a non-separator menu item: repo command or built-in.
fn activate_menu_item(
    app: &mut App,
    op_tx: &std::sync::mpsc::Sender<OpResult>,
    item: &app::MenuItem,
) {
    if let Some((raw_cmd, background)) = &item.repo_cmd {
        let name = item.label.clone();
        let cmd = raw_cmd.clone();
        let bg = *background;
        app.close_menu();
        launch_repo_cmd(app, op_tx, &name, &cmd, bg);
    } else {
        dispatch_menu_action(app, op_tx, item.key);
    }
}

/// Handle key events for the log action menu.
fn handle_log_menu_key(app: &mut App, _op_tx: &std::sync::mpsc::Sender<OpResult>, key: KeyCode) {
    match key {
        KeyCode::Down => app.menu_next(),
        KeyCode::Up => app.menu_previous(),
        KeyCode::PageDown => app.menu_next_page(),
        KeyCode::PageUp => app.menu_previous_page(),
        KeyCode::Esc => app.close_menu(),
        KeyCode::Enter => {
            if let Some(item) = app.menu_items.get(app.menu_selected).cloned() {
                dispatch_log_menu_action(app, item.key);
            }
        }
        KeyCode::Char(character) => dispatch_log_menu_action(app, character),
        _ => {}
    }
}

fn dispatch_log_menu_action(app: &mut App, key: char) {
    match key {
        'c' => app.copy_log_to_clipboard(),
        'x' => {
            app.close_menu();
            app.clear_log();
        }
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
        KeyCode::PageDown => app.menu_next_page(),
        KeyCode::PageUp => app.menu_previous_page(),
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
        'p' => {
            app.close_menu();
            launch_op(
                app,
                op_tx,
                OpRequest::SavePatchAndRevert {
                    file_path: file.path,
                },
            );
        }
        'P' => {
            app.close_menu();
            launch_op(
                app,
                op_tx,
                OpRequest::ApplyPatch {
                    file_path: file.path,
                },
            );
        }
        _ => {}
    }
}

fn handle_branch_menu_key(app: &mut App, op_tx: &std::sync::mpsc::Sender<OpResult>, key: KeyCode) {
    match key {
        KeyCode::Down => app.menu_next(),
        KeyCode::Up => app.menu_previous(),
        KeyCode::PageDown => app.menu_next_page(),
        KeyCode::PageUp => app.menu_previous_page(),
        KeyCode::Esc => app.close_menu(),
        KeyCode::Enter => {
            if let Some(item) = app.menu_items.get(app.menu_selected).cloned() {
                if !item.is_separator {
                    dispatch_branch_menu_action(app, op_tx, item.key);
                }
            }
        }
        k => {
            if let KeyCode::Char(c) = k {
                dispatch_branch_menu_action(app, op_tx, c);
            }
        }
    }
}

fn dispatch_branch_menu_action(
    app: &mut App,
    op_tx: &std::sync::mpsc::Sender<OpResult>,
    key: char,
) {
    let branch = match app
        .branch_info_list
        .get(app.branches_pane_selected)
        .cloned()
    {
        Some(b) => b,
        None => {
            app.close_menu();
            return;
        }
    };

    match key {
        'c' => {
            app.close_menu();
            let (name, is_remote) = if branch.is_remote_only {
                (format!("origin/{}", branch.name), true)
            } else {
                (branch.name, false)
            };
            launch_op(app, op_tx, OpRequest::CheckoutBranch { name, is_remote });
        }
        'h' => {
            app.close_menu();
            app.open_history_for_branch(HistoryFilter::BranchFull(branch.name));
        }
        'u' => {
            let of = branch.upstream.map(|u| u.branch).unwrap_or_default();
            app.close_menu();
            if !of.is_empty() {
                app.open_history_for_branch(HistoryFilter::BranchAheadOf {
                    branch: branch.name,
                    of,
                });
            }
        }
        'U' => {
            let of = branch.upstream.map(|u| u.branch).unwrap_or_default();
            app.close_menu();
            if !of.is_empty() {
                app.open_history_for_branch(HistoryFilter::BranchBehindOf {
                    branch: branch.name,
                    of,
                });
            }
        }
        't' => {
            let of = branch.trunk.map(|t| t.branch).unwrap_or_default();
            app.close_menu();
            if !of.is_empty() {
                app.open_history_for_branch(HistoryFilter::BranchAheadOf {
                    branch: branch.name,
                    of,
                });
            }
        }
        'T' => {
            let of = branch.trunk.map(|t| t.branch).unwrap_or_default();
            app.close_menu();
            if !of.is_empty() {
                app.open_history_for_branch(HistoryFilter::BranchBehindOf {
                    branch: branch.name,
                    of,
                });
            }
        }
        'p' => {
            let upstream = branch
                .upstream
                .as_ref()
                .map(|u| u.branch.clone())
                .unwrap_or_else(|| format!("origin/{}", branch.name));
            app.close_menu();
            launch_op(
                app,
                op_tx,
                OpRequest::PullBranch {
                    name: branch.name,
                    upstream,
                },
            );
        }
        'n' => {
            let base = branch.name.clone();
            app.close_menu();
            app.open_new_branch_from_input(base);
        }
        'd' => {
            app.open_confirm_delete_local_branch();
        }
        _ => {}
    }
}

fn handle_confirm_delete_local_branch_key(
    app: &mut App,
    op_tx: &std::sync::mpsc::Sender<OpResult>,
    key: KeyCode,
) {
    match key {
        KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
            let name = app.branch_to_delete.clone();
            app.restore_base_mode();
            launch_op(app, op_tx, OpRequest::DeleteBranch(name));
        }
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
            app.restore_base_mode();
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
                let (name, is_remote) = if item.is_remote {
                    (format!("origin/{}", item.name), true)
                } else {
                    (item.name, false)
                };
                launch_op(app, op_tx, OpRequest::CheckoutBranch { name, is_remote });
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
                let base = app.branch_input_base.clone();
                app.close_new_branch_input();
                if base.is_empty() {
                    launch_op(app, op_tx, OpRequest::CreateBranch(name));
                } else {
                    launch_op(app, op_tx, OpRequest::CreateBranchFrom { name, base });
                }
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

            // Space: add the current directory as a repo, keep picker open
            KeyCode::Char(' ') => {
                if let Some(path) = app.picker_selected_path() {
                    match app.add_repo_path(&path) {
                        Ok(Some(new_path)) => {
                            add_repo_to_app(app, &new_path, dirty_rx);
                        }
                        Ok(None) => {
                            // Already tracked — stay open so user can navigate further
                        }
                        Err(_e) => {
                            // Not a valid git repo — stay open
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
    let case_sensitive_sort = app.config.general.case_sensitive_path_sorting;
    if let Ok(status) = git::get_repo_status(new_path, case_sensitive_sort) {
        app.repos.push(status);
        app.sort_repos();

        // Select the newly added repo in the Repositories pane.
        if let Some(idx) = app.repos.iter().position(|r| r.path == new_path) {
            app.selected = idx;
        }

        // Discover and add submodules
        if let Ok(repo) = git2::Repository::open(new_path) {
            if let Ok(submodules) = repo.submodules() {
                for sub in submodules {
                    if let Some(sub_path) = sub.path().to_str() {
                        let full = format!("{}/{}", new_path, sub_path);
                        if app.state.add_repo(&full) {
                            if let Ok(s) = git::get_repo_status(&full, case_sensitive_sort) {
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
}

fn refresh_repos(app: &mut App) {
    let paths = app.tracked_paths();
    app.scanning = true;
    let started = Instant::now();
    app.log(format!("scanning {} repo(s)", paths.len()));

    let case_sensitive_sort = app.config.general.case_sensitive_path_sorting;
    app.repos = paths
        .iter()
        .map(|p| {
            git::get_repo_status(p, case_sensitive_sort)
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

/// Return the value after `flag` in `args`, or `None` if not present / no value follows.
fn parse_path_flag(args: &[String], flag: &str) -> Option<std::path::PathBuf> {
    let mut iter = args.iter();
    while let Some(a) = iter.next() {
        if a == flag {
            return iter.next().map(std::path::PathBuf::from);
        }
    }
    None
}

fn refresh_single_repo(app: &mut App, path: &str) {
    let case_sensitive_sort = app.config.general.case_sensitive_path_sorting;
    if let Some(repo) = app.repos.iter_mut().find(|r| r.path == path) {
        match git::get_repo_status(path, case_sensitive_sort) {
            Ok(fresh) => *repo = fresh,
            Err(e) => *repo = git::RepoStatus::error_entry(path, format!("{e}")),
        }
    }
    app.last_refreshed = Some(Instant::now());
}

#[cfg(test)]
mod tests {
    use super::parse_path_flag;
    use std::path::PathBuf;

    #[test]
    fn parse_path_flag_returns_value_after_flag() {
        let args = &[
            "gitover".to_string(),
            "--config".to_string(),
            "/etc/my.yaml".to_string(),
        ];
        assert_eq!(
            parse_path_flag(args, "--config"),
            Some(PathBuf::from("/etc/my.yaml"))
        );
    }

    #[test]
    fn parse_path_flag_not_present_returns_none() {
        let args = &["gitover".to_string(), "--version".to_string()];
        assert_eq!(parse_path_flag(args, "--config"), None);
        assert_eq!(parse_path_flag(args, "--state"), None);
    }

    #[test]
    fn parse_path_flag_missing_value_returns_none() {
        // Flag is the last arg — no value follows it.
        let args = &["gitover".to_string(), "--config".to_string()];
        assert_eq!(parse_path_flag(args, "--config"), None);
    }

    #[test]
    fn parse_path_flag_both_flags_present() {
        let args = &[
            "gitover".to_string(),
            "--config".to_string(),
            "/cfg.yaml".to_string(),
            "--state".to_string(),
            "/state.yaml".to_string(),
        ];
        assert_eq!(
            parse_path_flag(args, "--config"),
            Some(PathBuf::from("/cfg.yaml"))
        );
        assert_eq!(
            parse_path_flag(args, "--state"),
            Some(PathBuf::from("/state.yaml"))
        );
    }

    #[test]
    fn parse_path_flag_returns_first_occurrence() {
        // Only the first occurrence of the flag is used.
        let args = &[
            "gitover".to_string(),
            "--config".to_string(),
            "/first.yaml".to_string(),
            "--config".to_string(),
            "/second.yaml".to_string(),
        ];
        assert_eq!(
            parse_path_flag(args, "--config"),
            Some(PathBuf::from("/first.yaml"))
        );
    }
}

fn handle_history_key(
    app: &mut App,
    op_tx: &std::sync::mpsc::Sender<OpResult>,
    key: KeyCode,
    modifiers: KeyModifiers,
) {
    match key {
        KeyCode::Char('h') => app.close_history(),
        KeyCode::Tab => {
            app.cycle_focus();
            app.refresh_details();
        }
        KeyCode::BackTab => {
            app.cycle_focus_reverse();
            app.refresh_details();
        }
        KeyCode::Down if modifiers.contains(KeyModifiers::SHIFT) && app.focus == Focus::History => {
            app.next_commit();
            app.refresh_details();
        }
        KeyCode::Up if modifiers.contains(KeyModifiers::SHIFT) && app.focus == Focus::History => {
            app.previous_commit();
            app.refresh_details();
        }
        // Alternative bindings for terminals that intercept Shift+Arrow (e.g. Zed).
        KeyCode::Char('.') if app.focus == Focus::History => {
            app.next_commit();
            app.refresh_details();
        }
        KeyCode::Char(',') if app.focus == Focus::History => {
            app.previous_commit();
            app.refresh_details();
        }
        KeyCode::Down => {
            app.next();
            if app.focus == Focus::Branches {
                app.reload_history_from_branches();
            } else {
                app.reload_history_if_open(false);
            }
            app.refresh_details();
        }
        KeyCode::Up => {
            app.previous();
            if app.focus == Focus::Branches {
                app.reload_history_from_branches();
            } else {
                app.reload_history_if_open(false);
            }
            app.refresh_details();
        }
        KeyCode::PageDown => {
            app.next_page();
            if app.focus == Focus::Branches {
                app.reload_history_from_branches();
            } else {
                app.reload_history_if_open(false);
            }
            app.refresh_details();
        }
        KeyCode::PageUp => {
            app.previous_page();
            if app.focus == Focus::Branches {
                app.reload_history_from_branches();
            } else {
                app.reload_history_if_open(false);
            }
            app.refresh_details();
        }
        KeyCode::Enter => {
            if app.focus == Focus::Branches {
                app.open_branch_action_menu();
            } else if app.focus == Focus::Log && app.show_log {
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
        KeyCode::Char('d') => app.toggle_details(),
        KeyCode::Char('b') => {
            if app.show_branches {
                app.close_branches_pane();
            } else {
                app.open_branches_pane();
            }
        }
        KeyCode::Char('r') => refresh_repos(app),
        KeyCode::Char('T') => app.next_theme(),
        KeyCode::Char('A') => app.enter_pick_mode(),
        KeyCode::Char('D') => app.request_remove_selected(),
        KeyCode::Char('c') => {
            if app.focus == Focus::Branches {
                if let Some(b) = app.selected_branch_info().cloned() {
                    if !b.is_current {
                        let (name, is_remote) = if b.is_remote_only {
                            (format!("origin/{}", b.name), true)
                        } else {
                            (b.name, false)
                        };
                        launch_op(app, op_tx, OpRequest::CheckoutBranch { name, is_remote });
                    }
                }
            } else {
                app.open_branch_select();
            }
        }
        KeyCode::Char('f') => launch_op(app, op_tx, OpRequest::Fetch),
        KeyCode::Char('p') => {
            if let Some(op) = branch_pull_op(app) {
                launch_op(app, op_tx, op);
            } else {
                launch_op(app, op_tx, OpRequest::Pull);
            }
        }
        KeyCode::Char('P') => launch_op(app, op_tx, OpRequest::Push),
        KeyCode::Char('?') => app.mode = AppMode::HelpOverlay,
        KeyCode::Esc => {
            if app.focus == Focus::Branches {
                app.close_branches_pane();
            }
        }
        _ => {}
    }
}
