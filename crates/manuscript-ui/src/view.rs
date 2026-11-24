// SPDX-License-Identifier: MIT OR Apache-2.0
//! How the writer sees the text (ED-3, ACC-1): focus mode, typewriter scrolling, the sepia page,
//! the editor's text size and width. Kept in the app's preferences, separate from export.

use serde::{Deserialize, Serialize};

use crate::brand::EditorSurface;
use crate::editor::EditorLook;

/// The view preferences (the app's part of the preferences file).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ViewPrefs {
    /// The editor's page: the window theme's, or sepia.
    pub surface: EditorSurface,
    /// Text size in points.
    pub size: f32,
    /// The text column's width in points.
    pub width: f32,
    /// Keep the caret's line at a fixed height.
    pub typewriter: bool,
    /// In focus mode, dim the paragraphs around the caret's.
    pub dim_in_focus: bool,
    /// Show grammar and style hints (ED-9); off unless the writer turns them on.
    pub grammar: bool,
    /// The document converter the writer installed, for other formats (EXP-7).
    pub converter: wordsmith_manuscript_io::external::Converter,
    /// The interface's language (ACC-4): a code from `i18n::LANGUAGES`, or empty to follow the
    /// computer.
    pub language: String,
}

impl Default for ViewPrefs {
    fn default() -> Self {
        ViewPrefs {
            surface: EditorSurface::Theme,
            size: 17.0,
            width: 680.0,
            typewriter: false,
            dim_in_focus: false,
            grammar: false,
            language: String::new(),
            converter: Default::default(),
        }
    }
}

/// The smallest and largest text size and column width.
pub const SIZE: (f32, f32) = (10.0, 36.0);
/// The column width's range.
pub const WIDTH: (f32, f32) = (360.0, 1400.0);

impl ViewPrefs {
    /// The same preferences within their ranges (a hand-edited file cannot ask for absurd values).
    pub fn bounded(mut self) -> ViewPrefs {
        self.size = if self.size.is_finite() { self.size.clamp(SIZE.0, SIZE.1) } else { 17.0 };
        self.width = if self.width.is_finite() { self.width.clamp(WIDTH.0, WIDTH.1) } else { 680.0 };
        self
    }

    /// The editor look for these preferences.
    pub fn look(&self, dark: bool, focus: bool) -> EditorLook {
        EditorLook {
            size: self.size,
            width: self.width,
            surface: self.surface,
            dark,
            dim_others: focus && self.dim_in_focus,
            ..EditorLook::default()
        }
    }
}

/// The shell's chrome as it was before focus mode hid it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Chrome {
    /// The ribbon.
    pub ribbon: bool,
    /// The quick-access row.
    pub quick_row: bool,
    /// The status bar.
    pub status_bar: bool,
}
