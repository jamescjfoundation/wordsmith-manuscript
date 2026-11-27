// SPDX-License-Identifier: MIT OR Apache-2.0
//! The inspector's commands, the project's lists, snapshots, and the panel in the harness.

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use jcj_shell::egui;
use serde_json::{Value, json};
use wordsmith_manuscript_model::{Area, Entry, Item, ItemId, ItemText, Kind, Place, Project, Text};
use wordsmith_manuscript_ui::inspector::{self, InspectorView};
use wordsmith_manuscript_ui::session::Session;

fn id(n: u64) -> ItemId {
    ItemId::from_counter(n)
}

fn session() -> Session {
    let mut p = Project::new();
    p.binder.insert(Place::end_of(Area::Manuscript), Item::with_id(id(1), Kind::Text, "Arrival")).unwrap();
    p.binder.insert(Place::end_of(Area::Manuscript), Item::with_id(id(2), Kind::Text, "Storm")).unwrap();
    p.binder.insert(Place::end_of(Area::Bible), Item::with_id(id(9), Kind::Entry, "Mara")).unwrap();
    p.entries.insert(id(9), Entry::new("character", "Mara"));
    p.texts.insert(id(1), ItemText::with_body(Text::from_paragraphs(&["The keeper came."])));
    Session::new(p)
}

fn run(s: &mut Session, cmd: &str, v: Value) {
    s.run(cmd, &v).unwrap_or_else(|e| panic!("{cmd}: {e}"));
}

#[test]
fn every_field_is_a_command_and_one_undo_step() {
    let mut s = session();
    let start = s.project.clone();
    let i = id(1).as_text();
    run(&mut s, "item.setSynopsis", json!({ "id": i, "synopsis": "Arrival at the light." }));
    run(&mut s, "item.setNotes", json!({ "id": i, "notes": "Check the *tide*." }));
    run(&mut s, "item.setLabel", json!({ "id": i, "label": "Idea" }));
    run(&mut s, "item.setStatus", json!({ "id": i, "status": "First draft" }));
    run(&mut s, "item.addKeyword", json!({ "id": i, "keyword": "night" }));
    run(&mut s, "item.linkEntry", json!({ "id": i, "entry": id(9).as_text() }));
    run(&mut s, "item.setTarget", json!({ "id": i, "words": 2500 }));
    run(&mut s, "item.setField", json!({ "id": i, "field": "Point of view", "value": "Mara" }));
    let got = s.run("item.get", &json!({ "id": i })).unwrap();
    assert_eq!(got["synopsis"], "Arrival at the light.");
    assert_eq!(got["label"], "Idea");
    assert_eq!(got["status"], "First draft");
    assert_eq!(got["keywords"], json!(["night"]));
    assert_eq!(got["entries"], json!([id(9).as_text()]));
    assert_eq!(got["target"], 2500);
    assert_eq!(got["fields"]["Point of view"], "Mara");
    assert!(s.project.settings.keywords.iter().any(|k| k.name == "night"), "a new keyword joins the project's list");
    for _ in 0..8 {
        s.undo();
    }
    assert_eq!(s.project, start);
}

#[test]
fn values_must_come_from_the_project_lists() {
    let mut s = session();
    let i = id(1).as_text();
    assert!(s.run("item.setLabel", &json!({ "id": i, "label": "Nonsense" })).is_err());
    assert!(s.run("item.setStatus", &json!({ "id": i, "status": "Nonsense" })).is_err());
    assert!(s.run("item.linkEntry", &json!({ "id": i, "entry": id(2).as_text() })).is_err(), "only bible entries link");
    assert!(s.run("item.setField", &json!({ "id": i, "field": "Unknown", "value": "x" })).is_err());
}

