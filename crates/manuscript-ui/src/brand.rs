// SPDX-License-Identifier: MIT OR Apache-2.0
//! Wordsmith Manuscript's brand: design tokens, the editor's page colors and the mark.

use jcj_shell::egui::{self, Color32, Painter, Pos2, Rect, Stroke};
use jcj_shell::theme::Tokens;
use serde::Deserialize;

/// The brand's design tokens (`assets/brand/tokens.json`).
pub const TOKENS_JSON: &str = include_str!("../../../assets/brand/tokens.json");

/// The app icon at 256 pixels, for the window.
pub const ICON_256: &[u8] = include_bytes!("../../../assets/brand/icon-256.png");

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

/// Paint the mark (the letter M and the cursor of three sheets, as
/// `assets/brand/wordsmith-manuscript-mark-ink.svg`) in `rect` in one ink color, as the shell asks
/// for it in the quick-access row and the About page. The letter is drawn from straight pieces, close
/// to the outlined original at the sizes the shell uses; the two sheets behind are lighter.
pub fn paint_mark(painter: &Painter, rect: Rect, ink: Color32) {
    // The artwork's box, in the SVG's units: 389 wide, 362 high, starting at y = -41.78.
    const W: f32 = 388.93;
    const H: f32 = 361.58;
    const TOP: f32 = -41.78;
    const MID: f32 = 119.95; // The letter's axis.
    let s = (rect.width() / W).min(rect.height() / H);
    let origin = rect.center() - egui::vec2(W, H) * s / 2.0;
    let at = |x: f32, y: f32| Pos2::new(origin.x + x * s, origin.y + (y - TOP) * s);
    let mirror = |x: f32| 2.0 * MID - x;
    let block = |x0: f32, y0: f32, x1: f32, y1: f32| Rect::from_two_pos(at(x0, y0), at(x1, y1));

    // The cursor: each sheet stands 61.2 wide and 239.84 high, a step up and to the right of the
    // one in front. Only the part a sheet shows is painted, so the lighter ones never overlap.
    let (step_x, step_y, width, height) = (37.95, 20.87, 61.2, 239.84);
    for (i, tone) in [(0.0_f32, 1.0_f32), (1.0, 0.6), (2.0, 0.3)] {
        let (x, y) = (251.84 + i * step_x, -i * step_y);
        let color = ink.gamma_multiply(tone);
        if i == 0.0 {
            painter.rect_filled(block(x, y, x + width, y + height), 0.0, color);
        } else {
            let front_right = x - step_x + width;
            painter.rect_filled(block(front_right, y, x + width, y + height), 0.0, color);
            painter.rect_filled(block(x, y, front_right, y + step_y), 0.0, color);
        }
    }

    // The letter: each half is a slab serif above and below a stem, and a diagonal to the middle.
    let round = 12.0 * s;
    for side in [false, true] {
        let x = |v: f32| if side { mirror(v) } else { v };
        let shape = |points: &[(f32, f32)]| {
            let mut pts: Vec<Pos2> = points.iter().map(|&(px, py)| at(x(px), py)).collect();
            if side {
                pts.reverse();
            }
            egui::Shape::convex_polygon(pts, ink, Stroke::NONE)
        };
        painter.rect_filled(block(x(5.86), 0.0, x(73.63), 40.04), round, ink);
        painter.rect_filled(block(x(1.95), 191.8, x(99.61), 231.84), round, ink);
        painter.add(shape(&[(28.71, 30.0), (70.6, 30.0), (69.14, 200.0), (25.78, 200.0)]));
        painter.add(shape(&[(66.0, 0.0), (80.37, 1.37), (MID, 101.95), (MID, 168.36), (102.73, 162.7), (70.31, 76.76)]));
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
