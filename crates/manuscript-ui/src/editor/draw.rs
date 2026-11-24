// SPDX-License-Identifier: MIT OR Apache-2.0
//! Laying out and drawing the editor's paragraphs, and finding text under the pointer.

use std::sync::Arc;

use jcj_doc::Paragraph;
use jcj_shell::egui::text::{CCursor, LayoutJob, TextFormat};
use jcj_shell::egui::{self, Align, Align2, FontFamily, FontId, Galley, Pos2, Rect, Sense, Stroke, Ui, pos2, vec2};
use wordsmith_manuscript_model::doc::{self as mdoc, BlockTag};

use super::{Editor, Laid, para_index};
use crate::brand::{EditorSurface, PageColors};

/// The editor's own families, made from the serif faces the core bundles.
pub const SERIF: &str = "manuscript-serif";
/// Bold.
pub const SERIF_BOLD: &str = "manuscript-serif-bold";
/// Italic.
pub const SERIF_ITALIC: &str = "manuscript-serif-italic";
/// Bold italic.
pub const SERIF_BOLD_ITALIC: &str = "manuscript-serif-bold-italic";

/// Install the editor's faces once per context.
pub fn install_fonts(ctx: &egui::Context) {
    let done = egui::Id::unique("manuscript-editor-fonts");
    if ctx.data(|d| d.get_temp::<bool>(done)).unwrap_or(false) {
        return;
    }
    let family = crate::brand::editor_font();
    for (name, style) in [(SERIF, "Regular"), (SERIF_BOLD, "Bold"), (SERIF_ITALIC, "Italic"), (SERIF_BOLD_ITALIC, "Bold Italic")] {
        if let Some(face) = jcj_layout::fonts::bundled_face(&family, style) {
            ctx.add_font(egui::epaint::text::FontInsert {
                name: name.to_owned(),
                data: egui::FontData::from_static(face.bytes),
                families: vec![egui::epaint::text::InsertFontFamily {
                    family: FontFamily::Name(name.into()),
                    priority: egui::epaint::text::FontPriority::Highest,
                }],
            });
            // Characters the serif lacks fall back to the interface font.
            ctx.add_font(egui::epaint::text::FontInsert {
                name: format!("{name}-fallback"),
                data: egui::FontData::from_static(jcj_layout::fonts::bundled_face("Inter", "Regular").map_or(face.bytes, |f| f.bytes)),
                families: vec![egui::epaint::text::InsertFontFamily {
                    family: FontFamily::Name(name.into()),
                    priority: egui::epaint::text::FontPriority::Lowest,
                }],
            });
        }
    }
    ctx.data_mut(|d| d.insert_temp(done, true));
}

/// How the editor looks: size, width, line spacing, surface.
#[derive(Clone, Debug, PartialEq)]
pub struct EditorLook {
    /// Body text size in points.
    pub size: f32,
    /// The text column's width in points.
    pub width: f32,
    /// Line height as a multiple of the size.
    pub line: f32,
    /// The page's surface.
    pub surface: EditorSurface,
    /// Whether the window is dark.
    pub dark: bool,
    /// The scene break's symbol in the editor.
    pub scene_break: String,
    /// Paragraphs other than the caret's are dimmed (focus mode at 0.2).
    pub dim_others: bool,
}

impl Default for EditorLook {
    fn default() -> Self {
        EditorLook { size: 17.0, width: 680.0, line: 1.55, surface: EditorSurface::Theme, dark: false, scene_break: "#".into(), dim_others: false }
    }
}

impl EditorLook {
    fn colors(&self) -> PageColors {
        crate::brand::page_colors(self.surface, self.dark)
    }
}

fn family(bold: bool, italic: bool) -> FontFamily {
    FontFamily::Name(
        match (bold, italic) {
            (false, false) => SERIF,
            (true, false) => SERIF_BOLD,
            (false, true) => SERIF_ITALIC,
            (true, true) => SERIF_BOLD_ITALIC,
        }
        .into(),
    )
}

