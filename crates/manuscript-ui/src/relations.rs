// SPDX-License-Identifier: MIT OR Apache-2.0
//! Relationships between bible entries (BIB-4): typed links ("sibling of", "works for"), shown
//! from both sides. A relation is kept on the entry it starts from; the other entry shows it by
//! its inverse ("employs" for "works for"), or by the same name when it goes both ways.

use jcj_shell::egui::{self, RichText, Ui};
use serde_json::{Value, json};
use wordsmith_manuscript_model::{ItemId, Project};

use crate::commands::{Spec, id_param, str_param};
use crate::session::{Outcome, Session};

/// The relations offered, each with its inverse (the same name when it goes both ways).
pub const KINDS: [(&str, &str); 12] = [
    ("sibling of", "sibling of"),
    ("married to", "married to"),
    ("friend of", "friend of"),
    ("rival of", "rival of"),
    ("parent of", "child of"),
    ("child of", "parent of"),
    ("works for", "employs"),
    ("employs", "works for"),
    ("member of", "has member"),
    ("lives in", "home of"),
    ("owns", "owned by"),
    ("part of", "has part"),
];

/// How a relation reads from the other entry.
pub fn inverse(kind: &str) -> String {
    KINDS.iter().find(|(k, _)| *k == kind).map_or_else(|| format!("{kind} (from)"), |(_, i)| (*i).to_owned())
}

/// An entry's relations from both sides: (relation as read from this entry, the other entry,
/// whether it is kept on this entry).
pub fn both_ways(p: &Project, id: ItemId) -> Vec<(String, ItemId, bool)> {
    // Relations to an entry that is gone (emptied from the Trash) are not shown.
    let mut out: Vec<(String, ItemId, bool)> = p
        .entries
        .get(&id)
        .map(|e| e.relations.iter().filter(|(_, to)| p.entries.contains_key(to)).map(|(k, to)| (k.clone(), *to, true)).collect())
        .unwrap_or_default();
    for (other, e) in &p.entries {
        for (k, to) in &e.relations {
            if *to == id && *other != id {
                out.push((inverse(k), *other, false));
            }
        }
    }
    out
}

/// The relation commands.
pub fn specs() -> Vec<Spec> {
    let spec = |id, label, params, about, mutates| Spec { id, label, key: "", params, about, mutates };
    vec![
        spec(
            "entry.relate",
            "Relate Entries",
            r#"{"id": id, "relation": string, "to": id}"#,
            "Link an entry to another with a relation (\"sibling of\", \"works for\", or your own).",
            true,
        ),
        spec("entry.unrelate", "Remove Relation", r#"{"id": id, "relation": string, "to": id}"#, "Remove a relation (kept on `id`).", true),
        spec(
            "entry.relations",
            "Relationships",
            r#"{"id": id}"#,
            "An entry's relations from both sides: relation, other entry, and which side keeps it.",
            false,
        ),
    ]
}

/// Run a relation command, or `None` if `id` is not one.
pub fn run(s: &mut Session, id: &str, v: &Value) -> Option<Outcome> {
    Some(match id {
        "entry.relate" | "entry.unrelate" => (|| {
            let from = id_param(v, "id")?;
            let to = id_param(v, "to")?;
            let kind = str_param(v, "relation")?.trim().to_lowercase();
            if kind.is_empty() {
                return Err("a relation needs a name".into());
            }
            for e in [from, to] {
                if !s.project.entries.contains_key(&e) {
                    return Err(format!("{e} is not a bible entry"));
                }
            }
            if from == to {
                return Err("an entry cannot be related to itself".into());
            }
            let adding = id == "entry.relate";
            let has = s.project.entries[&from].relations.iter().any(|(k, t)| *k == kind && *t == to);
            if adding && has {
                return Err("they are already related so".into());
            }
            if !adding && !has {
                return Err("there is no such relation".into());
            }
            s.change(if adding { "Relate Entries" } else { "Remove Relation" }, &[], &[from], |p| {
                if let Some(e) = p.entries.get_mut(&from) {
                    if adding {
                        e.relations.push((kind, to));
                    } else {
                        e.relations.retain(|(k, t)| !(*k == kind && *t == to));
                    }
                }
                Ok(())
            })?;
            Ok(json!({}))
        })(),
        "entry.relations" => (|| {
            let i = id_param(v, "id")?;
            Ok(json!(both_ways(&s.project, i)
                .into_iter()
                .map(|(k, o, own)| json!({ "relation": k, "entry": o.as_text(), "name": s.project.entries.get(&o).map(|e| e.name.clone()).unwrap_or_default(), "kept_here": own }))
                .collect::<Vec<_>>()))
        })(),
        _ => return None,
    })
}

/// The relationships part of the entry form: the relations, and adding one.
#[derive(Default)]
pub struct RelationsEdit {
    kind: String,
    to: Option<ItemId>,
}

/// Draw the relationships section. Returns commands to run and entries to open.
pub fn section(ui: &mut Ui, p: &Project, id: ItemId, edit: &mut RelationsEdit) -> (Vec<(&'static str, Value)>, Vec<ItemId>) {
    let mut todo = Vec::new();
    let mut open = Vec::new();
    let rels = both_ways(p, id);
    ui.collapsing(RichText::new(format!("{} ({})", jcj_shell::i18n::tr("Relationships"), rels.len())), |ui| {
        for (k, other, own) in &rels {
            ui.horizontal(|ui| {
                ui.label(RichText::new(k).weak());
                let name = p.entries.get(other).map(|e| e.name.clone()).unwrap_or_default();
                if ui.link(&name).clicked() {
                    open.push(*other);
                }
                let (from, kind, to) = if *own { (id, k.clone(), *other) } else { (*other, inverse(k), id) };
                // The relation is kept on one entry; removing it from either side removes it.
                let kept = if *own {
                    kind.clone()
                } else {
                    p.entries.get(&from).and_then(|e| e.relations.iter().find(|(_, t)| *t == to).map(|(k, _)| k.clone())).unwrap_or(kind)
                };
                if ui.small_button("×").on_hover_text("Remove this relation").clicked() {
                    todo.push(("entry.unrelate", json!({ "id": from.as_text(), "relation": kept, "to": to.as_text() })));
                }
            });
        }
        ui.horizontal(|ui| {
            let c = egui::ComboBox::from_id_salt("relation-kind")
                .selected_text(if edit.kind.is_empty() { "Relation…".to_owned() } else { edit.kind.clone() })
                .show_ui(ui, |ui| {
                    for (k, _) in KINDS {
                        ui.selectable_value(&mut edit.kind, k.to_owned(), k);
                    }
                });
            c.response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::ComboBox, true, "Relation"));
            let others: Vec<(ItemId, String)> = p.entries.iter().filter(|(i, _)| **i != id).map(|(i, e)| (*i, e.name.clone())).collect();
            let shown = edit.to.and_then(|t| p.entries.get(&t)).map_or_else(|| "Entry…".to_owned(), |e| e.name.clone());
            let c = egui::ComboBox::from_id_salt("relation-to").selected_text(shown).show_ui(ui, |ui| {
                for (i, name) in &others {
                    ui.selectable_value(&mut edit.to, Some(*i), name);
                }
            });
            c.response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::ComboBox, true, "Related entry"));
            if ui.add_enabled(!edit.kind.is_empty() && edit.to.is_some(), egui::Button::new("Relate")).clicked()
                && let Some(to) = edit.to
            {
                todo.push(("entry.relate", json!({ "id": id.as_text(), "relation": edit.kind, "to": to.as_text() })));
                *edit = RelationsEdit::default();
            }
        });
    });
    (todo, open)
}
