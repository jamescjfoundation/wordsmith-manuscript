// SPDX-License-Identifier: MIT OR Apache-2.0
//! Import commands: what the io layer reads, placed in the project as one undo step, with its
//! report.

use std::path::Path;

use serde_json::{Value, json};
use wordsmith_manuscript_io::notes;
use wordsmith_manuscript_model::{Area, Item, ItemId, Kind, Place, Project, bible};

use crate::commands::{Spec, str_param};
use crate::session::{Outcome, Session};

/// The import commands.
pub fn specs() -> Vec<Spec> {
    vec![Spec {
        id: "import.notes",
        label: "Import Notes Folder",
        key: "",
        params: r#"{"path": string}"#,
        about: "Each Markdown note in a folder becomes a bible entry; front matter becomes fields; [[links]] become links (one undo step).",
        mutates: true,
    }]
}

/// The folder at the end of `path` (titles) under the Story Bible, made where missing.
fn bible_folder(p: &mut Project, path: &[String]) -> Result<Option<ItemId>, String> {
    let mut parent: Option<ItemId> = None;
    for title in path {
        let existing = match parent {
            None => p.binder.area(Area::Bible).iter().find(|i| i.kind == Kind::Folder && &i.title == title).map(|i| i.id),
            Some(pid) => p.binder.get(pid).and_then(|it| it.children.iter().find(|i| i.kind == Kind::Folder && &i.title == title).map(|i| i.id)),
        };
        parent = Some(match existing {
            Some(id) => id,
            None => {
                let f = Item::new(Kind::Folder, title.clone());
                let id = f.id;
                let place = parent.map_or(Place::end_of(Area::Bible), |pid| Place::inside(Area::Bible, pid));
                p.binder.insert(place, f).map_err(|e| e.to_string())?;
                id
            }
        });
    }
    Ok(parent)
}

fn import_notes(s: &mut Session, path: &Path) -> Outcome {
    let templates = bible::templates(&s.project.settings);
    let types: Vec<(&str, &str)> = templates.iter().map(|t| (t.kind.as_str(), t.plural.as_str())).collect();
    let mut folder = notes::read_folder(path, &types, &mut ItemId::new)?;
    let others: Vec<_> = s.project.entries.iter().map(|(i, e)| (*i, e.clone())).collect();
    notes::resolve(&mut folder, &others);
    notes::check_words(path, &mut folder);
    let ids: Vec<ItemId> = folder.notes.iter().map(|n| n.id).collect();
    let plural = |kind: &str| templates.iter().find(|t| t.kind == kind).map_or_else(|| "Lore".to_owned(), |t| t.plural.clone());
    let report = folder.report.clone();
    s.change("Import Notes", &[], &ids, |p| {
        for note in folder.notes {
            let path = if note.folders.is_empty() { vec![plural(&note.entry.kind)] } else { note.folders.clone() };
            let parent = bible_folder(p, &path)?;
            let place = parent.map_or(Place::end_of(Area::Bible), |pid| Place::inside(Area::Bible, pid));
            p.binder.insert(place, Item::with_id(note.id, Kind::Entry, note.entry.name.clone())).map_err(|e| e.to_string())?;
            p.entries.insert(note.id, note.entry);
        }
        Ok(())
    })?;
    Ok(json!({
        "notes": report.notes,
        "links_resolved": report.resolved,
        "links_unresolved": report.unresolved.iter().map(|(n, name)| json!({ "note": n, "name": name })).collect::<Vec<_>>(),
        "skipped": report.skipped.iter().map(|(n, why)| json!({ "file": n, "why": why })).collect::<Vec<_>>(),
        "word_check": if report.word_mismatches.is_empty() { json!("every word arrived") } else { json!(report.word_mismatches) },
    }))
}

/// Run an import command, or `None` if `id` is not one.
pub fn run(s: &mut Session, id: &str, v: &Value) -> Option<Outcome> {
    Some(match id {
        "import.notes" => str_param(v, "path").map(|p| p.to_owned()).and_then(|p| import_notes(s, Path::new(&p))),
        _ => return None,
    })
}