/// The layout job of one paragraph.
fn job(
    p: &Paragraph,
    tag: &BlockTag,
    first: bool,
    look: &EditorLook,
    width: f32,
    colors: &PageColors,
    passes: &[egui::Color32],
) -> (LayoutJob, f32, f32, f32) {
    let (size, bold_all, center, indent, inset, above, below) = match tag {
        BlockTag::Heading { level } => {
            let size = look.size
                * match level {
                    1 => 1.6,
                    2 => 1.35,
                    _ => 1.15,
                };
            (size, true, true, 0.0, 0.0, look.size * 0.9, look.size * 0.5)
        }
        BlockTag::Quote { .. } => (look.size, false, false, 0.0, look.size * 2.0, 2.0, 2.0),
        BlockTag::Verse { .. } => (look.size, false, false, 0.0, look.size * 2.5, look.size * 0.3, look.size * 0.3),
        BlockTag::Aside { .. } | BlockTag::EmptyFence { .. } => (look.size, false, false, 0.0, look.size * 1.5, 2.0, 2.0),
        BlockTag::Note { .. } | BlockTag::EmptyNote { .. } => (look.size * 0.82, false, false, 0.0, look.size * 1.5, 2.0, 2.0),
        _ => (look.size, false, false, if first { 0.0 } else { look.size * 1.5 }, 0.0, 0.0, 0.0),
    };
    let mut job =
        LayoutJob { wrap: egui::text::TextWrapping { max_width: (width - inset * 2.0).max(60.0), ..Default::default() }, ..Default::default() };
    if center {
        job.halign = Align::Center;
    }
    let line_height = Some(size * look.line);
    let mut lead = indent;
    for (range, props) in p.run_ranges() {
        let Some(text) = p.text.get(range) else { continue };
        let m = mdoc::marks_of(props);
        let note = mdoc::NOTE_REF.is_set(props);
        let mut f = TextFormat {
            font_id: FontId::new(if note { size * 0.7 } else { size }, family(m.bold || bold_all, m.italic)),
            color: colors.text,
            line_height,
            ..TextFormat::default()
        };
        if note {
            f.valign = Align::TOP;
            f.color = colors.dim;
        }
        if m.strike {
            f.strikethrough = Stroke::new(1.0, colors.text);
        }
        if m.link.is_some() {
            f.underline = Stroke::new(1.0, colors.dim);
        }
        if m.comment.is_some() {
            f.background = colors.selection.gamma_multiply(0.5);
        } else if let Some(c) = m.rev.and_then(|r| passes.get(r as usize)) {
            // Text of a revision pass, in the pass's color.
            f.background = c.gamma_multiply(0.6);
        }
        let shown = if m.small_caps { text.to_uppercase() } else { text.to_owned() };
        if m.small_caps {
            f.font_id.size = size * 0.8;
        }
        // Small caps change the text's length; the hit-test maps by characters, which stay the
        // same count for the scripts small caps are used with.
        job.append(&shown, lead, f);
        lead = 0.0;
    }
    if p.text.is_empty() {
        job.append(
            "",
            indent,
            TextFormat { font_id: FontId::new(size, family(bold_all, false)), color: colors.text, line_height, ..TextFormat::default() },
        );
    }
    (job, inset, above, below)
}

/// The block kinds that start a fresh run of body text (the next paragraph has no indent).
fn breaks_flow(tag: &BlockTag) -> bool {
    !matches!(tag, BlockTag::Body)
}

