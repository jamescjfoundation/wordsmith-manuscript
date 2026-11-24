// SPDX-License-Identifier: MIT OR Apache-2.0
//! The writer's own bible entry types (BIB-2): change the fields of a type, add a type, rename a
//! field (every entry keeps its value), or put a changed default type back.
//!
//! Templates are data (`[extra] templates` in the project settings, read by
//! `manuscript-model::bible::templates`): a type the writer changes or adds is kept there, and
//! replaces the default of the same key. Entries never lose a value when their template changes:
//! a value whose field left the template is still shown with the entry, after the template's
//! fields.

use jcj_shell::egui::{self, RichText, Ui};
use serde_json::{Value, json};
use wordsmith_manuscript_model::entries::{FieldType, Template, TemplateField};
use wordsmith_manuscript_model::{ItemId, Project, bible};

use crate::commands::{Spec, str_param};
use crate::session::{Outcome, Session};

/// The settings key.
pub const KEY: &str = "templates";

/// The field types, as commands name them.
pub const FIELD_TYPES: [(&str, FieldType, &str); 5] = [
    ("text", FieldType::Text, "Text"),
    ("long-text", FieldType::LongText, "Long text"),
    ("list", FieldType::List, "List"),
    ("image", FieldType::Image, "Picture"),
    ("link", FieldType::Link, "Link to an entry"),
];

fn custom(p: &Project) -> Vec<Template> {
    p.settings.extra.get(KEY).and_then(|v| serde_json::from_value(v.clone()).ok()).unwrap_or_default()
}

fn store(p: &mut Project, list: &[Template]) -> Result<(), String> {
    if list.is_empty() {
        p.settings.extra.remove(KEY);
    } else {
        p.settings.extra.insert(KEY.into(), serde_json::to_value(list).map_err(|e| e.to_string())?);
    }
    Ok(())
}

/// A type key made from a name (`Ship's Crew` → `ships-crew`).
pub fn key_of(name: &str) -> String {
    let mut out = String::new();
    for c in name.trim().to_lowercase().chars() {
        if c.is_alphanumeric() {
            out.push(c);
        } else if (c.is_whitespace() || c == '-' || c == '_') && !out.ends_with('-') && !out.is_empty() {
            out.push('-');
        }
    }
    out.trim_end_matches('-').to_owned()
}

fn template_of(v: &Value) -> Result<Template, String> {
    let kind = str_param(v, "kind")?.trim().to_owned();
    if kind.is_empty() || key_of(&kind) != kind {
        return Err(format!("`{kind}` is not a type key (lowercase letters, digits and hyphens)"));
    }
    let title = str_param(v, "title")?.trim().to_owned();
    if title.is_empty() {
        return Err("a type needs a name".into());
    }
    let plural = v.get("plural").and_then(Value::as_str).map(str::trim).filter(|p| !p.is_empty()).map_or_else(|| format!("{title}s"), str::to_owned);
    let list = v.get("fields").and_then(Value::as_array).ok_or("`fields` (a list) is required")?;
    let mut fields: Vec<TemplateField> = Vec::new();
    for f in list {
        let name = f.get("name").and_then(Value::as_str).map(str::trim).unwrap_or("").to_owned();
        if name.is_empty() {
            return Err("every field needs a name".into());
        }
        if fields.iter().any(|x| x.name == name) {
            return Err(format!("two fields are named `{name}`"));
        }
        let k = f.get("type").and_then(Value::as_str).unwrap_or("text");
        let kind = FIELD_TYPES
            .iter()
            .find(|(key, _, _)| *key == k)
            .map(|(_, t, _)| *t)
            .ok_or_else(|| format!("`{k}` is not a field type (text, long-text, list, image, link)"))?;
        fields.push(TemplateField { name, kind });
    }
    Ok(Template { kind, title, plural, fields })
}

/// The entry type commands.
pub fn specs() -> Vec<Spec> {
    let spec = |id, label, params, about| Spec { id, label, key: "", params, about, mutates: true };
    vec![
        spec(
            "bible.setTemplate",
            "Set Entry Type",
            r#"{"kind": string, "title": string, "plural"?: string, "fields": [{"name": string, "type"?: "text"|"long-text"|"list"|"image"|"link"}]}"#,
            "Add an entry type, or change one's name and fields (entries keep every value).",
        ),
        spec(
            "bible.renameTemplateField",
            "Rename Entry Field",
            r#"{"kind": string, "from": string, "to": string}"#,
            "Rename a field of an entry type; every entry of the type keeps its value under the new name.",
        ),
        spec(
            "bible.resetTemplate",
            "Reset Entry Type",
            r#"{"kind": string}"#,
            "Put a changed default type back as it was, or remove an added type no entry uses.",
        ),
    ]
}

