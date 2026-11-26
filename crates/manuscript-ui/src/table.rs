// SPDX-License-Identifier: MIT OR Apache-2.0
//! A table of rows with columns the caller defines: sort by a column, edit a cell in place, drag
//! rows to reorder, rows indented as a tree, and only the rows in view drawn.
//!
//! core-candidate: open-source-core#7 (C6). Nothing here knows about manuscripts.

use std::hash::Hash;

use jcj_shell::egui::{self, Align2, Color32, FontId, Rect, Sense, Stroke, Ui, pos2, vec2};
use jcj_shell::theme::Theme;

/// How a column's cells are edited.
#[derive(Clone, Debug, PartialEq)]
pub enum Edit {
    /// Not editable.
    No,
    /// One line of text.
    Text,
    /// One of these choices (the empty string clears).
    Choice(Vec<String>),
}

/// A column.
#[derive(Clone, Debug)]
pub struct Column {
    /// The heading.
    pub title: String,
    /// The width in points.
    pub width: f32,
    /// Sortable by clicking the heading.
    pub sortable: bool,
    /// How cells are edited.
    pub edit: Edit,
    /// Sort as numbers rather than text.
    pub numeric: bool,
}

impl Column {
    /// A plain, sortable, read-only text column.
    pub fn new(title: &str, width: f32) -> Column {
        Column { title: title.into(), width, sortable: true, edit: Edit::No, numeric: false }
    }

    /// The same column, editable.
    pub fn edit(mut self, e: Edit) -> Column {
        self.edit = e;
        self
    }

    /// The same column, sorted as numbers.
    pub fn numeric(mut self) -> Column {
        self.numeric = true;
        self
    }
}

/// A cell: its text, a second line shown under it (in the first column), a color mark, and a
/// progress fraction drawn as a bar.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Cell {
    /// The text.
    pub text: String,
    /// A second, dimmer line.
    pub second: String,
    /// A color swatch before the text.
    pub color: Option<Color32>,
    /// A bar from 0 to 1 behind the text.
    pub progress: Option<f32>,
}

impl Cell {
    /// A text cell.
    pub fn text(t: impl Into<String>) -> Cell {
        Cell { text: t.into(), ..Cell::default() }
    }
}

/// A row.
#[derive(Clone, Debug)]
pub struct TableRow<K> {
    /// The row's key.
    pub key: K,
    /// How deep (indents the first column).
    pub depth: usize,
    /// One cell per column.
    pub cells: Vec<Cell>,
    /// Drawn dimmed.
    pub dim: bool,
}

/// The sort: column and direction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sort {
    /// The column.
    pub column: usize,
    /// Descending.
    pub descending: bool,
}

/// What happened this frame.
#[derive(Clone, Debug, PartialEq)]
pub enum TableEvent<K> {
    /// A row was clicked.
    Select(K),
    /// A row was double-clicked.
    Activate(K),
    /// A cell was edited.
    Edit {
        /// The row.
        key: K,
        /// The column.
        column: usize,
        /// The new value.
        value: String,
    },
    /// A row was dragged onto another: before it (`after` false) or after it.
    Move {
        /// The row dragged.
        key: K,
        /// The row dropped on.
        target: K,
        /// After the target rather than before.
        after: bool,
    },
}

/// The table's own state.
#[derive(Clone, Debug)]
pub struct TableState<K> {
    /// The sort, if any (a view: the rows' real order is unchanged).
    pub sort: Option<Sort>,
    /// The selected row.
    pub selected: Option<K>,
    editing: Option<(K, usize, String, bool)>,
    dragging: Option<K>,
}

impl<K> Default for TableState<K> {
    fn default() -> Self {
        TableState { sort: None, selected: None, editing: None, dragging: None }
    }
}

/// Order rows by a column (stable; numbers compare as numbers, text without case).
pub fn sorted<K: Clone>(rows: &[TableRow<K>], columns: &[Column], sort: Option<Sort>) -> Vec<TableRow<K>> {
    let mut out = rows.to_vec();
    let Some(s) = sort else { return out };
    let numeric = columns.get(s.column).is_some_and(|c| c.numeric);
    out.sort_by(|a, b| {
        let x = a.cells.get(s.column).map_or("", |c| c.text.as_str());
        let y = b.cells.get(s.column).map_or("", |c| c.text.as_str());
        let o = if numeric {
            let n = |t: &str| {
                t.replace(',', "")
                    .split(|c: char| !c.is_ascii_digit() && c != '.')
                    .next()
                    .and_then(|v| v.parse::<f64>().ok())
                    .unwrap_or(f64::NEG_INFINITY)
            };
            n(x).total_cmp(&n(y))
        } else {
            x.to_lowercase().cmp(&y.to_lowercase())
        };
        if s.descending { o.reverse() } else { o }
    });
    out
}