impl Editor {
    /// Lay out paragraph `i` (from the cache when it did not change).
    fn laid(&mut self, ui: &Ui, i: usize, look: &EditorLook, colors: &PageColors) -> Option<Laid> {
        let width = look.width;
        let first = i == 0 || self.paragraph(i - 1).is_none_or(|prev| breaks_flow(&mdoc::tag_of(prev)) || crate::stitch::is_divider(prev));
        let p = self.paragraph(i)?;
        let rev = p.rev;
        if let Some(l) = self.cache.get(&i)
            && l.rev == rev
            && (l.width - width).abs() < 0.5
            && l.first == first
        {
            return Some(l.clone());
        }
        let tag = mdoc::tag_of(p);
        let laid = if crate::stitch::is_divider(p) {
            let title = crate::stitch::divider_title(p);
            let galley = ui.painter().layout_no_wrap(title, FontId::proportional(look.size * 0.75), colors.dim);
            Laid { rev, width, first, galley, above: look.size * 1.2, below: look.size * 0.6, inset: 0.0 }
        } else {
            match &tag {
                BlockTag::SceneBreak | BlockTag::Image { .. } | BlockTag::Raw { .. } => {
                    let label = match &tag {
                        BlockTag::SceneBreak => look.scene_break.clone(),
                        BlockTag::Image { alt, role, .. } => format!("Picture ({}): {alt}", role.key()),
                        BlockTag::Raw { format, .. } => format!("Raw {format} block"),
                        _ => String::new(),
                    };
                    let mut j = LayoutJob::simple_singleline(label, FontId::new(look.size, family(false, false)), colors.dim);
                    j.halign = Align::Center;
                    j.wrap.max_width = width;
                    let galley = ui.painter().layout_job(j);
                    Laid { rev, width, first, galley, above: look.size * 0.5, below: look.size * 0.5, inset: 0.0 }
                }
                _ => {
                    let (j, inset, above, below) = job(p, &tag, first, look, width, colors, &self.pass_colors);
                    let galley = ui.painter().layout_job(j);
                    Laid { rev, width, first, galley, above, below, inset }
                }
            }
        };
        if let Some(h) = self.heights.get_mut(i) {
            *h = laid.above + laid.galley.size().y + laid.below;
        }
        self.cache.insert(i, laid.clone());
        Some(laid)
    }

    fn estimate(&self, i: usize, look: &EditorLook) -> f32 {
        let chars = self.paragraph(i).map_or(0, |p| p.text.chars().count()) as f32;
        let per_line = (look.width / (look.size * 0.5)).max(10.0);
        ((chars / per_line).ceil().max(1.0)) * look.size * look.line
    }