#[test]
fn removing_a_list_name_in_use_asks_first_then_clears_it() {
    let mut s = session();
    run(&mut s, "item.setLabel", json!({ "id": id(1).as_text(), "label": "Idea" }));
    run(&mut s, "item.addKeyword", json!({ "id": id(1).as_text(), "keyword": "night" }));
    let err = s.run("project.setLabels", &json!({ "labels": [{ "name": "Action", "color": "#C0504D" }] })).unwrap_err();
    assert!(err.contains("clear"));
    run(&mut s, "project.setLabels", json!({ "labels": [{ "name": "Action", "color": "#C0504D" }], "clear": true }));
    assert_eq!(s.project.binder.get(id(1)).unwrap().label, None);
    run(&mut s, "project.setKeywords", json!({ "keywords": [], "clear": true }));
    assert!(s.project.texts[&id(1)].meta.keywords.is_empty());
    run(&mut s, "project.setStatuses", json!({ "statuses": ["Plan", "Draft", "Done"] }));
    assert_eq!(s.project.settings.statuses, ["Plan", "Draft", "Done"]);
    assert!(s.run("project.setLabels", &json!({ "labels": [{ "name": "A", "color": "red" }] })).is_err());
    assert!(s.run("project.setLabels", &json!({ "labels": [{ "name": "A" }, { "name": "A" }] })).is_err());
}

#[test]
fn book_details() {
    let mut s = session();
    run(&mut s, "project.setBook", json!({ "title": "The Light", "author": "Test Author", "series_number": 2 }));
    assert_eq!(s.project.settings.book.title, "The Light");
    assert_eq!(s.project.settings.book.series_number, Some(2));
    assert!(s.run("project.setBook", &json!({ "nonsense": 1 })).is_err());
}

#[test]
fn snapshots_take_list_restore_and_delete() {
    let mut s = session();
    let i = id(1).as_text();
    let taken = s.run("snapshot.take", &json!({ "id": i, "name": "First draft" })).unwrap()["taken"].as_str().unwrap().to_owned();
    run(&mut s, "text.set", json!({ "id": i, "text": "Rewritten.\n" }));
    run(&mut s, "snapshot.restore", json!({ "id": i, "taken": taken }));
    assert_eq!(s.project.texts[&id(1)].body, Text::from_paragraphs(&["The keeper came."]));
    let list = s.run("snapshot.list", &json!({ "id": i })).unwrap();
    assert_eq!(list.as_array().unwrap().len(), 2, "restoring kept the rewritten text as a snapshot too");
    assert!(list[1]["name"].as_str().unwrap().starts_with("Before restoring"));
    s.undo();
    assert_eq!(s.project.texts[&id(1)].body, Text::from_paragraphs(&["Rewritten."]));
    run(&mut s, "snapshot.rename", json!({ "id": i, "taken": taken, "name": "Draft before the storm" }));
    assert_eq!(s.project.snapshots[&id(1)][0].name, "Draft before the storm");
    let got = s.run("snapshot.get", &json!({ "id": i, "taken": taken })).unwrap();
    assert_eq!(got["text"], "The keeper came.\n");
    run(&mut s, "snapshot.delete", json!({ "id": i, "taken": taken }));
    assert_eq!(s.run("snapshot.list", &json!({ "id": i })).unwrap().as_array().unwrap().len(), 1);
}

#[test]
fn the_panel_shows_the_selected_item() {
    struct St {
        s: Session,
        v: InspectorView,
        styled: bool,
    }
    let mut s = session();
    s.run("item.setSynopsis", &json!({ "id": id(1).as_text(), "synopsis": "Arrival at the light." })).unwrap();
    let mut h = Harness::builder().with_size(egui::vec2(320.0, 700.0)).build_ui_state(
        |ui, st: &mut St| {
            if !st.styled {
                wordsmith_manuscript_ui::brand::BRAND.apply(ui.ctx(), jcj_shell::theme::Mode::Light);
                st.styled = true;
            }
            let _ = inspector::panel(ui, &mut st.s, &mut st.v, Some(id(1)));
        },
        St { s, v: InspectorView::default(), styled: false },
    );
    h.run();
    assert!(h.query_by_label("Arrival").is_some());
    assert!(h.query_by_label("Synopsis").is_some());
    assert!(h.query_by_label("Take Snapshot").is_some());
}
