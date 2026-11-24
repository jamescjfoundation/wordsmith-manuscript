// SPDX-License-Identifier: MIT OR Apache-2.0
//! The inspector (INS-1 to INS-4, SPEC 8): an item's synopsis, notes, label, status, keywords,
//! linked bible entries, target and point of view, and the project's label, status and keyword
//! lists. Every change is a command and one undo step.

use jcj_shell::egui::{self, RichText, Ui};
use serde_json::{Value, json};
use wordsmith_manuscript_model::{Area, ItemId, Kind, Named, POV_FIELD, Project};

use crate::commands::{Spec, id_param, opt_str, str_param};
use crate::session::{Outcome, Session};

const fn spec(id: &'static str, label: &'static str, params: &'static str, about: &'static str) -> Spec {
    Spec { id, label, key: "", params, about, mutates: true }
}

/// The inspector's commands.
pub fn specs() -> Vec<Spec> {
    vec![
        spec("item.setSynopsis", "Set Synopsis", r#"{"id": id, "synopsis": string}"#, "An item's short summary."),
        spec("item.setNotes", "Set Notes", r#"{"id": id, "notes": string}"#, "An item's notes (Markdown dialect)."),
        spec("item.setLabel", "Set Label", r#"{"id": id, "label": string|null}"#, "An item's label, from the project's list (null clears it)."),
        spec("item.setStatus", "Set Status", r#"{"id": id, "status": string|null}"#, "An item's status, from the project's list (null clears it)."),
        spec(
            "item.addKeyword",
            "Add Keyword",
            r#"{"id": id, "keyword": string}"#,
            "Add a keyword to an item (added to the project's list when new).",
        ),
        spec("item.removeKeyword", "Remove Keyword", r#"{"id": id, "keyword": string}"#, "Remove a keyword from an item."),
        spec("item.linkEntry", "Link Bible Entry", r#"{"id": id, "entry": id}"#, "Link a bible entry to an item."),
        spec("item.unlinkEntry", "Unlink Bible Entry", r#"{"id": id, "entry": id}"#, "Unlink a bible entry from an item."),
        spec("item.setTarget", "Set Target", r#"{"id": id, "words": n|null}"#, "An item's word target."),
        spec(
            "item.setField",
            "Set Field",
            r#"{"id": id, "field": string, "value": string}"#,
            "A custom field's value (point of view at 0.1); empty clears it.",
        ),
        spec(
            "item.get",
            "Get Item",
            r#"{"id": id}"#,
            "An item's title, role, label, status, synopsis, notes, keywords, entries, target, fields and words.",
        ),
        spec(
            "project.setLabels",
            "Set Labels",
            r##"{"labels": [{"name": string, "color"?: "#RRGGBB"}], "clear"?: bool}"##,
            "The project's labels; removing one in use needs clear: true.",
        ),
        spec(
            "project.setStatuses",
            "Set Statuses",
            r#"{"statuses": [string], "clear"?: bool}"#,
            "The project's statuses in order; removing one in use needs clear: true.",
        ),
        spec(
            "project.setKeywords",
            "Set Keywords",
            r##"{"keywords": [{"name": string, "color"?: "#RRGGBB"}], "clear"?: bool}"##,
            "The project's keywords; removing one in use needs clear: true.",
        ),
        spec(
            "project.setBook",
            "Set Book Details",
            r#"{"title"?: string, "subtitle"?: string, "author"?: string, "pen_name"?: string, ...}"#,
            "The book's title, author and other details.",
        ),
    ]
}

fn has_text(p: &Project, id: ItemId) -> Result<(), String> {
    match p.binder.get(id) {
        None => Err(format!("no binder item {id}")),
        Some(i) if !i.kind.has_text() => Err("this item has no text of its own to describe".into()),
        Some(_) => Ok(()),
    }
}

fn named_list(v: &Value, key: &str) -> Result<Vec<Named>, String> {
    let arr = v.get(key).and_then(Value::as_array).ok_or_else(|| format!("`{key}` (a list) is required"))?;
    let mut out: Vec<Named> = Vec::new();
    for x in arr {
        let name = x.get("name").and_then(Value::as_str).or_else(|| x.as_str()).map(str::trim).unwrap_or("");
        if name.is_empty() {
            return Err("every name needs some text".into());
        }
        if out.iter().any(|n| n.name == name) {
            return Err(format!("`{name}` is in the list twice"));
        }
        let color = x.get("color").and_then(Value::as_str).unwrap_or("").to_owned();
        if !color.is_empty() && !(color.len() == 7 && color.starts_with('#') && color[1..].chars().all(|c| c.is_ascii_hexdigit())) {
            return Err(format!("`{color}` is not a color (#RRGGBB)"));
        }
        out.push(Named { name: name.to_owned(), color });
    }
    Ok(out)
}

/// How many items use each of `names` as a label (`label`), status, or keyword.
fn in_use(p: &Project, what: &str, name: &str) -> usize {
    p.binder
        .rows()
        .iter()
        .filter(|r| match what {
            "label" => r.item.label.as_deref() == Some(name),
            "status" => r.item.status.as_deref() == Some(name),
            _ => p.texts.get(&r.item.id).is_some_and(|t| t.meta.keywords.iter().any(|k| k == name)),
        })
        .count()
}

fn set_list(s: &mut Session, v: &Value, what: &'static str) -> Outcome {
    let clear = v.get("clear").and_then(Value::as_bool).unwrap_or(false);
    let new: Vec<Named> = match what {
        "status" => v
            .get("statuses")
            .and_then(Value::as_array)
            .ok_or("`statuses` (a list) is required")?
            .iter()
            .map(|x| x.as_str().map(|n| Named::new(n.trim(), "")).ok_or_else(|| "statuses are names".to_owned()))
            .collect::<Result<_, _>>()?,
        "label" => named_list(v, "labels")?,
        _ => named_list(v, "keywords")?,
    };
    if new.iter().any(|n| n.name.is_empty()) {
        return Err("every name needs some text".into());
    }
    let old: Vec<String> = match what {
        "label" => s.project.settings.labels.iter().map(|n| n.name.clone()).collect(),
        "status" => s.project.settings.statuses.clone(),
        _ => s.project.settings.keywords.iter().map(|n| n.name.clone()).collect(),
    };
    let removed: Vec<String> = old.into_iter().filter(|o| !new.iter().any(|n| &n.name == o)).collect();
    let used: usize = removed.iter().map(|r| in_use(&s.project, what, r)).sum();
    if used > 0 && !clear {
        return Err(format!(
            "{used} item{} use{} a {what} being removed: pass \"clear\": true to clear it from them",
            if used == 1 { "" } else { "s" },
            if used == 1 { "s" } else { "" }
        ));
    }
    let texts: Vec<ItemId> = if what == "keyword" { s.project.texts.keys().copied().collect() } else { Vec::new() };
    s.change(&format!("Set {what}s"), &texts, &[], |p| {
        match what {
            "label" => p.settings.labels = new,
            "status" => p.settings.statuses = new.into_iter().map(|n| n.name).collect(),
            _ => p.settings.keywords = new,
        }
        let ids: Vec<ItemId> = p.binder.rows().iter().map(|r| r.item.id).collect();
        for id in ids {
            if let Some(item) = p.binder.get_mut(id) {
                if what == "label" && item.label.as_ref().is_some_and(|l| removed.contains(l)) {
                    item.label = None;
                }
                if what == "status" && item.status.as_ref().is_some_and(|l| removed.contains(l)) {
                    item.status = None;
                }
            }
            if what == "keyword"
                && let Some(t) = p.texts.get_mut(&id)
            {
                t.meta.keywords.retain(|k| !removed.contains(k));
            }
        }
        Ok(())
    })?;
    Ok(json!({ "cleared": used }))
}

fn item_json(p: &Project, id: ItemId) -> Result<Value, String> {
    let item = p.binder.get(id).ok_or_else(|| format!("no binder item {id}"))?;
    let meta = p.text(id).map(|t| t.meta.clone()).unwrap_or_default();
    Ok(json!({
        "id": id.as_text(),
        "title": item.title,
        "role": item.role.key(),
        "include": item.include,
        "label": item.label,
        "status": item.status,
        "synopsis": meta.synopsis,
        "notes": meta.notes,
        "keywords": meta.keywords,
        "entries": meta.entries.iter().map(|e| e.as_text()).collect::<Vec<_>>(),
        "target": meta.target,
        "fields": meta.fields,
        "words": p.words_under(id),
    }))
}

/// Run an inspector command, or `None` if `id` is not one.
pub fn run(s: &mut Session, id: &str, v: &Value) -> Option<Outcome> {
    let r = match id {
        "item.get" => id_param(v, "id").and_then(|i| item_json(&s.project, i)),
        "item.setSynopsis" | "item.setNotes" => (|| {
            let i = id_param(v, "id")?;
            has_text(&s.project, i)?;
            let key = if id == "item.setSynopsis" { "synopsis" } else { "notes" };
            let value = str_param(v, key)?.to_owned();
            s.change(if key == "synopsis" { "Synopsis" } else { "Notes" }, &[i], &[], |p| {
                let m = &mut p.text_mut(i).meta;
                if key == "synopsis" {
                    m.synopsis = value;
                } else {
                    m.notes = value;
                }
                Ok(())
            })?;
            Ok(json!({}))
        })(),
        "item.setLabel" | "item.setStatus" => (|| {
            let i = id_param(v, "id")?;
            s.project.binder.get(i).ok_or_else(|| format!("no binder item {i}"))?;
            let key = if id == "item.setLabel" { "label" } else { "status" };
            let value = opt_str(v, key).map(str::to_owned).filter(|x| !x.is_empty());
            if let Some(x) = &value {
                let known =
                    if key == "label" { s.project.settings.labels.iter().any(|l| &l.name == x) } else { s.project.settings.statuses.contains(x) };
                if !known {
                    return Err(format!("`{x}` is not one of the project's {key}s"));
                }
            }
            s.change(if key == "label" { "Label" } else { "Status" }, &[], &[], |p| {
                if let Some(item) = p.binder.get_mut(i) {
                    if key == "label" {
                        item.label = value;
                    } else {
                        item.status = value;
                    }
                }
                Ok(())
            })?;
            Ok(json!({}))
        })(),
        "item.addKeyword" | "item.removeKeyword" => (|| {
            let i = id_param(v, "id")?;
            has_text(&s.project, i)?;
            let k = str_param(v, "keyword")?.trim().to_owned();
            if k.is_empty() {
                return Err("a keyword needs some text".into());
            }
            let add = id == "item.addKeyword";
            s.change(if add { "Add Keyword" } else { "Remove Keyword" }, &[i], &[], |p| {
                if add {
                    if !p.settings.keywords.iter().any(|n| n.name == k) {
                        p.settings.keywords.push(Named::new(&k, ""));
                    }
                    let kws = &mut p.text_mut(i).meta.keywords;
                    if !kws.contains(&k) {
                        kws.push(k);
                    }
                } else {
                    p.text_mut(i).meta.keywords.retain(|x| *x != k);
                }
                Ok(())
            })?;
            Ok(json!({}))
        })(),
        "item.linkEntry" | "item.unlinkEntry" => (|| {
            let i = id_param(v, "id")?;
            has_text(&s.project, i)?;
            let e = id_param(v, "entry")?;
            if s.project.binder.get(e).is_none_or(|x| x.kind != Kind::Entry) {
                return Err(format!("{e} is not a bible entry"));
            }
            let link = id == "item.linkEntry";
            s.change(if link { "Link Entry" } else { "Unlink Entry" }, &[i], &[], |p| {
                let es = &mut p.text_mut(i).meta.entries;
                if link {
                    if !es.contains(&e) {
                        es.push(e);
                    }
                } else {
                    es.retain(|x| *x != e);
                }
                Ok(())
            })?;
            Ok(json!({}))
        })(),
        "item.setTarget" => (|| {
            let i = id_param(v, "id")?;
            has_text(&s.project, i)?;
            let words = match v.get("words") {
                None | Some(Value::Null) => None,
                Some(w) => Some(w.as_u64().filter(|w| *w <= 10_000_000).ok_or("`words` is a number of words")?),
            };
            s.change("Target", &[i], &[], |p| {
                p.text_mut(i).meta.target = words.filter(|w| *w > 0);
                Ok(())
            })?;
            Ok(json!({}))
        })(),
        "item.setField" => (|| {
            let i = id_param(v, "id")?;
            has_text(&s.project, i)?;
            let field = str_param(v, "field")?.to_owned();
            if !s.project.settings.fields.iter().any(|f| f.name == field) {
                return Err(format!("`{field}` is not one of the project's fields"));
            }
            let value = str_param(v, "value")?.trim().to_owned();
            s.change("Field", &[i], &[], |p| {
                let f = &mut p.text_mut(i).meta.fields;
                if value.is_empty() {
                    f.remove(&field);
                } else {
                    f.insert(field, value);
                }
                Ok(())
            })?;
            Ok(json!({}))
        })(),
        "project.setLabels" => set_list(s, v, "label"),
        "project.setStatuses" => set_list(s, v, "status"),
        "project.setKeywords" => set_list(s, v, "keyword"),
        "project.setBook" => (|| {
            let mut book = s.project.settings.book.clone();
            let mut map = serde_json::to_value(&book).map_err(|e| e.to_string())?;
            let Some(obj) = v.as_object() else { return Err("details are an object".into()) };
            for (k, val) in obj {
                const FIELDS: [&str; 17] = [
                    "title",
                    "subtitle",
                    "author",
                    "pen_name",
                    "surname",
                    "short_title",
                    "contact",
                    "language",
                    "series",
                    "series_number",
                    "isbn",
                    "publisher",
                    "copyright_year",
                    "rights",
                    "description",
                    "subjects",
                    "keywords",
                ];
                if !FIELDS.contains(&k.as_str()) {
                    return Err(format!("`{k}` is not a book detail"));
                }
                map[k] = val.clone();
            }
            book = serde_json::from_value(map).map_err(|e| format!("book details: {e}"))?;
            s.change("Book Details", &[], &[], |p| {
                p.settings.book = book;
                Ok(())
            })?;
            Ok(json!({}))
        })(),
        _ => return None,
    };
    Some(r)
}

/// The item's comments in text order, each editable and removable (ED-7).
fn comments_section(ui: &mut Ui, s: &mut Session, view: &mut InspectorView, id: ItemId, errors: &mut Vec<String>) {
    let Ok(list) = s.run("comment.list", &json!({ "id": id.as_text() })) else { return };
    let rows = list["comments"].as_array().cloned().unwrap_or_default();
    if rows.is_empty() {
        return;
    }
    ui.add_space(6.0);
    ui.label(format!("Comments ({})", rows.len()));
    for row in rows {
        let key = row["comment"].as_str().unwrap_or("").to_owned();
        let words = row["words"].as_str().unwrap_or("");
        ui.group(|ui| {
            if row["anchored"].as_bool() == Some(true) {
                let short: String = words.chars().take(60).collect();
                ui.label(RichText::new(format!("“{short}{}”", if words.chars().count() > 60 { "…" } else { "" })).italics().weak());
            } else {
                ui.label(RichText::new("Its words are no longer in the text").weak());
            }
            let buf = view.comments.entry(key.clone()).or_default();
            let r = ui.add(egui::TextEdit::multiline(buf).desired_rows(2).desired_width(f32::INFINITY).hint_text("Comment"));
            r.widget_info(|| egui::WidgetInfo::labeled(egui::Role::TextInput, true, format!("Comment {key}")));
            if view.focus_comment.as_deref() == Some(key.as_str()) {
                r.request_focus();
                view.focus_comment = None;
            }
            if r.lost_focus() && row["text"].as_str() != Some(buf.as_str()) {
                if let Err(e) = s.run("comment.set", &json!({ "id": id.as_text(), "comment": key, "text": buf.clone() })) {
                    errors.push(e);
                }
            }
            ui.horizontal(|ui| {
                let by = row["by"].as_str().unwrap_or("");
                let date = row["date"].as_str().unwrap_or("");
                let sign = [by, date].iter().filter(|x| !x.is_empty()).copied().collect::<Vec<_>>().join(" · ");
                if !sign.is_empty() {
                    ui.label(RichText::new(sign).small().weak());
                }
                if ui.small_button("Remove").on_hover_text("Remove the comment; the words stay").clicked()
                    && let Err(e) = s.run("comment.remove", &json!({ "id": id.as_text(), "comment": key }))
                {
                    errors.push(e);
                }
            });
        });
    }
}

/// The inspector panel's state: text being typed, committed when the field loses focus.
#[derive(Default)]
pub struct InspectorView {
    /// A comment just added, whose text field takes the keyboard next.
    pub focus_comment: Option<String>,
    comments: std::collections::BTreeMap<String, String>,
    item: Option<ItemId>,
    rev: u64,
    synopsis: String,
    notes: String,
    keyword: String,
    target: String,
    pov: String,
}

impl InspectorView {
    fn load(&mut self, p: &Project, id: ItemId, rev: u64) {
        let meta = p.text(id).map(|t| t.meta.clone()).unwrap_or_default();
        self.item = Some(id);
        self.rev = rev;
        self.synopsis = meta.synopsis;
        self.notes = meta.notes;
        self.target = meta.target.map(|t| t.to_string()).unwrap_or_default();
        self.pov = meta.fields.get(POV_FIELD).cloned().unwrap_or_default();
        self.keyword.clear();
        self.comments = meta.comments.into_iter().map(|(k, c)| (k, c.text)).collect();
    }
}

/// Draw the inspector for `selected`. Returns errors for the status bar.
pub fn panel(ui: &mut Ui, s: &mut Session, view: &mut InspectorView, selected: Option<ItemId>) -> Vec<String> {
    let mut errors = Vec::new();
    let Some(id) = selected.filter(|i| s.project.binder.contains(*i)) else {
        ui.label("Select an item in the binder.");
        return errors;
    };
    let rev = s.revision();
    let typing = ui.ctx().memory(|m| m.focused().is_some());
    if view.item != Some(id) || (view.rev != rev && !typing) {
        view.load(&s.project, id, rev);
    }
    let Some(item) = s.project.binder.get(id).cloned() else { return errors };
    let errs = std::cell::RefCell::new(Vec::new());
    let run = |s: &mut Session, cmd: &str, v: Value| {
        if let Err(e) = s.run(cmd, &v) {
            errs.borrow_mut().push(e);
        }
    };
    egui::ScrollArea::vertical().id_salt("manuscript-inspector").auto_shrink([false, false]).show(ui, |ui| {
        ui.label(RichText::new(&item.title).strong());
        ui.label(RichText::new(format!("{} · {} words", item.role.title(), crate::binder::format_words(s.project.words_under(id)))).weak());
        if !item.kind.has_text() {
            return;
        }
        ui.add_space(6.0);
        let l = ui.label("Synopsis");
        let r = ui.add(egui::TextEdit::multiline(&mut view.synopsis).desired_rows(3).desired_width(f32::INFINITY)).labelled_by(l.id);
        if r.lost_focus() && s.project.text(id).map(|t| t.meta.synopsis.as_str()).unwrap_or("") != view.synopsis {
            run(s, "item.setSynopsis", json!({ "id": id.as_text(), "synopsis": view.synopsis }));
        }
        ui.horizontal(|ui| {
            let l = ui.label("Label");
            let current = item.label.clone().unwrap_or_else(|| "None".into());
            let c = egui::ComboBox::from_id_salt("inspector-label").selected_text(current).show_ui(ui, |ui| {
                if ui.selectable_label(item.label.is_none(), "None").clicked() {
                    run(s, "item.setLabel", json!({ "id": id.as_text(), "label": null }));
                }
                for l in s.project.settings.labels.clone() {
                    if ui.selectable_label(item.label.as_ref() == Some(&l.name), &l.name).clicked() {
                        run(s, "item.setLabel", json!({ "id": id.as_text(), "label": l.name }));
                    }
                }
            });
            c.response.labelled_by(l.id);
        });
        ui.horizontal(|ui| {
            let l = ui.label("Status");
            let current = item.status.clone().unwrap_or_else(|| "None".into());
            let c = egui::ComboBox::from_id_salt("inspector-status").selected_text(current).show_ui(ui, |ui| {
                if ui.selectable_label(item.status.is_none(), "None").clicked() {
                    run(s, "item.setStatus", json!({ "id": id.as_text(), "status": null }));
                }
                for st in s.project.settings.statuses.clone() {
                    if ui.selectable_label(item.status.as_ref() == Some(&st), &st).clicked() {
                        run(s, "item.setStatus", json!({ "id": id.as_text(), "status": st }));
                    }
                }
            });
            c.response.labelled_by(l.id);
        });
        ui.horizontal(|ui| {
            let l = ui.label(POV_FIELD);
            let r = ui.text_edit_singleline(&mut view.pov).labelled_by(l.id);
            if r.lost_focus() && s.project.text(id).and_then(|t| t.meta.fields.get(POV_FIELD)).map(String::as_str).unwrap_or("") != view.pov.trim() {
                run(s, "item.setField", json!({ "id": id.as_text(), "field": POV_FIELD, "value": view.pov }));
            }
        });
        ui.horizontal(|ui| {
            let l = ui.label("Target");
            let r = ui.add(egui::TextEdit::singleline(&mut view.target).desired_width(80.0).hint_text("words")).labelled_by(l.id);
            if r.lost_focus() {
                let words: Option<u64> = view.target.trim().replace(',', "").parse().ok();
                if words != s.project.text(id).and_then(|t| t.meta.target) {
                    run(s, "item.setTarget", json!({ "id": id.as_text(), "words": words }));
                }
            }
        });
        ui.add_space(6.0);
        ui.label("Keywords");
        let kws = s.project.text(id).map(|t| t.meta.keywords.clone()).unwrap_or_default();
        ui.horizontal_wrapped(|ui| {
            for k in &kws {
                if ui.small_button(format!("{k} ×")).on_hover_text("Remove").clicked() {
                    run(s, "item.removeKeyword", json!({ "id": id.as_text(), "keyword": k }));
                }
            }
            let r = ui.add(egui::TextEdit::singleline(&mut view.keyword).desired_width(100.0).hint_text("Add keyword"));
            r.widget_info(|| egui::WidgetInfo::labeled(egui::Role::TextInput, true, "Add keyword"));
            if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) && !view.keyword.trim().is_empty() {
                run(s, "item.addKeyword", json!({ "id": id.as_text(), "keyword": view.keyword }));
                view.keyword.clear();
            }
        });
        ui.add_space(6.0);
        ui.label("Bible entries");
        let linked = s.project.text(id).map(|t| t.meta.entries.clone()).unwrap_or_default();
        ui.horizontal_wrapped(|ui| {
            for e in &linked {
                let name = s.project.binder.get(*e).map_or_else(|| "(deleted)".to_owned(), |i| i.title.clone());
                if ui.small_button(format!("{name} ×")).on_hover_text("Unlink").clicked() {
                    run(s, "item.unlinkEntry", json!({ "id": id.as_text(), "entry": e.as_text() }));
                }
            }
            let entries: Vec<(ItemId, String)> = s
                .project
                .binder
                .rows_of(Area::Bible)
                .iter()
                .filter(|r| r.item.kind == Kind::Entry && !linked.contains(&r.item.id))
                .map(|r| (r.item.id, r.item.title.clone()))
                .collect();
            if !entries.is_empty() {
                let c = egui::ComboBox::from_id_salt("inspector-link-entry").selected_text("Link…").show_ui(ui, |ui| {
                    for (eid, title) in entries {
                        if ui.selectable_label(false, title).clicked() {
                            run(s, "item.linkEntry", json!({ "id": id.as_text(), "entry": eid.as_text() }));
                        }
                    }
                });
                c.response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::ComboBox, true, "Link a bible entry"));
            }
        });
        ui.add_space(6.0);
        let l = ui.label("Notes");
        let r = ui.add(egui::TextEdit::multiline(&mut view.notes).desired_rows(6).desired_width(f32::INFINITY)).labelled_by(l.id);
        if r.lost_focus() && s.project.text(id).map(|t| t.meta.notes.as_str()).unwrap_or("") != view.notes {
            run(s, "item.setNotes", json!({ "id": id.as_text(), "notes": view.notes }));
        }
        comments_section(ui, s, view, id, &mut errs.borrow_mut());
        crate::snapshots::panel_section(ui, s, id, &mut errs.borrow_mut());
    });
    errors.extend(errs.into_inner());
    if s.revision() != rev {
        view.load(&s.project, id, s.revision());
    }
    errors
}