    /// Draw the editor in `ui`. Returns the response of the text area.
    pub fn ui(&mut self, ui: &mut Ui, look: &EditorLook) -> egui::Response {
        install_fonts(ui.ctx());
        let colors = look.colors();
        let n = self.paragraphs().len();
        if self.heights.len() != n {
            self.heights = (0..n).map(|i| self.estimate(i, look)).collect();
            self.cache.retain(|k, _| *k < n);
        }
        let id = ui.make_persistent_id("manuscript-editor");
        let area = ui.max_rect();
        ui.painter().rect_filled(area, 0.0, colors.page);
        let margin = 40.0;
        let total: f32 = self.heights.iter().sum::<f32>() + margin * 2.0 + if self.typewriter { area.height() * 0.5 } else { 0.0 };
        let caret_para = para_index(&self.session.sel.focus);
        let mut scroll = egui::ScrollArea::vertical().id_salt("manuscript-editor-scroll").auto_shrink([false, false]);
        if self.reveal {
            let mut y = margin;
            for h in self.heights.iter().take(caret_para) {
                y += h;
            }
            let view = area.height();
            let target = if self.typewriter { (y - view * 0.45).max(0.0) } else { (y - view * 0.3).max(0.0) };
            let current = ui.ctx().data(|d| d.get_temp::<f32>(id.with("offset"))).unwrap_or(0.0);
            let caret_h = self.heights.get(caret_para).copied().unwrap_or(0.0);
            if self.typewriter || y < current || y + caret_h > current + view - 20.0 {
                scroll = scroll.vertical_scroll_offset(target);
            }
            self.reveal = false;
        }
        let mut resp_out = None;
        let out = scroll.show_viewport(ui, |ui, viewport| {
            let full = Rect::from_min_size(ui.min_rect().min, vec2(ui.available_width(), total));
            let resp = ui.interact(full, id, Sense::click_and_drag());
            ui.allocate_rect(full, Sense::hover());
            let left = full.min.x + ((full.width() - look.width) / 2.0).max(margin);
            let mut y = full.min.y + margin;
            let (sa, sb) = self.session.sel.ordered();
            let mut hits: Vec<(usize, Rect, Arc<Galley>, f32)> = Vec::new();
            for i in 0..n {
                let h = self.heights.get(i).copied().unwrap_or(0.0);
                let top_rel = y - full.min.y;
                if top_rel + h < viewport.min.y - 200.0 || top_rel > viewport.max.y + 200.0 {
                    y += h;
                    continue;
                }
                let Some(l) = self.laid(ui, i, look, &colors) else { break };
                let h = l.above + l.galley.size().y + l.below;
                let origin = pos2(left + l.inset, y + l.above);
                let gal_rect = Rect::from_min_size(origin, vec2(look.width - l.inset * 2.0, l.galley.size().y));
                // Selection.
                if !self.session.sel.is_collapsed() && i >= para_index(&sa) && i <= para_index(&sb) {
                    let text_len = self.paragraph(i).map_or(0, |p| p.text.len());
                    let from = if i == para_index(&sa) { sa.off } else { 0 };
                    let to = if i == para_index(&sb) { sb.off } else { text_len };
                    let p = self.paragraph(i);
                    let (fc, tc) = p.map_or((0, 0), |p| (char_index(&p.text, from), char_index(&p.text, to)));
                    for r in selection_rects(&l.galley, fc, tc, i != para_index(&sb)) {
                        ui.painter().rect_filled(r.translate(origin.to_vec2()), 0.0, colors.selection);
                    }
                }
                let marked: Vec<(usize, usize)> = match self
                    .paragraph(i)
                    .map(|p| (p.rev, p.text.clone(), mdoc::is_text(&mdoc::tag_of(p)) && !crate::stitch::is_divider(p)))
                {
                    Some((rev, text, true)) => match self.spell.as_mut() {
                        Some(spell) => spell.of(i, rev, &text).into_iter().map(|r| (char_index(&text, r.start), char_index(&text, r.end))).collect(),
                        None => Vec::new(),
                    },
                    _ => Vec::new(),
                };
                let dim = look.dim_others && i != caret_para;
                let p = self.paragraph(i);
                let divider = p.is_some_and(crate::stitch::is_divider);
                let tint = if dim { colors.dim } else { colors.text };
                if divider {
                    let mid = origin.y + l.galley.size().y / 2.0;
                    ui.painter().line_segment([pos2(left, mid), pos2(origin.x - 8.0, mid)], Stroke::new(1.0, colors.dim));
                    let gx = left + 12.0;
                    ui.painter().galley(pos2(gx, origin.y), l.galley.clone(), colors.dim);
                    ui.painter().line_segment([pos2(gx + l.galley.size().x + 8.0, mid), pos2(left + look.width, mid)], Stroke::new(1.0, colors.dim));
                } else {
                    ui.painter().galley_with_override_text_color(origin, l.galley.clone(), tint);
                    for (a, b) in &marked {
                        for rect in selection_rects(&l.galley, *a, *b, false) {
                            squiggle(ui.painter(), rect.translate(origin.to_vec2()), egui::Color32::from_rgb(0xC0, 0x39, 0x2B));
                        }
                    }
                    if p.is_some_and(|p| matches!(mdoc::tag_of(p), BlockTag::Aside { .. } | BlockTag::EmptyFence { verse: false, .. })) {
                        ui.painter().line_segment([pos2(left + l.inset * 0.5, y), pos2(left + l.inset * 0.5, y + h)], Stroke::new(2.0, colors.dim));
                    }
                    if let Some(BlockTag::Note { label, .. } | BlockTag::EmptyNote { label, .. }) = p.map(mdoc::tag_of) {
                        ui.painter().text(
                            pos2(origin.x - 8.0, origin.y),
                            Align2::RIGHT_TOP,
                            format!("{label}."),
                            FontId::proportional(look.size * 0.75),
                            colors.dim,
                        );
                    }
                }
                hits.push((i, gal_rect, l.galley.clone(), y));
                // The caret.
                if i == caret_para && resp.has_focus() && !divider {
                    let off = self.session.sel.focus.off;
                    let c = p.map_or(0, |p| char_index(&p.text, off));
                    let r = l.galley.pos_from_cursor(CCursor::new(c)).translate(origin.to_vec2());
                    let caret = Rect::from_min_max(pos2(r.min.x, r.min.y), pos2(r.min.x + 1.5, r.max.y));
                    ui.painter().rect_filled(caret, 0.0, colors.text);
                    self.caret_rect = Some(caret);
                }
                y += h;
            }
            if let Some(p) = resp.interact_pointer_pos() {
                self.pointer(ui, &resp, p, &hits);
            }
            self.hover_link = None;
            if let Some(p) = resp.hover_pos()
                && let Some((i, off)) = self.hit(p, &hits)
            {
                let link = self.paragraph(i).and_then(|q| mdoc::LINK.get_or_none(q.props_of_char(off.min(q.text.len().saturating_sub(1)))));
                if let Some(l) = link {
                    if resp.clicked() && ui.input(|inp| inp.modifiers.command) {
                        self.follow = Some(l.clone());
                    }
                    self.hover_link = Some((l, p));
                }
                if resp.secondary_clicked() {
                    self.open_menu(i, off, p);
                }
            }
            if resp.clicked() || resp.drag_started() {
                resp.request_focus();
            }
            resp_out = Some(resp);
        });
        ui.ctx().data_mut(|d| d.insert_temp(id.with("offset"), out.state.unclamped_offset().y));
        let resp = resp_out.unwrap_or_else(|| ui.interact(area, id, Sense::hover()));
        if resp.has_focus() {
            ui.memory_mut(|m| {
                m.set_focus_lock_filter(id, egui::EventFilter { tab: true, horizontal_arrows: true, vertical_arrows: true, escape: false })
            });
            self.input(ui, look);
        }
        resp
    }

