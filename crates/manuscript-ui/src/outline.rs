// SPDX-License-Identifier: MIT OR Apache-2.0
//! The outline (PLN-2, SPEC 7.2): a table of a folder's descendants with title and synopsis,
//! label, status, words, target, point of view and keywords; sortable as a view, editable in
//! place, rows dragged to reorder the binder.

use std::collections::HashSet;

use jcj_shell::egui::Ui;
use serde_json::{Value, json};
use wordsmith_manuscript_model::{Area, ItemId, POV_FIELD, Place, Project};

use crate::commands::{Spec, id_param};
use crate::session::{Outcome, Session};
use crate::table::{Cell, Column, Edit, Sort, TableEvent, TableRow, TableState, sorted, table};

/// The outline's columns, in order.
pub const COLUMNS: [&str; 7] = ["Title", "Label", "Status", "Words", "Target", POV_FIELD, "Keywords"];

/// Where an outline or corkboard starts: the item when it holds children, else its parent; the
/// Manuscript area when there is neither.
pub fn root_of(p: &Project, selected: Option<ItemId>) -> Option<ItemId> {
    let id = selected?;
    let item = p.binder.get(id)?;
    if !item.children.is_empty() || item.kind == wordsmith_manuscript_model::Kind::Folder {
        return Some(id);
    }
    p.binder.place_of(id).and_then(|pl| pl.parent)
}

/// The items an outline of `root` shows, depth first, with their depth below it.
pub fn items(p: &Project, root: Option<ItemId>, hidden: &HashSet<ItemId>) -> Vec<(ItemId, usize)> {
    let rows = match root {
        Some(r) => {
            let Some(base) = p.binder.depth_of(r) else { return Vec::new() };
            let all = p.binder.rows();
            let start = all.iter().position(|row| row.item.id == r);
            let Some(start) = start else { return Vec::new() };
            all.into_iter()
                .skip(start + 1)
                .take_while(|row| row.depth + 1 > base)
                .map(|row| (row.item.id, row.depth + 1 - base - 1))
                .collect::<Vec<_>>()
        }
        None => p.binder.rows_of(Area::Manuscript).into_iter().map(|row| (row.item.id, row.depth)).collect(),
    };
    rows.into_iter().filter(|(id, _)| !hidden.contains(id)).collect()
}

fn hex(s: &str) -> Option<jcj_shell::egui::Color32> {
    let h = s.strip_prefix('#')?;
    let b = |i: usize| h.get(i..i + 2).and_then(|x| u8::from_str_radix(x, 16).ok());
    Some(jcj_shell::egui::Color32::from_rgb(b(0)?, b(2)?, b(4)?))
}

/// The table rows of an outline.
pub fn rows(p: &Project, list: &[(ItemId, usize)]) -> Vec<TableRow<ItemId>> {
    list.iter()
        .filter_map(|(id, depth)| {
            let item = p.binder.get(*id)?;
            let meta = p.text(*id).map(|t| t.meta.clone()).unwrap_or_default();
            let words = p.words_under(*id);
            let target = meta.target;
            Some(TableRow {
                key: *id,
                depth: *depth,
                dim: !item.include,
                cells: vec![
                    Cell {
                        text: if item.title.is_empty() { "Untitled".into() } else { item.title.clone() },
                        second: meta.synopsis.clone(),
                        ..Cell::default()
                    },
                    Cell {
                        text: item.label.clone().unwrap_or_default(),
                        color: item.label.as_deref().and_then(|l| p.settings.label_color(l)).and_then(hex),
                        ..Cell::default()
                    },
                    Cell::text(item.status.clone().unwrap_or_default()),
                    Cell::text(crate::binder::format_words(words)),
                    Cell {
                        text: target.map(|t| crate::binder::format_words(t as usize)).unwrap_or_default(),
                        progress: target.filter(|t| *t > 0).map(|t| words as f32 / t as f32),
                        ..Cell::default()
                    },
                    Cell::text(meta.fields.get(POV_FIELD).cloned().unwrap_or_default()),
                    Cell::text(meta.keywords.join(", ")),
                ],
            })
        })
        .collect()
}

