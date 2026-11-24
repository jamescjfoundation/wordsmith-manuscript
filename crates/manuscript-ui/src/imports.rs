// SPDX-License-Identifier: MIT OR Apache-2.0
//! Import commands: what the io layer reads, placed in the project as one undo step, with its
//! report.

use std::path::Path;

use serde_json::{Value, json};
use wordsmith_manuscript_io::import::{self, TextRules};
use wordsmith_manuscript_io::{mapping, notes};
use wordsmith_manuscript_model::{Area, Item, ItemId, Kind, Place, Project, Role, bible};

use crate::commands::{Spec, str_param};
use crate::session::{Outcome, Session};

/// The import commands.
pub fn specs() -> Vec<Spec> {
    vec![
        Spec {
            id: "import.preview",
            label: "Preview Import",
            key: "",
            params: r#"{"path": string, "rules"?: {"heading_roles"?: [role], "chapter_lines"?: bool, "split_scenes"?: bool, "scene_marks"?: [string], "underline_as_italic"?: bool}, "join_lines"?: bool}"#,
            about: "The binder an import would make, its report and its word-for-word check, without changing the project.",
            mutates: false,
        },
        Spec {
            id: "import.document",
            label: "Import Document",
            key: "",
            params: r#"{"path": string, "rules"?: {...}, "join_lines"?: bool, "into"?: id}"#,
            about: "Import a DOCX, ODT, RTF, Markdown, HTML or text file into the manuscript (or a folder) as one undo step; the rules are kept in the project.",
            mutates: true,
        },
        Spec {
            id: "import.notes",
            label: "Import Notes Folder",
            key: "",
            params: r#"{"path": string}"#,
            about: "Each Markdown note in a folder becomes a bible entry; front matter becomes fields; [[links]] become links (one undo step).",
            mutates: true,
        },
    ]
}

/// The rules for an import: given ones, else the project's kept ones, else the defaults.
fn rules_of(s: &Session, v: &Value) -> Result<mapping::Rules, String> {
    match v.get("rules") {
        Some(r) => serde_json::from_value(r.clone()).map_err(|e| format!("rules: {e}")),
        None => Ok(s.project.settings.extra.get("import").and_then(|r| serde_json::from_value(r.clone()).ok()).unwrap_or_default()),
    }
}

/// A plan as JSON for previews and reports.
pub fn plan_json(plan: &mapping::Plan) -> Value {
    json!({
        "items": plan.items.iter().map(|i| json!({ "title": i.title, "role": i.role.key(), "kind": format!("{:?}", i.kind).to_lowercase(), "depth": i.depth, "words": i.text.body.word_count() })).collect::<Vec<_>>(),
        "made": plan.report.made,
        "dropped": plan.report.dropped,
        "word_check": plan.report.check.as_ref().map(ToString::to_string),
        "notes_check": plan.report.notes_check.as_ref().map(ToString::to_string),
    })
}

/// Place a plan's items at the end of `into` (or the Manuscript), as one undo step.
pub fn place(s: &mut Session, plan: mapping::Plan, into: Option<ItemId>, label: &str) -> Result<Vec<ItemId>, String> {
    // Ids first, so the undo step knows every text it adds.
    let ids: Vec<ItemId> = plan.items.iter().map(|_| ItemId::new()).collect();
    let mut made: Vec<ItemId> = Vec::new();
    s.change(label, &ids, &[], |p| {
        let area = into.and_then(|i| p.binder.area_of(i)).unwrap_or(Area::Manuscript);
        // The parent at each depth.
        let mut parents: Vec<Option<ItemId>> = vec![into];
        for (it, id) in plan.items.into_iter().zip(ids.iter().copied()) {
            let parent = parents.get(it.depth).copied().flatten();
            let place = parent.map_or(Place::end_of(area), |pid| Place::inside(area, pid));
            let item = Item::with_id(id, it.kind, it.title).role(if area == Area::Manuscript { it.role } else { Role::None });
            p.binder.insert(place, item).map_err(|e| e.to_string())?;
            if !it.text.is_empty() {
                p.texts.insert(id, it.text);
            }
            parents.truncate(it.depth + 1);
            parents.push(Some(id));
            made.push(id);
        }
        Ok(())
    })?;
    Ok(made)
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
        "import.preview" => (|| {
            let path = str_param(v, "path")?.to_owned();
            let rules = rules_of(s, v)?;
            let text = TextRules { join_lines: v.get("join_lines").and_then(Value::as_bool).unwrap_or(true) };
            let plan = import::plan_file(Path::new(&path), &rules, &text)?;
            Ok(plan_json(&plan))
        })(),
        "import.document" => (|| {
            let path = str_param(v, "path")?.to_owned();
            let rules = rules_of(s, v)?;
            let text = TextRules { join_lines: v.get("join_lines").and_then(Value::as_bool).unwrap_or(true) };
            let into = crate::commands::opt_id(v, "into")?;
            let mut plan = import::plan_file(Path::new(&path), &rules, &text)?;
            // A single text with no title of its own takes the file's name.
            if let [only] = plan.items.as_mut_slice()
                && !only.title_from_source
                && let Some(stem) = Path::new(&path).file_stem()
            {
                only.title = stem.to_string_lossy().into_owned();
            }
            if let Some(c) = &plan.report.check
                && !c.passed()
            {
                return Err(format!("the import was stopped: its words do not match the source ({c})"));
            }
            let report = plan_json(&plan);
            let made = place(s, plan, into, "Import Document")?;
            if v.get("rules").is_some() {
                let r = serde_json::to_value(&rules).map_err(|e| e.to_string())?;
                s.project.settings.extra.insert("import".into(), r);
                s.touch();
            }
            Ok(json!({ "made": made.len(), "first": made.first().map(|i| i.as_text()), "report": report }))
        })(),
        _ => return None,
    })
}
