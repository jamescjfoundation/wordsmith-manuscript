// SPDX-License-Identifier: MIT OR Apache-2.0
//! A tree of rows with selection, expand and collapse, rename in place, and drag and drop.
//!
//! core-candidate: open-source-core issue #6 (C5). Nothing here knows about manuscripts: the
//! caller passes the visible rows (already flattened, with their depth) and gets events back.
//! Only the rows in view are drawn, so thousands of rows stay fast.

use std::hash::Hash;

use jcj_shell::egui::{self, Align2, Color32, FontId, Rect, Sense, Stroke, StrokeKind, Ui, pos2, vec2};
use jcj_shell::theme::Theme;

/// One visible row.
#[derive(Clone, Debug)]
pub struct TreeRow<K> {
    /// The row's key.
    pub key: K,
    /// How deep (0 at the top).
    pub depth: usize,
    /// The text.
    pub label: String,
    /// A short glyph drawn before the label (an icon character), or empty.
    pub glyph: String,
    /// Whether it has children (draws the open/closed arrow).
    pub has_children: bool,
    /// Whether its children are shown.
    pub open: bool,
    /// Drawn dimmed (left out of export, for example).
    pub dim: bool,
    /// A heading row (an area): bold, not draggable.
    pub heading: bool,
    /// Whether other rows may be dropped inside it.
    pub accepts_children: bool,
    /// Whether it can be dragged.
    pub draggable: bool,
    /// A colored mark at the row's right edge (a label color).
    pub mark: Option<Color32>,
    /// Text at the row's right (a word count).
    pub trailing: String,
}

impl<K> TreeRow<K> {
    /// A plain row.
    pub fn new(key: K, depth: usize, label: impl Into<String>) -> TreeRow<K> {
        TreeRow {
            key,
            depth,
            label: label.into(),
            glyph: String::new(),
            has_children: false,
            open: false,
            dim: false,
            heading: false,
            accepts_children: true,
            draggable: true,
            mark: None,
            trailing: String::new(),
        }
    }
}

/// Where a dragged row lands relative to the row under the pointer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Zone {
    /// Before it, as its previous sibling.
    Before,
    /// After it, as its next sibling.
    After,
    /// Inside it, as its last child.
    Inside,
}

/// What happened in the tree this frame.
#[derive(Clone, Debug, PartialEq)]
pub enum TreeEvent<K> {
    /// A row was clicked.
    Select(K),
    /// A row was double-clicked or Enter was pressed on it.
    Activate(K),
    /// A row's arrow was clicked (or Left/Right pressed).
    Toggle(K),
    /// A row was renamed in place.
    Rename(K, String),
    /// A row was dragged onto another.
    Drop {
        /// The row dragged.
        item: K,
        /// The row it was dropped on.
        target: K,
        /// Where.
        zone: Zone,
    },
    /// The context menu was asked for on a row.
    Menu(K),
}

/// The tree's own state between frames.
#[derive(Clone, Debug)]
pub struct TreeState<K> {
    /// The row being dragged.
    pub dragging: Option<K>,
    /// The row being renamed, and the text so far.
    pub renaming: Option<(K, String)>,
    rename_focus: bool,
    /// The selected row (the caller's choice, mirrored for keyboard moves).
    pub selected: Option<K>,
    /// Scroll so the selected row is in view on the next frame.
    pub reveal: bool,
}

impl<K> Default for TreeState<K> {
    fn default() -> Self {
        TreeState { dragging: None, renaming: None, rename_focus: false, selected: None, reveal: false }
    }
}

impl<K: Clone> TreeState<K> {
    /// Start renaming a row.
    pub fn rename(&mut self, key: K, current: &str) {
        self.renaming = Some((key, current.to_owned()));
        self.rename_focus = true;
    }
}

/// The height of one row.
pub const ROW_HEIGHT: f32 = 22.0;
/// The indent per level.
pub const INDENT: f32 = 16.0;

