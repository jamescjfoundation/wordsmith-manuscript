// SPDX-License-Identifier: MIT OR Apache-2.0
//! Custom fields (INS-6): text, date, checkbox and list fields the project defines, set per item
//! in the inspector and shown as outline columns. Point of view is the first, built in.

use jcj_shell::egui::{self, Ui};
use serde_json::{Value, json};
use wordsmith_manuscript_model::{FieldDef, FieldKind, ItemId, Project};

use crate::commands::{Spec, str_param};
use crate::session::{Outcome, Session};

/// The value a checkbox field holds when ticked (an unticked one holds none).
pub const YES: &str = "yes";

/// The field commands.
pub fn specs() -> Vec<Spec> {
    vec![
        Spec {
            id: "project.setFields",
            label: "Set Fields",
            key: "",
            params: r#"{"fields": [{"name": string, "kind"?: "text"|"date"|"checkbox"|"list", "choices"?: [string]}], "clear"?: bool}"#,
            about: "The project's custom fields in order; removing one that items use needs clear: true (their values go).",
            mutates: true,
        },
        Spec {
            id: "project.renameField",
            label: "Rename Field",
            key: "",
            params: r#"{"from": string, "to": string}"#,
            about: "Rename a custom field; every item keeps its value.",
            mutates: true,
        },
    ]
}

/// The key a field kind has in commands.
pub fn kind_key(k: FieldKind) -> &'static str {
    match k {
        FieldKind::Text => "text",
        FieldKind::Date => "date",
        FieldKind::Checkbox => "checkbox",
        FieldKind::List => "list",
    }
}

fn kind_of(k: &str) -> Result<FieldKind, String> {
    Ok(match k {
        "text" => FieldKind::Text,
        "date" => FieldKind::Date,
        "checkbox" => FieldKind::Checkbox,
        "list" => FieldKind::List,
        other => return Err(format!("`{other}` is not a field kind (text, date, checkbox or list)")),
    })
}

/// Whether `s` is a calendar date written `YYYY-MM-DD`.
pub fn is_date(s: &str) -> bool {
    chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").is_ok() && s.len() == 10
}

/// The value to keep for a field, checked against its kind ("" clears).
pub fn checked(def: &FieldDef, value: &str) -> Result<String, String> {
    let v = value.trim();
    if v.is_empty() {
        return Ok(String::new());
    }
    match def.kind {
        FieldKind::Text => Ok(v.to_owned()),
        FieldKind::Date if is_date(v) => Ok(v.to_owned()),
        FieldKind::Date => Err(format!("{}: `{v}` is not a date written YYYY-MM-DD", def.name)),
        FieldKind::Checkbox => match v.to_lowercase().as_str() {
            "yes" | "true" | "1" | "on" => Ok(YES.into()),
            "no" | "false" | "0" | "off" => Ok(String::new()),
            _ => Err(format!("{}: a checkbox is yes or no", def.name)),
        },
        FieldKind::List if def.choices.iter().any(|c| c == v) => Ok(v.to_owned()),
        FieldKind::List => Err(format!("{}: `{v}` is not one of its choices ({})", def.name, def.choices.join(", "))),
    }
}

fn uses(p: &Project, name: &str) -> usize {
    p.texts.values().filter(|t| t.meta.fields.contains_key(name)).count()
}

fn defs_of(v: &Value) -> Result<Vec<FieldDef>, String> {
    let list = v.get("fields").and_then(Value::as_array).ok_or("`fields` (a list) is required")?;
    let mut out: Vec<FieldDef> = Vec::new();
    for f in list {
        let name = f.get("name").and_then(Value::as_str).map(str::trim).unwrap_or("").to_owned();
        if name.is_empty() {
            return Err("every field needs a name".into());
        }
        if out.iter().any(|d| d.name == name) {
            return Err(format!("two fields are named `{name}`"));
        }
        let kind = kind_of(f.get("kind").and_then(Value::as_str).unwrap_or("text"))?;
        let choices: Vec<String> = f
            .get("choices")
            .and_then(Value::as_array)
            .map(|c| c.iter().filter_map(Value::as_str).map(|s| s.trim().to_owned()).filter(|s| !s.is_empty()).collect())
            .unwrap_or_default();
        if kind == FieldKind::List && choices.is_empty() {
            return Err(format!("the list field `{name}` needs choices"));
        }
        out.push(FieldDef { name, kind, choices: if kind == FieldKind::List { choices } else { Vec::new() } });
    }
    Ok(out)
}

