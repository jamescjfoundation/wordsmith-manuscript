// SPDX-License-Identifier: MIT OR Apache-2.0
//! Import commands: what the io layer reads, placed in the project as one undo step, with its
//! report.

use std::path::Path;

use serde_json::{Value, json};
use wordsmith_manuscript_io::import::{self, TextRules};
use wordsmith_manuscript_io::{docset, mapping, notes};
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
            id: "import.set",
            label: "Import Document Set",
            key: "",
            params: r#"{"paths"?: [string], "folder"?: string, "names"?: [{"marker": string, "dest": "notes"|"research"|"draft", "draft"?: n}], "rules"?: {...}}"#,
            about: "Import a set of documents placed by the roles in their names: plans into Notes, rough work into Research, the latest draft's files joined into the manuscript, earlier drafts into Research (one undo step).",
            mutates: true,
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

/// Where a plan goes in a set import: the area, and a folder to hold it (when titled).
pub struct Target {
    /// The area.
    pub area: Area,
    /// A folder made for the plan's items, with this title (none: the items go at the top).
    pub folder: Option<String>,
    /// The plan.
    pub plan: mapping::Plan,
}

/// Place several plans as one undo step. Returns the ids made.
pub fn place_many(s: &mut Session, targets: Vec<Target>, label: &str) -> Result<Vec<ItemId>, String> {
    let ids: Vec<ItemId> =
        targets.iter().flat_map(|t| t.plan.items.iter().map(|_| ItemId::new()).chain(t.folder.iter().map(|_| ItemId::new()))).collect();
    let mut pool = ids.clone().into_iter();
    let mut made = Vec::new();
    s.change(label, &ids, &[], |p| {
        for t in targets {
            let mut parents: Vec<Option<ItemId>> = vec![None];
            if let Some(title) = t.folder {
                let fid = pool.next().ok_or("out of ids")?;
                p.binder.insert(Place::end_of(t.area), Item::with_id(fid, Kind::Folder, title)).map_err(|e| e.to_string())?;
                parents = vec![Some(fid)];
                made.push(fid);
            }
            for it in t.plan.items {
                let id = pool.next().ok_or("out of ids")?;
                let parent = parents.get(it.depth).copied().flatten();
                let place = parent.map_or(Place::end_of(t.area), |pid| Place::inside(t.area, pid));
                let role = if t.area == Area::Manuscript { it.role } else { Role::None };
                p.binder.insert(place, Item::with_id(id, it.kind, it.title).role(role)).map_err(|e| e.to_string())?;
                if !it.text.is_empty() {
                    p.texts.insert(id, it.text);
                }
                parents.truncate(it.depth + 1);
                parents.push(Some(id));
                made.push(id);
            }
        }
        Ok(())
    })?;
    Ok(made)
}

/// Each Manuscript item's title path ("Part One/Chapter 2/Scene 1") with its id.
fn title_paths(p: &wordsmith_manuscript_model::Project) -> std::collections::HashMap<String, ItemId> {
    let mut out = std::collections::HashMap::new();
    let mut stack: Vec<String> = Vec::new();
    for r in p.binder.rows_of(Area::Manuscript) {
        stack.truncate(r.depth);
        stack.push(r.item.title.clone());
        out.entry(stack.join("/")).or_insert(r.item.id);
    }
    out
}

