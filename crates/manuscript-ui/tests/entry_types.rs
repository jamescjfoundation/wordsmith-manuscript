// SPDX-License-Identifier: MIT OR Apache-2.0
//! The writer's own entry types (BIB-2): changed and added types, renamed fields carrying every
//! entry's value, values never lost, defaults put back; the window drives the same commands.

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use jcj_shell::egui;
use serde_json::json;
use wordsmith_manuscript_model::{ItemId, Project, bible};
use wordsmith_manuscript_ui::entry_types::{self, TypesView};
use wordsmith_manuscript_ui::session::Session;

fn new_entry(s: &mut Session, kind: &str, name: &str) -> ItemId {
    s.run("bible.new", &json!({ "kind": kind, "name": name })).unwrap()["id"].as_str().unwrap().parse().unwrap()
}

#[test]
fn types_change_and_values_stay() {
    let mut s = Session::new(Project::new());
    let mara = new_entry(&mut s, "character", "Mara");
    s.run("entry.setField", &json!({ "id": mara.as_text(), "field": "Age", "value": "34" })).unwrap();
    // A changed default: fields replaced; Mara's age is still there, after the template's fields.
    s.run("bible.setTemplate", &json!({ "kind": "character", "title": "Character", "plural": "Characters", "fields": [{ "name": "Pronouns" }, { "name": "Voice", "type": "long-text" }] })).unwrap();
    let got = s.run("entry.get", &json!({ "id": mara.as_text() })).unwrap();
    assert_eq!(
        got["fields"].as_array().unwrap().iter().map(|f| f[0].as_str().unwrap_or(f["name"].as_str().unwrap_or(""))).collect::<Vec<_>>().len(),
        3,
        "{got}"
    );
    assert_eq!(s.project.entries[&mara].fields["Age"], "34");
    // Renaming a field carries the value.
    s.run("bible.setTemplate", &json!({ "kind": "character", "title": "Character", "fields": [{ "name": "Age" }] })).unwrap();
    s.run("bible.renameTemplateField", &json!({ "kind": "character", "from": "Age", "to": "Age at the start" })).unwrap();
    assert_eq!(s.project.entries[&mara].fields["Age at the start"], "34");
    assert!(!s.project.entries[&mara].fields.contains_key("Age"));
    s.undo();
    assert_eq!(s.project.entries[&mara].fields["Age"], "34");
    // A new type gets its own folder with its first entry; it cannot be removed while used.
    s.run("bible.setTemplate", &json!({ "kind": "ship", "title": "Ship", "fields": [{ "name": "Crew", "type": "list" }] })).unwrap();
    let ship = new_entry(&mut s, "ship", "The Heron");
    assert!(
        s.project.binder.area(wordsmith_manuscript_model::Area::Bible).iter().any(|i| i.title == "Ships" && i.children.iter().any(|c| c.id == ship))
    );
    assert!(s.run("bible.resetTemplate", &json!({ "kind": "ship" })).is_err());
    // A default put back.
    s.run("bible.resetTemplate", &json!({ "kind": "character" })).unwrap();
    assert_eq!(bible::template(&s.project, "character"), bible::default_templates().into_iter().next());
    assert!(s.run("bible.setTemplate", &json!({ "kind": "Bad Key", "title": "X", "fields": [] })).is_err());
    assert!(s.run("bible.setTemplate", &json!({ "kind": "x", "title": "X", "fields": [{ "name": "A" }, { "name": "A" }] })).is_err());
}

#[test]
fn the_window_adds_a_type() {
    struct St {
        s: Session,
        v: TypesView,
        styled: bool,
    }
    let mut v = TypesView::default();
    v.open = true;
    let mut h = Harness::builder().with_size(egui::vec2(800.0, 600.0)).build_ui_state(
        |ui, st: &mut St| {
            if !st.styled {
                wordsmith_manuscript_ui::brand::apply(ui.ctx(), jcj_shell::theme::Mode::Light);
                st.styled = true;
            }
            let _ = entry_types::window(ui.ctx(), &mut st.s, &mut st.v);
        },
        St { s: Session::new(Project::new()), v, styled: false },
    );
    h.run();
    h.get_by_label("New entry type").focus();
    h.get_by_label("New entry type").type_text("Spell");
    h.run();
    h.get_by_label("Add Type").click();
    h.run();
    assert!(bible::template(&h.state().s.project, "spell").is_some());
    assert!(h.query_by_label("Save Type").is_some());
}