    /// The paragraph and byte under a point.
    fn hit(&self, p: Pos2, hits: &[(usize, Rect, Arc<Galley>, f32)]) -> Option<(usize, usize)> {
        let (i, rect, galley, _) = hits.iter().find(|h| h.1.expand2(vec2(0.0, 2.0)).contains(p))?;
        let c = galley.cursor_from_pos(p - rect.min);
        let text = &self.paragraph(*i)?.text;
        Some((*i, byte_offset(text, c.index.0)))
    }

    /// Open the spelling menu when the word under the pointer is marked.
    fn open_menu(&mut self, i: usize, off: usize, at: Pos2) {
        let Some(p) = self.paragraph(i) else { return };
        let (rev, text) = (p.rev, p.text.clone());
        let Some(spell) = self.spell.as_mut() else { return };
        if let Some(r) = spell.of(i, rev, &text).into_iter().find(|r| r.start <= off && off <= r.end) {
            self.menu = Some((i, r, at));
        }
    }

    /// Draw the spelling menu, if open.
    pub fn menu_ui(&mut self, ctx: &egui::Context) {
        let Some((i, range, at)) = self.menu.clone() else { return };
        let word = self.paragraph(i).and_then(|p| p.text.get(range.clone())).unwrap_or("").to_owned();
        let lang = self.spell.as_ref().map_or_else(|| "en".to_owned(), |s| s.lang.clone());
        let mut close = false;
        let mut pick: Option<String> = None;
        let area = egui::Area::new(egui::Id::unique("manuscript-spelling-menu")).order(egui::Order::Foreground).fixed_pos(at).show(ctx, |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.set_min_width(160.0);
                for s in jcj_proof::suggestions(&word, 6, Some(&lang)) {
                    if ui.button(&s).clicked() {
                        pick = Some(s);
                    }
                }
                ui.separator();
                if ui.button("Add to Project Words").clicked() {
                    self.add_word = Some(word.clone());
                    close = true;
                }
                if ui.button("Ignore All").clicked() {
                    jcj_proof::ignore_word(&word);
                    close = true;
                }
            });
        });
        if let Some(s) = pick {
            self.session.sel = jcj_engine::Selection { anchor: super::pos(i, range.start), focus: super::pos(i, range.end) };
            if let Err(e) = self.guarded(&s, true, &|sess, t| crate::editor::pasted(sess, t)) {
                self.error = Some(e);
            }
            close = true;
        }
        if close || ctx.input(|i| i.key_pressed(egui::Key::Escape)) || (ctx.input(|i| i.pointer.any_pressed()) && !area.response.contains_pointer()) {
            self.menu = None;
        }
    }

    fn pointer(&mut self, ui: &Ui, resp: &egui::Response, p: Pos2, hits: &[(usize, Rect, Arc<Galley>, f32)]) {
        if resp.secondary_clicked() {
            return;
        }
        let Some((i, rect, galley, _)) = hits.iter().min_by(|a, b| {
            let da = dist_y(a.1, p.y);
            let db = dist_y(b.1, p.y);
            da.total_cmp(&db)
        }) else {
            return;
        };
        if self.paragraph(*i).is_some_and(crate::stitch::is_divider) {
            return;
        }
        let local = p - rect.min;
        let c = galley.cursor_from_pos(local);
        let text = self.paragraph(*i).map(|p| p.text.clone()).unwrap_or_default();
        let off = byte_offset(&text, c.index.0);
        let shift = ui.input(|i| i.modifiers.shift);
        if resp.double_clicked() {
            self.set_caret(*i, off, false);
            let _ = self.session.run("select.word", &serde_json::json!({}));
        } else if resp.drag_started() || resp.clicked() {
            self.set_caret(*i, off, shift);
        } else if resp.dragged() {
            self.set_caret(*i, off, true);
        }
        self.goal_x = None;
        self.reveal = false;
    }

    /// The caret's position inside its galley and the galley, for moving up and down.
    pub(super) fn caret_galley(&self) -> Option<(usize, Arc<Galley>, Rect)> {
        let i = para_index(&self.session.sel.focus);
        let l = self.cache.get(&i)?;
        let p = self.paragraph(i)?;
        let c = char_index(&p.text, self.session.sel.focus.off);
        Some((i, l.galley.clone(), l.galley.pos_from_cursor(CCursor::new(c))))
    }

    /// The galley of paragraph `i`, if laid out.
    pub(super) fn galley(&self, i: usize) -> Option<Arc<Galley>> {
        self.cache.get(&i).map(|l| l.galley.clone())
    }

    /// The colors the editor uses for a look (tests).
    pub fn colors_for(look: &EditorLook) -> PageColors {
        look.colors()
    }
}