/// The row height (two lines).
pub const ROW_HEIGHT: f32 = 38.0;

/// Draw the table.
pub fn table<K: Clone + PartialEq + Hash + std::fmt::Debug + Send + Sync + 'static>(
    ui: &mut Ui,
    id_salt: &str,
    state: &mut TableState<K>,
    columns: &[Column],
    rows: &[TableRow<K>],
) -> Vec<TableEvent<K>> {
    let t = Theme::get(ui.ctx());
    let mut events = Vec::new();
    // Headings.
    let (head, _) = ui.allocate_exact_size(vec2(ui.available_width(), 24.0), Sense::hover());
    ui.painter().rect_filled(head, 0.0, t.surface_high);
    let mut x = head.min.x + 6.0;
    for (ci, c) in columns.iter().enumerate() {
        let r = Rect::from_min_size(pos2(x, head.min.y), vec2(c.width, head.height()));
        let resp = ui.interact(r, ui.scope_id().with((id_salt, "head", ci)), Sense::click());
        let arrow = match state.sort {
            Some(s) if s.column == ci => {
                if s.descending {
                    " ▼"
                } else {
                    " ▲"
                }
            }
            _ => "",
        };
        ui.painter().text(
            pos2(r.min.x + 4.0, r.center().y),
            Align2::LEFT_CENTER,
            format!("{}{arrow}", c.title),
            FontId::proportional(12.0),
            t.text_dim,
        );
        let title = c.title.clone();
        resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::ColumnHeader, true, &title));
        if c.sortable && resp.clicked() {
            state.sort = match state.sort {
                Some(s) if s.column == ci && !s.descending => Some(Sort { column: ci, descending: true }),
                Some(s) if s.column == ci => None,
                _ => Some(Sort { column: ci, descending: false }),
            };
        }
        x += c.width;
    }
    let shown = sorted(rows, columns, state.sort);
    let pointer = ui.input(|i| i.pointer.interact_pos());
    let released = ui.input(|i| i.pointer.any_released());
    let mut drop_on: Option<(usize, bool)> = None;
    egui::ScrollArea::vertical().id_salt(id_salt).auto_shrink([false, false]).show_rows(ui, ROW_HEIGHT, shown.len(), |ui, range| {
        for i in range {
            let Some(row) = shown.get(i) else { continue };
            let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), ROW_HEIGHT), Sense::hover());
            let resp = ui.interact(
                rect,
                ui.scope_id().with((id_salt, "row", i)),
                if state.sort.is_none() { Sense::click_and_drag() } else { Sense::click() },
            );
            let selected = state.selected.as_ref() == Some(&row.key);
            if selected {
                ui.painter().rect_filled(rect, 0.0, t.accent_soft);
            } else if resp.hovered() {
                ui.painter().rect_filled(rect, 0.0, t.hover);
            }
            ui.painter().line_segment([pos2(rect.min.x, rect.max.y), pos2(rect.max.x, rect.max.y)], Stroke::new(0.5, t.border));
            let ink = if row.dim { t.text_dim } else { t.text };
            let mut x = rect.min.x + 6.0;
            for (ci, c) in columns.iter().enumerate() {
                let cell = row.cells.get(ci).cloned().unwrap_or_default();
                let indent = if ci == 0 && state.sort.is_none() { row.depth as f32 * 16.0 } else { 0.0 };
                let r = Rect::from_min_size(pos2(x + indent, rect.min.y), vec2((c.width - indent).max(20.0), ROW_HEIGHT));
                let editing_here = state.editing.as_ref().is_some_and(|(k, col, _, _)| k == &row.key && *col == ci);
                if editing_here {
                    let mut done: Option<Option<String>> = None;
                    if let Some((_, _, buf, focus)) = state.editing.as_mut() {
                        match &c.edit {
                            Edit::Choice(choices) => {
                                let mut picked = None;
                                ui.put(r.shrink(4.0), |ui: &mut Ui| {
                                    egui::ComboBox::from_id_salt((id_salt, "choice", i, ci))
                                        .selected_text(buf.clone())
                                        .show_ui(ui, |ui| {
                                            for ch in choices {
                                                if ui.selectable_label(*ch == *buf, if ch.is_empty() { "None" } else { ch }).clicked() {
                                                    picked = Some(ch.clone());
                                                }
                                            }
                                        })
                                        .response
                                });
                                if let Some(p) = picked {
                                    done = Some(Some(p));
                                } else if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                                    done = Some(None);
                                }
                            }
                            _ => {
                                let te = ui.put(
                                    Rect::from_min_size(pos2(r.min.x, r.min.y + 4.0), vec2(r.width() - 6.0, 22.0)),
                                    egui::TextEdit::singleline(buf),
                                );
                                if *focus {
                                    te.request_focus();
                                    *focus = false;
                                }
                                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                                    done = Some(None);
                                } else if te.lost_focus() {
                                    done = Some(Some(buf.clone()));
                                }
                            }
                        }
                    }
                    match done {
                        Some(Some(value)) => {
                            events.push(TableEvent::Edit { key: row.key.clone(), column: ci, value });
                            state.editing = None;
                        }
                        Some(None) => state.editing = None,
                        None => {}
                    }
                } else {
                    let clip = ui.painter().with_clip_rect(r.intersect(rect));
                    let mut tx = r.min.x + 4.0;
                    if let Some(p) = cell.progress {
                        let bar = Rect::from_min_size(pos2(r.min.x + 2.0, r.max.y - 8.0), vec2((r.width() - 10.0) * p.clamp(0.0, 1.0), 3.0));
                        clip.rect_filled(bar, 1.0, t.accent);
                    }
                    if let Some(col) = cell.color {
                        clip.rect_filled(Rect::from_min_size(pos2(tx, r.min.y + 9.0), vec2(8.0, 8.0)), 2.0, col);
                        tx += 12.0;
                    }
                    let (y1, second) = if cell.second.is_empty() { (r.center().y, false) } else { (r.min.y + 12.0, true) };
                    clip.text(pos2(tx, y1), Align2::LEFT_CENTER, &cell.text, FontId::proportional(13.0), ink);
                    if second {
                        clip.text(pos2(tx, r.min.y + 28.0), Align2::LEFT_CENTER, &cell.second, FontId::proportional(11.0), t.text_dim);
                    }
                    let can_edit = c.edit != Edit::No;
                    if can_edit && resp.double_clicked() && pointer.is_some_and(|p| r.contains(p)) {
                        state.editing = Some((row.key.clone(), ci, cell.text.clone(), true));
                    }
                }
                x += c.width;
            }
            let label = row.cells.first().map(|c| c.text.clone()).unwrap_or_default();
            resp.widget_info(|| egui::WidgetInfo::selected(egui::Role::Row, true, selected, &label));
            if resp.clicked() {
                state.selected = Some(row.key.clone());
                events.push(TableEvent::Select(row.key.clone()));
            }
            if resp.double_clicked() && state.editing.is_none() {
                events.push(TableEvent::Activate(row.key.clone()));
            }
            if resp.drag_started() && state.sort.is_none() {
                state.dragging = Some(row.key.clone());
            }
            if state.dragging.is_some()
                && let Some(p) = pointer
                && rect.contains(p)
            {
                let after = p.y > rect.center().y;
                drop_on = Some((i, after));
                let y = if after { rect.max.y - 1.0 } else { rect.min.y + 1.0 };
                ui.painter().line_segment([pos2(rect.min.x, y), pos2(rect.max.x, y)], Stroke::new(2.0, t.accent));
            }
        }
    });
    if released
        && let Some(key) = state.dragging.take()
        && let Some((i, after)) = drop_on
        && let Some(target) = shown.get(i)
        && target.key != key
    {
        events.push(TableEvent::Move { key, target: target.key.clone(), after });
    }
    events
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(k: u32, cells: &[&str]) -> TableRow<u32> {
        TableRow { key: k, depth: 0, cells: cells.iter().map(|c| Cell::text(*c)).collect(), dim: false }
    }

    #[test]
    fn sorting_is_stable_numeric_and_reversible() {
        let cols = vec![Column::new("Title", 100.0), Column::new("Words", 60.0).numeric()];
        let rows = vec![row(1, &["b", "1,200"]), row(2, &["A", "90"]), row(3, &["c", "90"])];
        let keys = |s| sorted(&rows, &cols, s).iter().map(|r| r.key).collect::<Vec<_>>();
        assert_eq!(keys(None), [1, 2, 3]);
        assert_eq!(keys(Some(Sort { column: 0, descending: false })), [2, 1, 3]);
        assert_eq!(keys(Some(Sort { column: 1, descending: false })), [2, 3, 1]);
        assert_eq!(keys(Some(Sort { column: 1, descending: true })), [1, 2, 3]);
    }
}
