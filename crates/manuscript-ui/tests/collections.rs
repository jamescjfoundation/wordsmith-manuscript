// SPDX-License-Identifier: MIT OR Apache-2.0
//! Collections (SRC-2): hand-picked lists and saved searches that follow the text, saved with
//! the project; items never move in the binder.

use serde_json::{Value, json};
use wordsmith_manuscript_model::{Area, Item, ItemId, ItemText, Kind, Place, Project, Text};
use wordsmith_manuscript_ui::session::Session;

fn id(n: u64) -> ItemId {
    ItemId::from_counter(n)
}

fn session() -> Session {
    let mut p = Project::new();
    for (n, words) in [(1, "The lamp was lit."), (2, "Rain fell all night."), (3, "The lamp went out.")] {
        p.binder.insert(Place::end_of(Area::Manuscript), Item::with_id(id(n), Kind::Text, format!("Scene {n}"))).unwrap();
        p.texts.insert(id(n), ItemText::with_body(Text::from_paragraphs(&[words])));
    }
    Session::new(p)
}

fn titles(v: &Value) -> Vec<String> {
    v.as_array().unwrap().iter().map(|x| x["title"].as_str().unwrap().to_owned()).collect()
}

#[test]
fn hand_picked_and_saved_searches() {
    let mut s = session();
    let binder_before = s.project.binder.clone();
    s.run("collection.new", &json!({ "name": "To revise", "items": [id(3).as_text(), id(1).as_text()] })).unwrap();
    s.run("collection.add", &json!({ "name": "To revise", "items": [id(2).as_text(), id(3).as_text()] })).unwrap();
    assert_eq!(
        titles(&s.run("collection.items", &json!({ "name": "To revise" })).unwrap()),
        ["Scene 3", "Scene 1", "Scene 2"],
        "in the order picked"
    );
    s.run("collection.remove", &json!({ "name": "To revise", "items": [id(1).as_text()] })).unwrap();
    assert!(s.run("collection.new", &json!({ "name": "To revise" })).is_err(), "names are unique");
    // A saved search follows the text.
    s.run("collection.saveSearch", &json!({ "name": "Lamp", "query": "lamp" })).unwrap();
    assert_eq!(titles(&s.run("collection.items", &json!({ "name": "Lamp" })).unwrap()), ["Scene 1", "Scene 3"]);
    s.run("text.set", &json!({ "id": id(2).as_text(), "text": "A lamp in the rain.\n" })).unwrap();
    assert_eq!(titles(&s.run("collection.items", &json!({ "name": "Lamp" })).unwrap()), ["Scene 1", "Scene 2", "Scene 3"]);
    assert!(s.run("collection.add", &json!({ "name": "Lamp", "items": [id(1).as_text()] })).is_err(), "a search holds what matches");
    assert!(s.run("collection.saveSearch", &json!({ "name": "Bad", "query": "(", "regex": true })).is_err());
    // Trashed items leave the lists; the binder never changes.
    s.run("binder.trash", &json!({ "id": id(3).as_text() })).unwrap();
    assert_eq!(titles(&s.run("collection.items", &json!({ "name": "To revise" })).unwrap()), ["Scene 2"]);
    s.undo();
    assert_eq!(s.project.binder, binder_before);
    let list = s.run("collection.list", &json!({})).unwrap();
    assert_eq!(list[0]["kind"], "list");
    assert_eq!(list[1]["kind"], "search");
    assert_eq!(list[1]["count"], 3);
    s.run("collection.rename", &json!({ "name": "Lamp", "to": "Lamps" })).unwrap();
    // Saved and read back.
    let dir = std::env::temp_dir().join(format!("wsm-collections-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    s.save_as(&dir).unwrap();
    let (_, back) = wordsmith_manuscript_format::folder::Folder::open(&dir).unwrap();
    assert_eq!(wordsmith_manuscript_ui::collections::all(&back), wordsmith_manuscript_ui::collections::all(&s.project));
    let _ = std::fs::remove_dir_all(&dir);
    s.run("collection.delete", &json!({ "name": "Lamps" })).unwrap();
    assert_eq!(s.run("collection.list", &json!({})).unwrap().as_array().unwrap().len(), 1);
}