/// Which zone a pointer at `y` falls in, for a row from `top` of height `h`.
pub fn zone(y: f32, top: f32, h: f32, inside_ok: bool) -> Zone {
    let f = ((y - top) / h).clamp(0.0, 1.0);
    if inside_ok {
        if f < 0.25 {
            Zone::Before
        } else if f > 0.75 {
            Zone::After
        } else {
            Zone::Inside
        }
    } else if f < 0.5 {
        Zone::Before
    } else {
        Zone::After
    }
}

/// Draw the tree and return what happened.
pub fn tree<K: Clone + PartialEq + Hash + std::fmt::Debug + Send + Sync + 'static>(
    ui: &mut Ui,
    id_salt: &str,
    state: &mut TreeState<K>,
    rows: &[TreeRow<K>],
) -> Vec<TreeEvent<K>> {
    let t = Theme::get(ui.ctx());
    let mut events = Vec::new();
    let mut hover_target: Option<(usize, Zone)> = None;
    let pointer = ui.input(|i| i.pointer.interact_pos());
    let released = ui.input(|i| i.pointer.any_released());
    let selected_index = state.selected.as_ref().and_then(|s| rows.iter().position(|r| &r.key == s));
    let mut scroll = egui::ScrollArea::vertical().id_salt(id_salt).auto_shrink([false, false]);
    if state.reveal {
        if let Some(i) = selected_index {
            let h = ui.available_height();
            let y = i as f32 * ROW_HEIGHT;
            scroll = scroll.vertical_scroll_offset((y - h / 2.0).max(0.0));
        }
        state.reveal = false;
    }
    scroll.show_rows(ui, ROW_HEIGHT, rows.len(), |ui, range| {
        for i in range {
            let Some(row) = rows.get(i) else { continue };
            let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), ROW_HEIGHT), Sense::hover());
            let id = ui.scope_id().with((id_salt, i, "row"));
            let resp = ui.interact(rect, id, if row.draggable { Sense::click_and_drag() } else { Sense::click() });
            let selected = state.selected.as_ref() == Some(&row.key);
            if selected {
                ui.painter().rect_filled(rect, 3.0, t.accent_soft);
            } else if resp.hovered() && state.dragging.is_none() {
                ui.painter().rect_filled(rect, 3.0, t.hover);
            }
            let x0 = rect.min.x + 4.0 + row.depth as f32 * INDENT;
            // The open/closed arrow.
            let arrow = Rect::from_min_size(pos2(x0, rect.min.y), vec2(14.0, ROW_HEIGHT));
            if row.has_children {
                let c = arrow.center();
                let pts = if row.open {
                    vec![pos2(c.x - 4.0, c.y - 2.0), pos2(c.x + 4.0, c.y - 2.0), pos2(c.x, c.y + 3.0)]
                } else {
                    vec![pos2(c.x - 2.0, c.y - 4.0), pos2(c.x + 3.0, c.y), pos2(c.x - 2.0, c.y + 4.0)]
                };
                ui.painter().add(egui::Shape::convex_polygon(pts, t.text_dim, Stroke::NONE));
            }
            let ink = if row.dim { t.text_dim } else { t.text };
            let mut x = x0 + 16.0;
            if !row.glyph.is_empty() {
                ui.painter().text(pos2(x, rect.center().y), Align2::LEFT_CENTER, &row.glyph, FontId::proportional(12.0), t.text_dim);
                x += 18.0;
            }
            let renaming_this = state.renaming.as_ref().is_some_and(|(k, _)| k == &row.key);
            if renaming_this {
                let edit_rect = Rect::from_min_max(pos2(x, rect.min.y + 1.0), pos2(rect.max.x - 4.0, rect.max.y - 1.0));
                let mut done = None;
                if let Some((_, text)) = state.renaming.as_mut() {
                    let r = ui.put(edit_rect, egui::TextEdit::singleline(text).id(id.with("rename")));
                    if state.rename_focus {
                        r.request_focus();
                        state.rename_focus = false;
                    }
                    let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
                    let escape = ui.input(|i| i.key_pressed(egui::Key::Escape));
                    if escape {
                        done = Some(None);
                    } else if r.lost_focus() || enter {
                        done = Some(Some(text.clone()));
                    }
                }
                match done {
                    Some(Some(text)) => {
                        events.push(TreeEvent::Rename(row.key.clone(), text));
                        state.renaming = None;
                    }
                    Some(None) => state.renaming = None,
                    None => {}
                }
            } else {
                let font = if row.heading { FontId::proportional(12.5) } else { FontId::proportional(13.0) };
                let galley = ui.painter().layout_no_wrap(row.label.clone(), font, ink);
                let max_w = (rect.max.x - x - 8.0 - if row.trailing.is_empty() { 0.0 } else { 48.0 }).max(10.0);
                let clip = Rect::from_min_max(pos2(x, rect.min.y), pos2(x + max_w, rect.max.y));
                ui.painter().with_clip_rect(clip).galley(pos2(x, rect.center().y - galley.size().y / 2.0), galley, ink);
                if !row.trailing.is_empty() {
                    ui.painter().text(
                        pos2(rect.max.x - 10.0, rect.center().y),
                        Align2::RIGHT_CENTER,
                        &row.trailing,
                        FontId::proportional(10.5),
                        t.text_dim,
                    );
                }
            }
            if let Some(c) = row.mark {
                ui.painter().rect_filled(Rect::from_min_size(pos2(rect.max.x - 4.0, rect.min.y + 4.0), vec2(3.0, ROW_HEIGHT - 8.0)), 1.0, c);
            }
            resp.widget_info(|| egui::WidgetInfo::selected(egui::Role::TreeItem, true, selected, &row.label));
            if resp.clicked() {
                if row.has_children && pointer.is_some_and(|p| arrow.contains(p)) {
                    events.push(TreeEvent::Toggle(row.key.clone()));
                } else {
                    events.push(TreeEvent::Select(row.key.clone()));
                }
            }
            if resp.double_clicked() {
                events.push(TreeEvent::Activate(row.key.clone()));
            }
            if resp.secondary_clicked() {
                events.push(TreeEvent::Select(row.key.clone()));
                events.push(TreeEvent::Menu(row.key.clone()));
            }
            if resp.drag_started() && row.draggable {
                state.dragging = Some(row.key.clone());
            }
            if state.dragging.is_some()
                && let Some(p) = pointer
                && rect.contains(p)
            {
                let z = zone(p.y, rect.min.y, ROW_HEIGHT, row.accepts_children);
                hover_target = Some((i, z));
                let line = |y: f32| [pos2(x0, y), pos2(rect.max.x - 4.0, y)];
                match z {
                    Zone::Before => {
                        ui.painter().line_segment(line(rect.min.y + 1.0), Stroke::new(2.0, t.accent));
                    }
                    Zone::After => {
                        ui.painter().line_segment(line(rect.max.y - 1.0), Stroke::new(2.0, t.accent));
                    }
                    Zone::Inside => {
                        ui.painter().rect_stroke(rect.shrink(1.0), 3.0, Stroke::new(1.5, t.accent), StrokeKind::Inside);
                    }
                }
            }
        }
    });
    if released && let Some(item) = state.dragging.take() {
        if let Some((i, zone)) = hover_target
            && let Some(target) = rows.get(i)
            && target.key != item
        {
            events.push(TreeEvent::Drop { item, target: target.key.clone(), zone });
        }
    }
    events
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zones_split_the_row() {
        assert_eq!(zone(0.0, 0.0, 20.0, true), Zone::Before);
        assert_eq!(zone(10.0, 0.0, 20.0, true), Zone::Inside);
        assert_eq!(zone(19.0, 0.0, 20.0, true), Zone::After);
        assert_eq!(zone(9.0, 0.0, 20.0, false), Zone::Before);
        assert_eq!(zone(11.0, 0.0, 20.0, false), Zone::After);
    }
}