/// Run a field command, or `None` if `id` is not one.
pub fn run(s: &mut Session, id: &str, v: &Value) -> Option<Outcome> {
    Some(match id {
        "project.setFields" => (|| {
            let new = defs_of(v)?;
            let clear = v.get("clear").and_then(Value::as_bool).unwrap_or(false);
            // A field that stays, or changes kind, keeps only the values its new kind accepts.
            let mut dropped = 0usize;
            let mut touched: Vec<ItemId> = Vec::new();
            for (tid, t) in &s.project.texts {
                for (k, val) in &t.meta.fields {
                    let keep = new.iter().find(|d| &d.name == k).is_some_and(|d| checked(d, val).is_ok_and(|c| &c == val));
                    if !keep {
                        dropped += 1;
                        touched.push(*tid);
                    }
                }
            }
            if dropped > 0 && !clear {
                return Err(format!(
                    "{dropped} value{} would be lost (a field removed, or values its new kind does not accept): pass \"clear\": true to drop them",
                    if dropped == 1 { "" } else { "s" }
                ));
            }
            touched.dedup();
            s.change("Set Fields", &touched, &[], |p| {
                for i in &touched {
                    let Some(t) = p.texts.get_mut(i) else { continue };
                    t.meta.fields.retain(|k, val| new.iter().find(|d| &d.name == k).is_some_and(|d| checked(d, val).is_ok_and(|c| &c == val)));
                }
                p.settings.fields = new;
                Ok(())
            })?;
            Ok(json!({ "cleared": dropped }))
        })(),
        "project.renameField" => (|| {
            let from = str_param(v, "from")?.to_owned();
            let to = str_param(v, "to")?.trim().to_owned();
            if to.is_empty() {
                return Err("the new name needs some text".into());
            }
            if !s.project.settings.fields.iter().any(|f| f.name == from) {
                return Err(format!("`{from}` is not one of the project's fields"));
            }
            if from != to && s.project.settings.fields.iter().any(|f| f.name == to) {
                return Err(format!("there is already a field `{to}`"));
            }
            let touched: Vec<ItemId> = s.project.texts.iter().filter(|(_, t)| t.meta.fields.contains_key(&from)).map(|(i, _)| *i).collect();
            s.change("Rename Field", &touched, &[], |p| {
                for f in p.settings.fields.iter_mut().filter(|f| f.name == from) {
                    f.name.clone_from(&to);
                }
                for i in &touched {
                    let Some(t) = p.texts.get_mut(i) else { continue };
                    if let Some(val) = t.meta.fields.remove(&from) {
                        t.meta.fields.insert(to.clone(), val);
                    }
                }
                Ok(())
            })?;
            Ok(json!({ "items": touched.len() }))
        })(),
        _ => return None,
    })
}

/// The project's fields as `project.setFields` takes them.
pub fn defs_json(p: &Project) -> Value {
    json!(p.settings.fields.iter().map(|f| json!({ "name": f.name, "kind": kind_key(f.kind), "choices": f.choices })).collect::<Vec<_>>())
}

/// How many items use a field.
pub fn in_use(p: &Project, name: &str) -> usize {
    uses(p, name)
}

/// The field editors of one item in the inspector. `buffers` holds text being typed (committed
/// when the field loses focus).
pub fn item_section(ui: &mut Ui, s: &mut Session, id: ItemId, buffers: &mut std::collections::BTreeMap<String, String>, errors: &mut Vec<String>) {
    let defs = s.project.settings.fields.clone();
    let current = s.project.text(id).map(|t| t.meta.fields.clone()).unwrap_or_default();
    for def in defs {
        let now = current.get(&def.name).cloned().unwrap_or_default();
        let mut set = |s: &mut Session, value: String| {
            if let Err(e) = s.run("item.setField", &json!({ "id": id.as_text(), "field": def.name, "value": value })) {
                errors.push(e);
            }
        };
        ui.horizontal(|ui| {
            let l = ui.label(&def.name);
            match def.kind {
                FieldKind::Text | FieldKind::Date => {
                    let buf = buffers.entry(def.name.clone()).or_insert_with(|| now.clone());
                    let mut edit = egui::TextEdit::singleline(buf);
                    if def.kind == FieldKind::Date {
                        edit = edit.hint_text("YYYY-MM-DD").desired_width(100.0);
                    }
                    let r = ui.add(edit).labelled_by(l.id);
                    if r.lost_focus() && buf.trim() != now {
                        let value = buf.clone();
                        set(s, value);
                    }
                }
                FieldKind::Checkbox => {
                    let mut on = now == YES;
                    let r = ui.checkbox(&mut on, "").labelled_by(l.id);
                    if r.changed() {
                        set(s, if on { YES.into() } else { String::new() });
                    }
                }
                FieldKind::List => {
                    let c = egui::ComboBox::from_id_salt(("inspector-field", def.name.as_str()))
                        .selected_text(if now.is_empty() { "None" } else { now.as_str() })
                        .show_ui(ui, |ui| {
                            if ui.selectable_label(now.is_empty(), "None").clicked() {
                                set(s, String::new());
                            }
                            for c in &def.choices {
                                if ui.selectable_label(&now == c, c).clicked() {
                                    set(s, c.clone());
                                }
                            }
                        });
                    c.response.labelled_by(l.id);
                }
            }
        });
    }
}