/// A wavy line under a rectangle (a spelling mark).
fn squiggle(painter: &egui::Painter, r: Rect, color: egui::Color32) {
    let y = r.max.y - 1.0;
    let mut pts = Vec::new();
    let mut x = r.min.x;
    let mut up = true;
    while x < r.max.x {
        pts.push(pos2(x, if up { y - 1.5 } else { y + 0.5 }));
        x += 2.5;
        up = !up;
    }
    if pts.len() > 1 {
        painter.add(egui::Shape::line(pts, Stroke::new(1.0, color)));
    }
}

fn dist_y(r: Rect, y: f32) -> f32 {
    if y < r.min.y {
        r.min.y - y
    } else if y > r.max.y {
        y - r.max.y
    } else {
        0.0
    }
}

/// The character index of byte `off` in `s`.
pub fn char_index(s: &str, off: usize) -> usize {
    let off = off.min(s.len());
    s.char_indices().take_while(|(b, _)| *b < off).count()
}

/// The byte offset of character `c` in `s`.
pub fn byte_offset(s: &str, c: usize) -> usize {
    s.char_indices().nth(c).map_or(s.len(), |(b, _)| b)
}

/// The rectangles covering characters `from..to` of a galley (galley coordinates). `to_end` also
/// covers the end of the last row (the selection continues into the next paragraph).
fn selection_rects(g: &Galley, from: usize, to: usize, to_end: bool) -> Vec<Rect> {
    let mut out = Vec::new();
    if g.rows.is_empty() {
        return out;
    }
    let a = g.pos_from_cursor(CCursor::new(from));
    let b = g.pos_from_cursor(CCursor::new(to));
    let right = g.rect.max.x.max(g.size().x);
    if (a.min.y - b.min.y).abs() < 1.0 {
        let x1 = if to_end { right } else { b.min.x };
        out.push(Rect::from_min_max(pos2(a.min.x, a.min.y), pos2(x1.max(a.min.x + 2.0), a.max.y)));
        return out;
    }
    out.push(Rect::from_min_max(pos2(a.min.x, a.min.y), pos2(right, a.max.y)));
    if b.min.y > a.max.y + 0.5 {
        out.push(Rect::from_min_max(pos2(0.0, a.max.y), pos2(right, b.min.y)));
    }
    let x1 = if to_end { right } else { b.min.x };
    out.push(Rect::from_min_max(pos2(0.0, b.min.y), pos2(x1, b.max.y)));
    out
}
