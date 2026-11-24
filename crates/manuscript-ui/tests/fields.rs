// SPDX-License-Identifier: MIT OR Apache-2.0
//! Custom fields (INS-6): defined per project, checked by kind, set per item, shown as outline
//! columns, renamed without losing values, saved and read back.

use serde_json::json;
use wordsmith_manuscript_model::{Area, FieldKind, Item, ItemId, ItemText, Kind, Place, Project, Role, Text};
use wordsmith_manuscript_ui::session::Session;

fn id(n: u64) -> ItemId {
    ItemId::from_counter(n)
}

fn session() -> Session {
    let mut p = Project::new();
    for n in 1..=2 {
        p.binder.insert(Place::end_of(Area::Manuscript), Item::with_id(id(n), Kind::Text, format!("Scene {n}")).role(Role::Scene)).unwrap();
        p.texts.insert(id(n), ItemText::with_body(Text::from_paragraphs(&["Words."])));
    }
    Session::new(p)
}

fn fields() -> serde_json::Value {
    json!([
        { "name": "Point of view" },
        { "name": "Scene date", "kind": "date" },
        { "name": "Checked", "kind": "checkbox" },
        { "name": "Weather", "kind": "list", "choices": ["Rain", "Fog"] },
    ])
}

#[test]
fn fields_are_checked_shown_in_the_outline_and_kept() {
    let mut s = session();
    s.run("project.setFields", &json!({ "fields": fields() })).unwrap();
    assert_eq!(s.project.settings.fields[3].kind, FieldKind::List);
    let set = |s: &mut Session, f: &str, v: &str| s.run("item.setField", &json!({ "id": id(1).as_text(), "field": f, "value": v }));
    set(&mut s, "Scene date", "1815-06-18").unwrap();
    assert!(set(&mut s, "Scene date", "the eighteenth").is_err());
    set(&mut s, "Checked", "true").unwrap();
    set(&mut s, "Weather", "Fog").unwrap();
    assert!(set(&mut s, "Weather", "Snow").is_err());
    let out = s.run("outline.list", &json!({})).unwrap();
    assert_eq!(out[0]["Scene date"], "1815-06-18");
    assert_eq!(out[0]["Weather"], "Fog");
    assert_eq!(out[0]["Checked"], "✓");
    assert_eq!(out[1]["Weather"], "");
    // Sorting by a field column orders the binder by it.
    s.run("item.setField", &json!({ "id": id(2).as_text(), "field": "Scene date", "value": "1815-06-01" })).unwrap();
    s.run("outline.applySort", &json!({ "column": 6 })).unwrap();
    assert_eq!(s.project.binder.area(Area::Manuscript)[0].id, id(2));
    // Renaming keeps the values; removing a field in use asks first.
    s.run("project.renameField", &json!({ "from": "Weather", "to": "Sky" })).unwrap();
    assert_eq!(s.project.text(id(1)).unwrap().meta.fields["Sky"], "Fog");
    let fewer = json!([{ "name": "Point of view" }, { "name": "Scene date", "kind": "date" }, { "name": "Checked", "kind": "checkbox" }]);
    assert!(s.run("project.setFields", &json!({ "fields": fewer })).is_err());
    assert_eq!(s.run("project.setFields", &json!({ "fields": fewer, "clear": true })).unwrap()["cleared"], 1);
    assert!(!s.project.text(id(1)).unwrap().meta.fields.contains_key("Sky"));
    s.undo();
    assert_eq!(s.project.text(id(1)).unwrap().meta.fields["Sky"], "Fog");
    // Saved and read back.
    let dir = std::env::temp_dir().join(format!("wsm-fields-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    s.save_as(&dir).unwrap();
    let (_, back) = wordsmith_manuscript_format::folder::Folder::open(&dir).unwrap();
    assert_eq!(back.settings.fields, s.project.settings.fields);
    assert_eq!(back.text(id(1)).unwrap().meta.fields, s.project.text(id(1)).unwrap().meta.fields);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn bad_field_lists_are_refused() {
    let mut s = session();
    assert!(s.run("project.setFields", &json!({ "fields": [{ "name": "A" }, { "name": "A" }] })).is_err());
    assert!(s.run("project.setFields", &json!({ "fields": [{ "name": "W", "kind": "list" }] })).is_err());
    assert!(s.run("project.setFields", &json!({ "fields": [{ "name": "W", "kind": "colour" }] })).is_err());
    // Changing a field's kind drops only the values the new kind refuses (asked first).
    s.run("item.setField", &json!({ "id": id(1).as_text(), "field": "Point of view", "value": "Mara" })).unwrap();
    assert!(s.run("project.setFields", &json!({ "fields": [{ "name": "Point of view", "kind": "date" }] })).is_err());
}