/// Adding and removing the project's fields, under the item's fields in the inspector.
#[derive(Default)]
pub struct FieldsEditor {
    name: String,
    kind: usize,
    choices: String,
}

const KINDS: [(&str, FieldKind); 4] =
    [("Text", FieldKind::Text), ("Date", FieldKind::Date), ("Checkbox", FieldKind::Checkbox), ("List", FieldKind::List)];

/// Draw the project's field list with add and remove.
pub fn project_section(ui: &mut Ui, s: &mut Session, ed: &mut FieldsEditor, errors: &mut Vec<String>) {
    egui::CollapsingHeader::new("Project fields").id_salt("inspector-project-fields").show(ui, |ui| {
        for f in s.project.settings.fields.clone() {
            ui.horizontal(|ui| {
                let used = in_use(&s.project, &f.name);
                ui.label(format!("{} ({}{})", f.name, kind_key(f.kind), if used > 0 { format!(", {used} items") } else { String::new() }));
                if ui.small_button("Remove").on_hover_text("Remove the field (items lose its value)").clicked() {
                    let mut defs = defs_json(&s.project);
                    if let Some(a) = defs.as_array_mut() {
                        a.retain(|d| d["name"] != json!(f.name));
                    }
                    if let Err(e) = s.run("project.setFields", &json!({ "fields": defs, "clear": true })) {
                        errors.push(e);
                    }
                }
            });
        }
        ui.horizontal(|ui| {
            let r = ui.add(egui::TextEdit::singleline(&mut ed.name).hint_text("New field").desired_width(110.0));
            r.widget_info(|| egui::WidgetInfo::labeled(egui::Role::TextInput, true, "New field name"));
            let c = egui::ComboBox::from_id_salt("inspector-new-field-kind").selected_text(KINDS[ed.kind.min(3)].0).show_ui(ui, |ui| {
                for (n, (label, _)) in KINDS.iter().enumerate() {
                    ui.selectable_value(&mut ed.kind, n, *label);
                }
            });
            c.response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::ComboBox, true, "New field kind"));
        });
        if KINDS[ed.kind.min(3)].1 == FieldKind::List {
            let r = ui.add(egui::TextEdit::singleline(&mut ed.choices).hint_text("Choices, separated by commas"));
            r.widget_info(|| egui::WidgetInfo::labeled(egui::Role::TextInput, true, "New field choices"));
        }
        if ui.button("Add Field").clicked() && !ed.name.trim().is_empty() {
            let mut defs = defs_json(&s.project);
            let choices: Vec<String> = ed.choices.split(',').map(|c| c.trim().to_owned()).filter(|c| !c.is_empty()).collect();
            if let Some(a) = defs.as_array_mut() {
                a.push(json!({ "name": ed.name.trim(), "kind": kind_key(KINDS[ed.kind.min(3)].1), "choices": choices }));
            }
            match s.run("project.setFields", &json!({ "fields": defs })) {
                Ok(_) => *ed = FieldsEditor::default(),
                Err(e) => errors.push(e),
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_are_checked_by_kind() {
        let date = FieldDef { name: "Day".into(), kind: FieldKind::Date, choices: Vec::new() };
        assert_eq!(checked(&date, " 1815-06-18 ").unwrap(), "1815-06-18");
        assert!(checked(&date, "1815-02-30").is_err());
        assert!(checked(&date, "June").is_err());
        let check = FieldDef { name: "Done".into(), kind: FieldKind::Checkbox, choices: Vec::new() };
        assert_eq!(checked(&check, "True").unwrap(), YES);
        assert_eq!(checked(&check, "no").unwrap(), "");
        let list = FieldDef { name: "Weather".into(), kind: FieldKind::List, choices: vec!["Rain".into(), "Sun".into()] };
        assert_eq!(checked(&list, "Rain").unwrap(), "Rain");
        assert!(checked(&list, "Snow").is_err());
    }
}
