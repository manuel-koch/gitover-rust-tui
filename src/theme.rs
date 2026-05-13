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

use ratatui::style::Color;

use crate::git::{DeltaKind, FileStatusKind};

/// All semantic colors used by the UI, collected in one place so themes can be
/// swapped at runtime without touching any draw logic.
#[derive(Debug, Clone)]
pub struct Theme {
    // ── Chrome ────────────────────────────────────────────────────────────────
    pub border_focused: Color,
    pub border_unfocused: Color,

    // ── Header ────────────────────────────────────────────────────────────────
    pub title: Color,
    pub spinner: Color,
    pub refresh_info: Color,
    pub auto_fetch_info: Color,

    // ── Repository table ──────────────────────────────────────────────────────
    pub table_header: Color,
    pub repo_clean: Color,
    pub repo_dirty: Color,
    pub branch: Color,
    pub error: Color,
    pub placeholder: Color,

    // ── Status counts ─────────────────────────────────────────────────────────
    pub status_staged: Color,
    pub status_conflict: Color,
    pub status_modified: Color,
    pub status_deleted: Color,
    pub status_untracked: Color,
    pub status_clean_text: Color,

    // ── Upstream / Trunk ──────────────────────────────────────────────────────
    pub sync_warning: Color, // out-of-sync upstream, ahead-only trunk
    pub sync_ok: Color,      // synced (shows dim)
    pub trunk_behind: Color, // trunk: behind > 0

    // ── Activity spinner ──────────────────────────────────────────────────────
    pub activity: Color,

    // ── Selection highlight ───────────────────────────────────────────────────
    pub selection_fg: Color,
    pub selection_bg: Color,

    // ── Output Log ────────────────────────────────────────────────────────────
    pub log_timestamp: Color,

    // ── Git History pane ──────────────────────────────────────────────────────
    pub history_hash: Color,
    pub history_timestamp: Color,
    pub history_author: Color,
    pub history_scroll_info: Color,

    // ── Delta (file changes in history) ───────────────────────────────────────
    pub delta_added: Color,
    pub delta_modified: Color,
    pub delta_deleted: Color,
    pub delta_renamed: Color,
    pub delta_other: Color,

    // ── Popups ────────────────────────────────────────────────────────────────
    pub popup_border: Color,
    pub popup_border_danger: Color,
    pub popup_target: Color,
    pub popup_confirm: Color,
    pub popup_cancel: Color,
    pub popup_confirm_danger: Color,
    pub popup_empty: Color,
    pub input_text: Color,

    // ── Help bar / hints ──────────────────────────────────────────────────────
    pub help_key: Color,
    pub help_key_confirm: Color,

    // ── Branch select ─────────────────────────────────────────────────────────
    pub branch_remote: Color,
    pub branch_local: Color,
}

impl Theme {
    /// Return the color for a file-status kind (detail panel).
    pub fn file_status_colour(&self, kind: &FileStatusKind) -> Color {
        match kind {
            FileStatusKind::Staged => self.status_staged,
            FileStatusKind::Modified => self.status_modified,
            FileStatusKind::Deleted => self.status_deleted,
            FileStatusKind::Conflict => self.status_conflict,
            FileStatusKind::Untracked => self.status_untracked,
        }
    }

    /// Return the color for a delta kind (history sub-rows).
    pub fn delta_colour(&self, kind: &DeltaKind) -> Color {
        match kind {
            DeltaKind::Added => self.delta_added,
            DeltaKind::Modified => self.delta_modified,
            DeltaKind::Deleted => self.delta_deleted,
            DeltaKind::Renamed => self.delta_renamed,
            DeltaKind::Other => self.delta_other,
        }
    }
}

// ── Built-in themes ───────────────────────────────────────────────────────────

/// Default dark theme — the original color scheme.
pub const DEFAULT: Theme = Theme {
    border_focused: Color::Cyan,
    border_unfocused: Color::DarkGray,

    title: Color::Cyan,
    spinner: Color::Yellow,
    refresh_info: Color::DarkGray,
    auto_fetch_info: Color::LightBlue,

    table_header: Color::Yellow,
    repo_clean: Color::Green,
    repo_dirty: Color::White,
    branch: Color::Cyan,
    error: Color::Red,
    placeholder: Color::DarkGray,

    sync_warning: Color::Yellow,
    sync_ok: Color::DarkGray,
    trunk_behind: Color::Red,

    activity: Color::Yellow,

    selection_fg: Color::Black,
    selection_bg: Color::Cyan,

    log_timestamp: Color::DarkGray,

    history_hash: Color::Yellow,
    history_timestamp: Color::DarkGray,
    history_author: Color::Cyan,
    history_scroll_info: Color::DarkGray,

    status_staged: Color::Blue,
    status_conflict: Color::Yellow,
    status_modified: Color::Green,
    status_deleted: Color::Red,
    status_untracked: Color::Cyan,
    status_clean_text: Color::DarkGray,

    delta_added: Color::Blue,     // matches status_staged
    delta_modified: Color::Green, // matches status_modified
    delta_deleted: Color::Red,    // matches status_deleted
    delta_renamed: Color::Blue,   // align delta_renamed with staged style
    delta_other: Color::Gray,     // align with untracked

    popup_border: Color::Cyan,
    popup_border_danger: Color::Red,
    popup_target: Color::Yellow,
    popup_confirm: Color::Green,
    popup_cancel: Color::Yellow,
    popup_confirm_danger: Color::Red,
    popup_empty: Color::DarkGray,
    input_text: Color::Cyan,

    help_key: Color::Yellow,
    help_key_confirm: Color::Green,

    branch_remote: Color::Yellow,
    branch_local: Color::DarkGray,
};

/// All available themes in cycle order.
pub const THEMES: &[&Theme] = &[&DEFAULT];
