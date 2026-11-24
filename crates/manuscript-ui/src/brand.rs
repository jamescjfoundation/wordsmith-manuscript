// SPDX-License-Identifier: MIT OR Apache-2.0
//! Wordsmith Manuscript's brand: design tokens, the editor's page colors and the mark.

use jcj_shell::egui::{self, Color32, Painter, Pos2, Rect, Stroke, StrokeKind};
use jcj_shell::theme::Tokens;
use serde::Deserialize;

/// The brand's design tokens (`assets/brand/manuscript/tokens.json`).
pub const TOKENS_JSON: &str = include_str!("../../../assets/brand/manuscript/tokens.json");

/// The app icon at 256 pixels, for the window.
pub const ICON_256: &[u8] = include_bytes!("../../../assets/brand/manuscript/icon-256.png");

/// The brand's tokens; the core's neutral tokens if the file could not be read (never expected:
/// a test reads it).
pub fn tokens() -> Tokens {
    Tokens::from_json(TOKENS_JSON).unwrap_or_default()
}

/// Install the brand's fonts and theme in a context the shell does not manage (tests, offscreen
/// pictures). The shell does this itself for the app's window.
pub fn apply(ctx: &egui::Context, mode: jcj_shell::theme::Mode) {
    let tokens = tokens();
    let (names, faces) = jcj_shell::theme::with_defaults(&tokens.fonts, &[]);
    jcj_shell::theme::install_fonts(ctx, &names, &faces);
    jcj_shell::theme::apply(ctx, &tokens.theme(mode));
}

/// The editor's page colors for one theme.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PageColors {
    /// The page behind the text.
    pub page: Color32,
    /// The text.
    pub text: Color32,
    /// Secondary marks: dividers, scene breaks, hints.
    pub dim: Color32,
    /// The selection.
    pub selection: Color32,
}

/// The editor surface: follows the window's theme, or sepia (ACC-1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, serde::Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EditorSurface {
    /// The window theme's page (light or dark).
    #[default]
    Theme,
    /// A warm sepia page, in any window theme.
    Sepia,
}

#[derive(Deserialize)]
struct RawPage {
    page: String,
    text: String,
    dim: String,
    selection: String,
}

#[derive(Deserialize)]
struct RawEditor {
    editor_font: String,
    sepia: RawPage,
    light: RawPage,
    dark: RawPage,
}

#[derive(Deserialize)]
struct RawFile {
    manuscript: RawEditor,
}

fn hex(s: &str) -> Color32 {
    let h = s.trim_start_matches('#');
    let byte = |i: usize| h.get(i..i + 2).and_then(|b| u8::from_str_radix(b, 16).ok()).unwrap_or(0);
    Color32::from_rgb(byte(0), byte(2), byte(4))
}

fn colors(p: &RawPage) -> PageColors {
    PageColors { page: hex(&p.page), text: hex(&p.text), dim: hex(&p.dim), selection: hex(&p.selection) }
}

const FALLBACK: PageColors =
    PageColors { page: Color32::WHITE, text: Color32::from_rgb(0x1F, 0x1F, 0x22), dim: Color32::GRAY, selection: Color32::LIGHT_BLUE };

/// The editor's colors for a surface and the window's darkness.
pub fn page_colors(surface: EditorSurface, dark: bool) -> PageColors {
    let Ok(raw) = serde_json::from_str::<RawFile>(TOKENS_JSON) else { return FALLBACK };
    match (surface, dark) {
        (EditorSurface::Sepia, _) => colors(&raw.manuscript.sepia),
        (EditorSurface::Theme, false) => colors(&raw.manuscript.light),
        (EditorSurface::Theme, true) => colors(&raw.manuscript.dark),
    }
}

/// The editor's text face name (a font bundled with the core).
pub fn editor_font() -> String {
    serde_json::from_str::<RawFile>(TOKENS_JSON).map(|r| r.manuscript.editor_font).unwrap_or_else(|_| "Source Serif 4".into())
}

/// Paint the mark (a page with lines of text and a turned corner, as `mark.svg`) in `rect` in one
/// ink color, as the shell asks for it in the quick-access row and the About page.
pub fn paint_mark(painter: &Painter, rect: Rect, ink: Color32) {
    let s = rect.width().min(rect.height()) / 64.0;
    let at = |x: f32, y: f32| Pos2::new(rect.left() + x * s, rect.top() + y * s);
    let page = vec![at(17.0, 12.0), at(39.0, 12.0), at(47.0, 20.0), at(47.0, 52.0), at(17.0, 52.0)];
    painter.add(egui::Shape::closed_line(page, Stroke::new(3.0 * s, ink)));
    painter.add(egui::Shape::convex_polygon(vec![at(39.0, 12.0), at(47.0, 20.0), at(39.0, 20.0)], ink, Stroke::NONE));
    for (y, w) in [(26.0, 20.0), (33.0, 20.0), (40.0, 13.0)] {
        let r = Rect::from_min_max(at(22.0, y), at(22.0 + w, y + 3.0));
        painter.rect(r, 1.5 * s, ink, Stroke::NONE, StrokeKind::Inside);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tokens_file_reads_and_names_the_brand() {
        assert!(Tokens::from_json(TOKENS_JSON).is_ok());
        assert!(serde_json::from_str::<RawFile>(TOKENS_JSON).is_ok());
        assert_eq!(editor_font(), "Source Serif 4");
    }

    #[test]
    fn sepia_differs_from_light_and_dark() {
        let sepia = page_colors(EditorSurface::Sepia, false);
        assert_ne!(sepia, page_colors(EditorSurface::Theme, false));
        assert_ne!(sepia, page_colors(EditorSurface::Theme, true));
        assert_eq!(sepia, page_colors(EditorSurface::Sepia, true));
        assert_eq!(sepia.page, Color32::from_rgb(0xF4, 0xEC, 0xD8));
    }

    #[test]
    fn the_icon_is_a_png() {
        assert_eq!(&ICON_256[..8], b"\x89PNG\r\n\x1a\n");
    }
}