/// A stored draft made from a plan; items whose title path the working draft has take its ids.
fn draft_body(plan: &mapping::Plan, paths: &std::collections::HashMap<String, ItemId>) -> wordsmith_manuscript_model::drafts::DraftBody {
    let mut body = wordsmith_manuscript_model::drafts::DraftBody::default();
    let mut parents: Vec<Option<ItemId>> = vec![None];
    let mut stack: Vec<String> = Vec::new();
    let mut used = std::collections::HashSet::new();
    for it in &plan.items {
        stack.truncate(it.depth);
        stack.push(it.title.clone());
        let id = paths.get(&stack.join("/")).copied().filter(|i| used.insert(*i)).unwrap_or_else(ItemId::new);
        let parent = parents.get(it.depth).copied().flatten();
        let place = parent.map_or(Place::end_of(Area::Manuscript), |pid| Place::inside(Area::Manuscript, pid));
        let _ = body.binder.insert(place, Item::with_id(id, it.kind, it.title.clone()).role(it.role));
        if !it.text.is_empty() {
            body.texts.insert(id, it.text.clone());
        }
        parents.truncate(it.depth + 1);
        parents.push(Some(id));
    }
    body
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
        "import.set" => (|| {
            let paths: Vec<std::path::PathBuf> = match (v.get("paths").and_then(Value::as_array), crate::commands::opt_str(v, "folder")) {
                (Some(list), _) => list.iter().filter_map(Value::as_str).map(std::path::PathBuf::from).collect(),
                (None, Some(dir)) => docset::folder_files(Path::new(dir))?,
                (None, None) => return Err("give `paths` or a `folder`".into()),
            };
            if paths.is_empty() {
                return Err("the set holds no files that can be imported".into());
            }
            let rules = rules_of(s, v)?;
            let set_rules: docset::SetRules = match v.get("names") {
                Some(n) => docset::SetRules { names: serde_json::from_value(n.clone()).map_err(|e| format!("names: {e}"))? },
                None => s.project.settings.extra.get("document_set").and_then(|r| serde_json::from_value(r.clone()).ok()).unwrap_or_default(),
            };
            let set = docset::plan_set(&paths, &set_rules, &rules, &TextRules::default())?;
            let mut checks = Vec::new();
            let mut targets = Vec::new();
            if let Some(m) = set.manuscript {
                if let Some(c) = &m.report.check
                    && !c.passed()
                {
                    return Err(format!("the import was stopped: the manuscript's words do not match its files ({c})"));
                }
                checks.push(json!({ "draft": set.draft_name, "check": m.report.check.as_ref().map(ToString::to_string), "items": m.items.len() }));
                targets.push(Target { area: Area::Manuscript, folder: None, plan: m });
            }
            for (dest, name, plan) in set.others {
                if let Some(c) = &plan.report.check
                    && !c.passed()
                {
                    return Err(format!("the import was stopped: the words of {name} do not match ({c})"));
                }
                let area = if dest == docset::Dest::Notes { Area::Notes } else { Area::Research };
                checks.push(json!({ "file": name, "into": area.key(), "check": plan.report.check.as_ref().map(ToString::to_string) }));
                let one = plan.items.len() == 1;
                let mut plan = plan;
                if one && let Some(it) = plan.items.first_mut() {
                    it.title.clone_from(&name);
                }
                targets.push(Target { area, folder: (!one).then_some(name), plan });
            }
            let latest_name = set.draft_name.clone();
            let made = place_many(s, targets, "Import Document Set")?;
            // Earlier drafts become stored drafts; their chapters and scenes take the ids of the
            // latest draft's ones with the same titles in the same place, so drafts compare
            // scene by scene.
            if !set.earlier.is_empty() {
                for (name, plan) in &set.earlier {
                    if let Some(c) = &plan.report.check
                        && !c.passed()
                    {
                        return Err(format!("the import was stopped after the latest draft: the words of {name} do not match ({c})"));
                    }
                }
                let earlier = set.earlier;
                s.change_drafts("Import Earlier Drafts", |p| {
                    let paths = title_paths(p);
                    let mut stored = Vec::new();
                    for (name, plan) in earlier {
                        let body = draft_body(&plan, &paths);
                        stored.push((name, body));
                    }
                    if p.settings.drafts.is_empty() && p.settings.draft_name.is_empty() && !latest_name.is_empty() {
                        p.settings.draft_name.clone_from(&latest_name);
                    }
                    for (name, body) in stored {
                        let did = ItemId::new().as_text();
                        let created = wordsmith_manuscript_model::snapshot::stamp(std::time::SystemTime::now());
                        p.drafts.insert(did.clone(), body);
                        p.settings.drafts.push(wordsmith_manuscript_model::drafts::DraftInfo { id: did, name, created });
                    }
                    Ok(())
                })?;
                checks.push(json!({ "earlier_drafts": s.project.settings.drafts.len() }));
            }
            if v.get("names").is_some() {
                s.project.settings.extra.insert("document_set".into(), serde_json::to_value(&set_rules).map_err(|e| e.to_string())?);
                s.touch();
            }
            Ok(json!({
                "files": set.files.iter().map(|f| json!({ "name": f.name, "dest": format!("{:?}", f.dest).to_lowercase(), "draft": f.draft })).collect::<Vec<_>>(),
                "made": made.len(),
                "checks": checks,
            }))
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