fn columns(p: &Project) -> Vec<Column> {
    let mut labels: Vec<String> = vec![String::new()];
    labels.extend(p.settings.labels.iter().map(|l| l.name.clone()));
    let mut statuses: Vec<String> = vec![String::new()];
    statuses.extend(p.settings.statuses.iter().cloned());
    vec![
        Column::new("Title and synopsis", 320.0).edit(Edit::Text),
        Column::new("Label", 110.0).edit(Edit::Choice(labels)),
        Column::new("Status", 120.0).edit(Edit::Choice(statuses)),
        Column::new("Words", 70.0).numeric(),
        Column::new("Target", 80.0).numeric().edit(Edit::Text),
        Column::new(POV_FIELD, 110.0).edit(Edit::Text),
        Column::new("Keywords", 160.0),
    ]
}

/// The outline's state.
#[derive(Default)]
pub struct OutlineView {
    /// The table's state (sort, selection).
    pub table: TableState<ItemId>,
}

/// What the outline asks of the app.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ask {
    /// Select an item.
    Select(ItemId),
    /// Open an item in the editor.
    Open(ItemId),
}

/// Draw the outline of `root`. Returns what the app should do and any errors.
pub fn view(
    ui: &mut Ui,
    s: &mut Session,
    v: &mut OutlineView,
    root: Option<ItemId>,
    hidden: &HashSet<ItemId>,
    selected: Option<ItemId>,
) -> (Vec<Ask>, Vec<String>) {
    let mut asks = Vec::new();
    let mut errors = Vec::new();
    let list = items(&s.project, root, hidden);
    let rows = rows(&s.project, &list);
    let cols = columns(&s.project);
    let title = root.and_then(|r| s.project.binder.get(r)).map_or_else(|| "Manuscript".to_owned(), |i| i.title.clone());
    ui.horizontal(|ui| {
        ui.heading(title);
        if let Some(sort) = v.table.sort
            && ui.button("Apply This Order to the Binder").clicked()
            && let Err(e) =
                s.run("outline.applySort", &json!({ "root": root.map(|r| r.as_text()), "column": sort.column, "descending": sort.descending }))
        {
            errors.push(e);
        }
    });
    v.table.selected = selected;
    for e in table(ui, "manuscript-outline", &mut v.table, &cols, &rows) {
        let r = match e {
            TableEvent::Select(id) => {
                asks.push(Ask::Select(id));
                Ok(())
            }
            TableEvent::Activate(id) => {
                asks.push(Ask::Open(id));
                Ok(())
            }
            TableEvent::Edit { key, column, value } => edit(s, key, column, value),
            TableEvent::Move { key, target, after } => {
                let Some(tp) = s.project.binder.place_of(target) else { continue };
                let index = if after { tp.index + 1 } else { tp.index };
                let mut p = json!({ "id": key.as_text(), "area": tp.area.key(), "index": index });
                if let Some(parent) = tp.parent {
                    p["parent"] = json!(parent.as_text());
                }
                s.run("binder.move", &p).map(|_| ())
            }
        };
        if let Err(e) = r {
            errors.push(e);
        }
    }
    (asks, errors)
}

fn edit(s: &mut Session, id: ItemId, column: usize, value: String) -> Result<(), String> {
    let i = id.as_text();
    let r = match column {
        0 => s.run("binder.rename", &json!({ "id": i, "title": value })),
        1 => s.run("item.setLabel", &json!({ "id": i, "label": if value.is_empty() { Value::Null } else { json!(value) } })),
        2 => s.run("item.setStatus", &json!({ "id": i, "status": if value.is_empty() { Value::Null } else { json!(value) } })),
        4 => {
            let words: Option<u64> = value.trim().replace(',', "").parse().ok();
            if words.is_none() && !value.trim().is_empty() {
                return Err(format!("`{value}` is not a number of words"));
            }
            s.run("item.setTarget", &json!({ "id": i, "words": words }))
        }
        5 => s.run("item.setField", &json!({ "id": i, "field": POV_FIELD, "value": value })),
        _ => Ok(json!({})),
    };
    r.map(|_| ())
}