/// Run an entry type command, or `None` if `id` is not one.
pub fn run(s: &mut Session, id: &str, v: &Value) -> Option<Outcome> {
    Some(match id {
        "bible.setTemplate" => (|| {
            let t = template_of(v)?;
            let mut list = custom(&s.project);
            match list.iter_mut().find(|x| x.kind == t.kind) {
                Some(x) => *x = t.clone(),
                None => list.push(t.clone()),
            }
            s.change("Set Entry Type", &[], &[], |p| store(p, &list))?;
            Ok(json!({ "kind": t.kind }))
        })(),
        "bible.renameTemplateField" => (|| {
            let kind = str_param(v, "kind")?.to_owned();
            let from = str_param(v, "from")?.to_owned();
            let to = str_param(v, "to")?.trim().to_owned();
            let mut t = bible::template(&s.project, &kind).ok_or_else(|| format!("`{kind}` is not an entry type of this project"))?;
            if to.is_empty() {
                return Err("the new name needs some text".into());
            }
            if !t.fields.iter().any(|f| f.name == from) {
                return Err(format!("`{kind}` has no field `{from}`"));
            }
            if from != to && t.fields.iter().any(|f| f.name == to) {
                return Err(format!("`{kind}` already has a field `{to}`"));
            }
            for f in t.fields.iter_mut().filter(|f| f.name == from) {
                f.name.clone_from(&to);
            }
            let entries: Vec<ItemId> =
                s.project.entries.iter().filter(|(_, e)| e.kind == kind && e.fields.contains_key(&from)).map(|(i, _)| *i).collect();
            if entries.iter().any(|i| s.project.entries.get(i).is_some_and(|e| e.fields.contains_key(&to) && from != to)) {
                return Err(format!("an entry already has a value named `{to}`: rename that one first"));
            }
            let mut list = custom(&s.project);
            match list.iter_mut().find(|x| x.kind == t.kind) {
                Some(x) => *x = t.clone(),
                None => list.push(t.clone()),
            }
            s.change("Rename Entry Field", &[], &entries, |p| {
                store(p, &list)?;
                for i in &entries {
                    if let Some(e) = p.entries.get_mut(i)
                        && let Some(val) = e.fields.remove(&from)
                    {
                        e.fields.insert(to.clone(), val);
                    }
                }
                Ok(())
            })?;
            Ok(json!({ "entries": entries.len() }))
        })(),
        "bible.resetTemplate" => (|| {
            let kind = str_param(v, "kind")?.to_owned();
            let mut list = custom(&s.project);
            if !list.iter().any(|t| t.kind == kind) {
                return Err(format!("`{kind}` is not changed from its default"));
            }
            let is_default = bible::default_templates().iter().any(|t| t.kind == kind);
            let used = s.project.entries.values().filter(|e| e.kind == kind).count();
            if !is_default && used > 0 {
                return Err(format!("{used} entr{} of this type: change their type first", if used == 1 { "y is" } else { "ies are" }));
            }
            list.retain(|t| t.kind != kind);
            s.change("Reset Entry Type", &[], &[], |p| store(p, &list))?;
            Ok(json!({}))
        })(),
        _ => return None,
    })
}

/// The entry types window's state.
#[derive(Default)]
pub struct TypesView {
    /// Whether the window is open.
    pub open: bool,
    selected: Option<String>,
    draft: Option<Template>,
    new_type: String,
    new_field: String,
}

/// Draw the entry types window. Returns errors for the status bar.
pub fn window(ctx: &egui::Context, s: &mut Session, v: &mut TypesView) -> Vec<String> {
    let mut errors = Vec::new();
    if !v.open {
        return errors;
    }
    let mut open = true;
    egui::Window::new("Entry Types").id(egui::Id::unique("manuscript-entry-types")).open(&mut open).default_size([560.0, 420.0]).show(ctx, |ui| {
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.set_width(170.0);
                for t in bible::templates(&s.project.settings) {
                    if ui.selectable_label(v.selected.as_deref() == Some(t.kind.as_str()), &t.title).clicked() {
                        v.selected = Some(t.kind.clone());
                        v.draft = Some(t);
                    }
                }
                ui.separator();
                let r = ui.add(egui::TextEdit::singleline(&mut v.new_type).hint_text("New type (Ship, Spell…)").desired_width(160.0));
                r.widget_info(|| egui::WidgetInfo::labeled(egui::Role::TextInput, true, "New entry type"));
                if ui.button("Add Type").clicked() && !v.new_type.trim().is_empty() {
                    let title = v.new_type.trim().to_owned();
                    let kind = key_of(&title);
                    match s.run(
                        "bible.setTemplate",
                        &json!({ "kind": kind, "title": title, "fields": [{ "name": "Description", "type": "long-text" }] }),
                    ) {
                        Ok(_) => {
                            v.selected = Some(kind.clone());
                            v.draft = bible::template(&s.project, &kind);
                            v.new_type.clear();
                        }
                        Err(e) => errors.push(e),
                    }
                }
            });
            ui.separator();
            ui.vertical(|ui| edit(ui, s, v, &mut errors));
        });
    });
    if !open {
        v.open = false;
    }
    errors
}

