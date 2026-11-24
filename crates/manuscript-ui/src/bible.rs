// SPDX-License-Identifier: MIT OR Apache-2.0
//! The story bible area (BIB-1 to BIB-3): entries of each type with their template's fields, a
//! body, aliases, and the documents that link to or mention them.

use jcj_shell::egui::{self, RichText, Ui};
use serde_json::{Value, json};
use wordsmith_manuscript_model::entries::{self, FieldType};
use wordsmith_manuscript_model::{Area, Entry, Item, ItemId, Kind, Place, Project, bible};

use crate::commands::{Spec, id_param, opt_id, opt_str, str_param};
use crate::session::{Outcome, Session};

const fn spec(id: &'static str, label: &'static str, params: &'static str, about: &'static str, mutates: bool) -> Spec {
    Spec { id, label, key: "", params, about, mutates }
}

/// The bible's commands.
pub fn specs() -> Vec<Spec> {
    vec![
        spec(
            "bible.new",
            "New Bible Entry",
            r#"{"kind": string, "name": string, "inside"?: id}"#,
            "A new entry of a type, in that type's folder (made if missing) or inside a folder.",
            true,
        ),
        spec("bible.list", "List Bible Entries", r#"{"kind"?: string}"#, "Every entry: id, kind, name, aliases.", false),
        spec("bible.templates", "Bible Templates", "{}", "The entry types and their fields.", false),
        spec("entry.get", "Get Entry", r#"{"id": id}"#, "An entry's kind, name, aliases, fields (in template order) and body.", false),
        spec(
            "entry.set",
            "Set Entry",
            r#"{"id": id, "name"?: string, "aliases"?: [string], "kind"?: string}"#,
            "Change an entry's name, aliases or type.",
            true,
        ),
        spec("entry.setField", "Set Entry Field", r#"{"id": id, "field": string, "value": string}"#, "A field's value (empty clears it).", true),
        spec("entry.setBody", "Set Entry Body", r#"{"id": id, "text": string}"#, "An entry's body in the Markdown dialect.", true),
        spec(
            "bible.backlinks",
            "Backlinks",
            r#"{"id": id}"#,
            "The documents that link to an entry, and those that mention its name or an alias.",
            false,
        ),
    ]
}

fn entry_of<'a>(p: &'a Project, id: ItemId) -> Result<&'a Entry, String> {
    p.entries.get(&id).ok_or_else(|| format!("{id} is not a bible entry"))
}

/// The folder of a type in the Story Bible (by the template's plural name), if there is one.
pub fn folder_of(p: &Project, kind: &str) -> Option<ItemId> {
    let plural = bible::template(p, kind)?.plural;
    p.binder.area(Area::Bible).iter().find(|i| i.kind == Kind::Folder && i.title == plural).map(|i| i.id)
}

/// Every text and entry body in the project (not the Trash), for backlinks.
pub fn documents(p: &Project) -> Vec<(ItemId, &wordsmith_manuscript_model::Text)> {
    let mut out = Vec::new();
    for area in [Area::Manuscript, Area::Bible, Area::Research, Area::Notes] {
        for r in p.binder.rows_of(area) {
            if let Some(t) = p.texts.get(&r.item.id) {
                out.push((r.item.id, &t.body));
            }
            if let Some(e) = p.entries.get(&r.item.id) {
                out.push((r.item.id, &e.body));
            }
        }
    }
    out
}

/// Run a bible command, or `None` if `id` is not one.
pub fn run(s: &mut Session, id: &str, v: &Value) -> Option<Outcome> {
    Some(match id {
        "bible.new" => (|| {
            let kind = str_param(v, "kind")?.to_owned();
            let name = str_param(v, "name")?.trim().to_owned();
            if name.is_empty() {
                return Err("an entry needs a name".into());
            }
            let template = bible::template(&s.project, &kind).ok_or_else(|| format!("`{kind}` is not an entry type of this project"))?;
            let inside = opt_id(v, "inside")?;
            if let Some(f) = inside
                && s.project.binder.area_of(f) != Some(Area::Bible)
            {
                return Err("entries live in the Story Bible".into());
            }
            let item = Item::new(Kind::Entry, name.clone());
            let eid = item.id;
            let existing = folder_of(&s.project, &kind);
            s.change("New Bible Entry", &[], &[eid], |p| {
                let parent = match inside.or(existing) {
                    Some(f) => f,
                    None => {
                        let folder = Item::new(Kind::Folder, template.plural.clone());
                        let fid = folder.id;
                        p.binder.insert(Place::end_of(Area::Bible), folder).map_err(|e| e.to_string())?;
                        fid
                    }
                };
                p.binder.insert(Place::inside(Area::Bible, parent), item).map_err(|e| e.to_string())?;
                p.entries.insert(eid, Entry::new(&kind, &name));
                Ok(())
            })?;
            Ok(json!({ "id": eid.as_text() }))
        })(),
        "bible.list" => (|| {
            let kind = opt_str(v, "kind");
            Ok(Value::Array(
                s.project
                    .binder
                    .rows_of(Area::Bible)
                    .iter()
                    .filter_map(|r| s.project.entries.get(&r.item.id).map(|e| (r.item.id, e)))
                    .filter(|(_, e)| kind.is_none_or(|k| e.kind == k))
                    .map(|(i, e)| json!({ "id": i.as_text(), "kind": e.kind, "name": e.name, "aliases": e.aliases }))
                    .collect(),
            ))
        })(),
        "bible.templates" => serde_json::to_value(bible::templates(&s.project.settings)).map_err(|e| e.to_string()),
        "entry.get" => (|| {
            let i = id_param(v, "id")?;
            let e = entry_of(&s.project, i)?;
            let fields: Vec<Value> = match bible::template(&s.project, &e.kind) {
                Some(t) => t.ordered(e).into_iter().map(|(n, val)| json!({ "name": n, "value": val })).collect(),
                None => e.fields.iter().map(|(n, val)| json!({ "name": n, "value": val })).collect(),
            };
            Ok(
                json!({ "kind": e.kind, "name": e.name, "aliases": e.aliases, "fields": fields, "body": wordsmith_manuscript_format::dialect::write(&e.body) }),
            )
        })(),
        "entry.set" => (|| {
            let i = id_param(v, "id")?;
            let mut e = entry_of(&s.project, i)?.clone();
            if let Some(n) = opt_str(v, "name") {
                if n.trim().is_empty() {
                    return Err("an entry needs a name".into());
                }
                e.name = n.trim().to_owned();
            }
            if let Some(a) = v.get("aliases").and_then(Value::as_array) {
                e.aliases = a.iter().filter_map(Value::as_str).map(str::trim).filter(|x| !x.is_empty()).map(str::to_owned).collect();
            }
            if let Some(k) = opt_str(v, "kind") {
                if bible::template(&s.project, k).is_none() {
                    return Err(format!("`{k}` is not an entry type of this project"));
                }
                e.kind = k.to_owned();
            }
            s.change("Entry", &[], &[i], |p| {
                if let Some(item) = p.binder.get_mut(i) {
                    item.title.clone_from(&e.name);
                }
                p.entries.insert(i, e);
                Ok(())
            })?;
            Ok(json!({}))
        })(),
        "entry.setField" => (|| {
            let i = id_param(v, "id")?;
            entry_of(&s.project, i)?;
            let field = str_param(v, "field")?.trim().to_owned();
            if field.is_empty() {
                return Err("a field needs a name".into());
            }
            let value = str_param(v, "value")?.to_owned();
            s.change("Entry Field", &[], &[i], |p| {
                if let Some(e) = p.entries.get_mut(&i) {
                    if value.trim().is_empty() {
                        e.fields.remove(&field);
                    } else {
                        e.fields.insert(field, value);
                    }
                }
                Ok(())
            })?;
            Ok(json!({}))
        })(),
        "entry.setBody" => (|| {
            let i = id_param(v, "id")?;
            entry_of(&s.project, i)?;
            let body = wordsmith_manuscript_format::dialect::parse(str_param(v, "text")?);
            s.change("Entry Body", &[], &[i], |p| {
                if let Some(e) = p.entries.get_mut(&i) {
                    e.body = body;
                }
                Ok(())
            })?;
            Ok(json!({}))
        })(),
        "bible.backlinks" => (|| {
            let i = id_param(v, "id")?;
            let e = entry_of(&s.project, i)?;
            let b = entries::backlinks(i, e, documents(&s.project));
            let title = |x: &ItemId| s.project.binder.get(*x).map(|it| it.title.clone()).unwrap_or_default();
            Ok(json!({
                "linked": b.linked.iter().map(|x| json!({ "id": x.as_text(), "title": title(x) })).collect::<Vec<_>>(),
                "mentioned": b.mentioned.iter().map(|x| json!({ "id": x.as_text(), "title": title(x) })).collect::<Vec<_>>(),
            }))
        })(),
        _ => return None,
    })
}

/// The entry view's state: field text being typed, committed when a field is left.
#[derive(Default)]
pub struct EntryView {
    item: Option<ItemId>,
    rev: u64,
    name: String,
    aliases: String,
    fields: Vec<(String, String, FieldType)>,
    new_field: String,
}

impl EntryView {
    fn load(&mut self, p: &Project, id: ItemId, rev: u64) {
        let Some(e) = p.entries.get(&id) else { return };
        self.item = Some(id);
        self.rev = rev;
        self.name = e.name.clone();
        self.aliases = e.aliases.join(", ");
        let t = bible::template(p, &e.kind);
        self.fields = match &t {
            Some(t) => t
                .ordered(e)
                .into_iter()
                .map(|(n, v)| {
                    let kind = t.fields.iter().find(|f| f.name == n).map_or(FieldType::Text, |f| f.kind);
                    (n, v.to_owned(), kind)
                })
                .collect(),
            None => e.fields.iter().map(|(n, v)| (n.clone(), v.clone(), FieldType::Text)).collect(),
        };
    }
}

/// What the entry view asks of the app.
pub enum Ask {
    /// Open a document.
    Open(ItemId),
}

/// Draw an entry's form and backlinks (its body is in the editor below).
pub fn form(ui: &mut Ui, s: &mut Session, v: &mut EntryView, id: ItemId) -> (Vec<Ask>, Vec<String>) {
    let mut asks = Vec::new();
    let mut errors = Vec::new();
    let rev = s.revision();
    let typing = ui.ctx().memory(|m| m.focused().is_some());
    if v.item != Some(id) || (v.rev != rev && !typing) {
        v.load(&s.project, id, rev);
    }
    let Some(e) = s.project.entries.get(&id).cloned() else { return (asks, errors) };
    let mut todo: Vec<(&str, Value)> = Vec::new();
    ui.horizontal(|ui| {
        let l = ui.label("Name");
        let r = ui.add(egui::TextEdit::singleline(&mut v.name).font(egui::TextStyle::Heading).desired_width(260.0)).labelled_by(l.id);
        if r.lost_focus() && v.name.trim() != e.name {
            todo.push(("entry.set", json!({ "id": id.as_text(), "name": v.name })));
        }
        let templates = bible::templates(&s.project.settings);
        let current = templates.iter().find(|t| t.kind == e.kind).map_or_else(|| e.kind.clone(), |t| t.title.clone());
        let c = egui::ComboBox::from_id_salt("entry-kind").selected_text(current).show_ui(ui, |ui| {
            for t in &templates {
                if ui.selectable_label(t.kind == e.kind, &t.title).clicked() {
                    todo.push(("entry.set", json!({ "id": id.as_text(), "kind": t.kind })));
                }
            }
        });
        c.response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::ComboBox, true, "Entry type"));
    });
    ui.horizontal(|ui| {
        let l = ui.label("Also called");
        let r = ui
            .add(egui::TextEdit::singleline(&mut v.aliases).hint_text("other names, separated by commas").desired_width(f32::INFINITY))
            .labelled_by(l.id);
        if r.lost_focus() {
            let aliases: Vec<&str> = v.aliases.split(',').map(str::trim).filter(|a| !a.is_empty()).collect();
            if aliases != e.aliases.iter().map(String::as_str).collect::<Vec<_>>() {
                todo.push(("entry.set", json!({ "id": id.as_text(), "aliases": aliases })));
            }
        }
    });
    egui::Grid::new("entry-fields").num_columns(2).striped(true).show(ui, |ui| {
        for (name, value, kind) in &mut v.fields {
            let l = ui.label(name.as_str());
            let r = match kind {
                FieldType::LongText | FieldType::List => {
                    ui.add(egui::TextEdit::multiline(value).desired_rows(2).desired_width(420.0)).labelled_by(l.id)
                }
                _ => ui.add(egui::TextEdit::singleline(value).desired_width(420.0)).labelled_by(l.id),
            };
            if r.lost_focus() && e.fields.get(name.as_str()).map_or("", String::as_str) != value.as_str() {
                todo.push(("entry.setField", json!({ "id": id.as_text(), "field": name, "value": value })));
            }
            ui.end_row();
        }
    });
    ui.horizontal(|ui| {
        let r = ui.add(egui::TextEdit::singleline(&mut v.new_field).hint_text("Add a field").desired_width(160.0));
        r.widget_info(|| egui::WidgetInfo::labeled(egui::Role::TextInput, true, "Add a field"));
        if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) && !v.new_field.trim().is_empty() {
            v.fields.push((v.new_field.trim().to_owned(), String::new(), FieldType::Text));
            v.new_field.clear();
        }
    });
    // Backlinks (BIB-3).
    let b = entries::backlinks(id, &e, documents(&s.project));
    ui.collapsing(RichText::new(format!("Linked from {} · mentioned in {}", b.linked.len(), b.mentioned.len())), |ui| {
        for (title, list) in [("Linked from", &b.linked), ("Mentioned in", &b.mentioned)] {
            if list.is_empty() {
                continue;
            }
            ui.label(RichText::new(title).weak());
            for d in list {
                let t = s.project.binder.get(*d).map(|i| i.title.clone()).unwrap_or_default();
                if ui.link(t).clicked() {
                    asks.push(Ask::Open(*d));
                }
            }
        }
    });
    for (cmd, params) in todo {
        if let Err(err) = s.run(cmd, &params) {
            errors.push(err);
        }
    }
    if s.revision() != rev {
        v.load(&s.project, id, s.revision());
    }
    (asks, errors)
}