/// The outline's commands.
pub fn specs() -> Vec<Spec> {
    vec![
        Spec {
            id: "outline.list",
            label: "Outline",
            key: "",
            params: r#"{"root"?: id, "sort"?: n, "descending"?: bool}"#,
            about: "The outline of an item (or the manuscript): each row's id, depth and columns.",
            mutates: false,
        },
        Spec {
            id: "outline.applySort",
            label: "Apply Outline Order",
            key: "",
            params: r#"{"root"?: id, "column": n, "descending"?: bool}"#,
            about: "Reorder the binder by an outline column, within each folder (one undo step).",
            mutates: true,
        },
    ]
}

/// Run an outline command, or `None` if `id` is not one.
pub fn run(s: &mut Session, id: &str, v: &Value) -> Option<Outcome> {
    let root = |v: &Value| -> Result<Option<ItemId>, String> {
        match v.get("root") {
            None | Some(Value::Null) => Ok(None),
            Some(_) => id_param(v, "root").map(Some),
        }
    };
    Some(match id {
        "outline.list" => (|| {
            let r = root(v)?;
            let list = items(&s.project, r, &HashSet::new());
            let sort = v
                .get("sort")
                .and_then(Value::as_u64)
                .map(|c| Sort { column: c as usize, descending: v.get("descending").and_then(Value::as_bool).unwrap_or(false) });
            let rows = sorted(&rows(&s.project, &list), &columns(&s.project), sort);
            Ok(Value::Array(
                rows.iter()
                    .map(|row| {
                        let mut o = json!({ "id": row.key.as_text(), "depth": row.depth, "synopsis": row.cells.first().map(|c| c.second.clone()) });
                        for (n, name) in COLUMNS.iter().enumerate() {
                            o[*name] = json!(row.cells.get(n).map(|c| c.text.clone()).unwrap_or_default());
                        }
                        o
                    })
                    .collect(),
            ))
        })(),
        "outline.applySort" => (|| {
            let r = root(v)?;
            let column = v.get("column").and_then(Value::as_u64).ok_or("`column` is required")? as usize;
            let sort = Sort { column, descending: v.get("descending").and_then(Value::as_bool).unwrap_or(false) };
            let cols = columns(&s.project);
            let list = items(&s.project, r, &HashSet::new());
            // Each parent's children in the sorted order, parents in binder order.
            let mut parents: Vec<(Option<ItemId>, Area)> = Vec::new();
            for (id, _) in &list {
                if let Some(pl) = s.project.binder.place_of(*id)
                    && !parents.contains(&(pl.parent, pl.area))
                {
                    parents.push((pl.parent, pl.area));
                }
            }
            let mut plan: Vec<(ItemId, Place)> = Vec::new();
            for (parent, area) in parents {
                let kids: Vec<(ItemId, usize)> = list
                    .iter()
                    .filter(|(id, _)| s.project.binder.place_of(*id).is_some_and(|pl| pl.parent == parent && pl.area == area))
                    .copied()
                    .collect();
                let ordered = sorted(&rows(&s.project, &kids), &cols, Some(sort));
                for (n, row) in ordered.iter().enumerate() {
                    plan.push((row.key, Place { area, parent, index: n }));
                }
            }
            s.change("Apply Outline Order", &[], &[], |p| {
                for (id, place) in plan {
                    let from = p.binder.place_of(id).ok_or("an item vanished")?;
                    // The target index counts siblings after the items already placed.
                    let index = if from.parent == place.parent && from.area == place.area && from.index < place.index {
                        place.index + 1
                    } else {
                        place.index
                    };
                    p.binder.move_to(id, Place { index, ..place }).map_err(|e| e.to_string())?;
                }
                Ok(())
            })?;
            Ok(json!({}))
        })(),
        _ => return None,
    })
}