fn edit(ui: &mut Ui, s: &mut Session, v: &mut TypesView, errors: &mut Vec<String>) {
    let Some(d) = v.draft.as_mut() else {
        ui.label(RichText::new("Choose a type to change its fields.").weak());
        return;
    };
    let original = bible::template(&s.project, &d.kind);
    let used = s.project.entries.values().filter(|e| e.kind == d.kind).count();
    ui.horizontal(|ui| {
        let l = ui.label("Name");
        ui.add(egui::TextEdit::singleline(&mut d.title).desired_width(140.0)).labelled_by(l.id);
        let l = ui.label("Folder");
        ui.add(egui::TextEdit::singleline(&mut d.plural).desired_width(140.0)).labelled_by(l.id);
    });
    ui.label(RichText::new(format!("{used} entr{} of this type", if used == 1 { "y" } else { "ies" })).weak());
    let mut remove = None;
    let mut up = None;
    let mut rename: Option<(String, String)> = None;
    for (i, f) in d.fields.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            let before = original.as_ref().and_then(|o| o.fields.get(i)).map(|x| x.name.clone());
            let r = ui.add(egui::TextEdit::singleline(&mut f.name).desired_width(150.0));
            r.widget_info(|| egui::WidgetInfo::labeled(egui::Role::TextInput, true, format!("Field {}", i + 1)));
            if r.lost_focus()
                && let Some(b) = before
                && b != f.name
                && !f.name.trim().is_empty()
            {
                rename = Some((b, f.name.trim().to_owned()));
            }
            let current = FIELD_TYPES.iter().find(|(_, t, _)| *t == f.kind).map_or("Text", |x| x.2);
            let c = egui::ComboBox::from_id_salt(("entry-type-field", i)).selected_text(current).show_ui(ui, |ui| {
                for (_, t, label) in FIELD_TYPES {
                    ui.selectable_value(&mut f.kind, t, label);
                }
            });
            c.response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::ComboBox, true, format!("Type of field {}", i + 1)));
            if ui.add_enabled(i > 0, egui::Button::new("↑").small()).on_hover_text("Move up").clicked() {
                up = Some(i);
            }
            if ui.small_button("Remove").on_hover_text("Entries keep their values").clicked() {
                remove = Some(i);
            }
        });
    }
    if let Some(i) = up {
        d.fields.swap(i, i - 1);
    }
    if let Some(i) = remove {
        d.fields.remove(i);
    }
    ui.horizontal(|ui| {
        let r = ui.add(egui::TextEdit::singleline(&mut v.new_field).hint_text("New field").desired_width(150.0));
        r.widget_info(|| egui::WidgetInfo::labeled(egui::Role::TextInput, true, "New field"));
        if ui.button("Add Field").clicked() && !v.new_field.trim().is_empty() {
            d.fields.push(TemplateField { name: v.new_field.trim().to_owned(), kind: FieldType::Text });
            v.new_field.clear();
        }
    });
    if let Some((from, to)) = rename {
        let kind = d.kind.clone();
        match s.run("bible.renameTemplateField", &json!({ "kind": kind, "from": from, "to": to })) {
            Ok(_) => v.draft = bible::template(&s.project, &kind),
            Err(e) => errors.push(e),
        }
        return;
    }
    ui.add_space(6.0);
    let mut reset: Option<String> = None;
    ui.horizontal(|ui| {
        let changed = original.as_ref() != Some(&*d);
        if ui.add_enabled(changed, egui::Button::new("Save Type")).clicked() {
            let fields: Vec<Value> = d
                .fields
                .iter()
                .map(|f| json!({ "name": f.name, "type": FIELD_TYPES.iter().find(|(_, t, _)| *t == f.kind).map_or("text", |x| x.0) }))
                .collect();
            if let Err(e) = s.run("bible.setTemplate", &json!({ "kind": d.kind, "title": d.title, "plural": d.plural, "fields": fields })) {
                errors.push(e);
            }
        }
        let is_custom = custom(&s.project).iter().any(|t| t.kind == d.kind);
        let is_default = bible::default_templates().iter().any(|t| t.kind == d.kind);
        let label = if is_default { "Back to the Default" } else { "Remove Type" };
        if is_custom && ui.button(label).clicked() {
            reset = Some(d.kind.clone());
        }
    });
    if let Some(kind) = reset {
        match s.run("bible.resetTemplate", &json!({ "kind": kind })) {
            Ok(_) => {
                v.draft = bible::template(&s.project, &kind);
                if v.draft.is_none() {
                    v.selected = None;
                }
            }
            Err(e) => errors.push(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_from_names() {
        assert_eq!(key_of("Ship's Crew"), "ships-crew");
        assert_eq!(key_of("  Spell  "), "spell");
        assert_eq!(key_of("Ünterwelt 2"), "ünterwelt-2");
    }
}
