// SPDX-License-Identifier: MIT OR Apache-2.0
//! The story bible: entries in their type's folder, fields from templates, names kept in step
//! with the binder, bodies edited in the editor, backlinks.

use serde_json::{Value, json};
use wordsmith_manuscript_model::{Area, Item, ItemId, ItemText, Kind, Place, Project, Text};
use wordsmith_manuscript_ui::Manuscript;
use wordsmith_manuscript_ui::session::Session;

fn new(s: &mut Session, kind: &str, name: &str) -> ItemId {
    let out = s.run("bible.new", &json!({ "kind": kind, "name": name })).unwrap();
    out["id"].as_str().unwrap().parse().unwrap()
}

#[test]
fn entries_go_in_their_types_folder() {
    let p = wordsmith_manuscript_model::templates::build("novel", "Book", &mut ItemId::new).unwrap();
    let mut s = Session::new(p);
    let mara = new(&mut s, "character", "Mara");
    let folder = s.project.binder.place_of(mara).unwrap().parent.unwrap();
    assert_eq!(s.project.binder.get(folder).unwrap().title, "Characters");
    // A type without a folder gets one.
    let mut s = Session::new(Project::new());
    let mill = new(&mut s, "place", "The Mill");
    let folder = s.project.binder.place_of(mill).unwrap().parent.unwrap();
    assert_eq!(s.project.binder.get(folder).unwrap().title, "Places");
    assert_eq!(s.project.binder.area_of(mill), Some(Area::Bible));
    assert!(s.run("bible.new", &json!({ "kind": "spaceship", "name": "X" })).is_err());
    s.undo();
    assert!(s.project.binder.area(Area::Bible).is_empty() && s.project.entries.is_empty());
}

#[test]
fn fields_follow_the_template_and_names_stay_in_step() {
    let mut s = Session::new(Project::new());
    let id = new(&mut s, "character", "Mara");
    let i = id.as_text();
    s.run("entry.setField", &json!({ "id": i, "field": "Age", "value": "34" })).unwrap();
    s.run("entry.setField", &json!({ "id": i, "field": "Eyes", "value": "Grey" })).unwrap();
    let got = s.run("entry.get", &json!({ "id": i })).unwrap();
    let names: Vec<&str> = got["fields"].as_array().unwrap().iter().map(|f| f["name"].as_str().unwrap()).collect();
    assert_eq!(names[0], "Role in story");
    assert_eq!(names.last(), Some(&"Eyes"), "fields outside the template come after it");
    s.run("entry.set", &json!({ "id": i, "name": "Mara Vell", "aliases": ["the keeper", " "] })).unwrap();
    assert_eq!(s.project.binder.get(id).unwrap().title, "Mara Vell");
    assert_eq!(s.project.entries[&id].aliases, ["the keeper"]);
    s.run("binder.rename", &json!({ "id": i, "title": "Mara" })).unwrap();
    assert_eq!(s.project.entries[&id].name, "Mara");
    s.undo();
    assert_eq!(s.project.entries[&id].name, "Mara Vell");
    let templates: Value = s.run("bible.templates", &json!({})).unwrap();
    assert_eq!(templates.as_array().unwrap().len(), 6);
}

#[test]
fn an_entrys_body_is_written_in_the_editor() {
    let mut s = Session::new(Project::new());
    let id = new(&mut s, "place", "The Mill");
    let mut app = Manuscript::with_session(s);
    app.open_item(id);
    let ed = app.editor.as_mut().unwrap();
    ed.type_text("A").unwrap();
    for c in " ruined mill by the river.".chars() {
        ed.type_text(&c.to_string()).unwrap();
    }
    app.write_back();
    let s = app.session.as_ref().unwrap();
    assert_eq!(s.project.entries[&id].body, Text::from_paragraphs(&["A ruined mill by the river."]));
}

#[test]
fn backlinks_by_link_and_by_mention() {
    let mut s = Session::new(Project::new());
    let mara = new(&mut s, "character", "Mara");
    s.run("entry.set", &json!({ "id": mara.as_text(), "aliases": ["the keeper"] })).unwrap();
    for (n, text) in [(1, format!("[She](wsm:{mara}) climbed.\n")), (2, "The keeper slept.\n".to_owned()), (3, "Nobody.\n".to_owned())] {
        let id = ItemId::from_counter(n);
        s.project.binder.insert(Place::end_of(Area::Manuscript), Item::with_id(id, Kind::Text, format!("Scene {n}"))).unwrap();
        s.project.texts.insert(id, ItemText::with_body(wordsmith_manuscript_format::dialect::parse(&text)));
    }
    let b = s.run("bible.backlinks", &json!({ "id": mara.as_text() })).unwrap();
    assert_eq!(b["linked"][0]["title"], "Scene 1");
    assert_eq!(b["mentioned"][0]["title"], "Scene 2");
    assert_eq!(b["mentioned"].as_array().unwrap().len(), 1);
}

#[test]
fn the_word_list_follows_entry_names_and_aliases() {
    let mut s = Session::new(Project::new());
    let id = new(&mut s, "character", "Quellistra");
    let words = wordsmith_manuscript_ui::spelling::project_words(&s.project);
    assert!(words.contains("Quellistra"));
    s.run("entry.set", &json!({ "id": id.as_text(), "aliases": ["Quell-the-Bright"] })).unwrap();
    let words = wordsmith_manuscript_ui::spelling::project_words(&s.project);
    assert!(words.contains("Quell") && words.contains("Bright"));
    s.run("entry.set", &json!({ "id": id.as_text(), "name": "Ysolde" })).unwrap();
    let words = wordsmith_manuscript_ui::spelling::project_words(&s.project);
    assert!(words.contains("Ysolde") && !words.contains("Quellistra"));
    let text = "Ysolde met Quell.";
    assert!(wordsmith_manuscript_ui::spelling::marks(text, "en", &words).is_empty());
}

#[test]
fn a_link_to_a_deleted_entry_shows_as_broken_and_stays() {
    let mut s = Session::new(Project::new());
    let id = new(&mut s, "place", "Harbor");
    let scene = ItemId::from_counter(50);
    s.project.binder.insert(Place::end_of(Area::Manuscript), Item::with_id(scene, Kind::Text, "Scene")).unwrap();
    s.project.texts.insert(scene, ItemText::with_body(wordsmith_manuscript_format::dialect::parse(&format!("At the [harbor](wsm:{id}).\n"))));
    s.run("binder.trash", &json!({ "id": id.as_text() })).unwrap();
    s.run("binder.emptyTrash", &json!({ "confirm": true })).unwrap();
    // The link is never removed silently.
    assert!(wordsmith_manuscript_format::dialect::write(&s.project.texts[&scene].body).contains(&format!("wsm:{id}")));
    let app = Manuscript::with_session(s);
    assert_eq!(app.link_target(&format!("wsm:{id}")).map(|x| x.0), Some("A deleted item".into()));
}

#[test]
fn a_notes_folder_of_200_entries_imports_with_every_link_resolved_or_reported() {
    let folder = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/notes-folder");
    let p = wordsmith_manuscript_model::templates::build("novel", "Town", &mut ItemId::new).unwrap();
    let mut s = Session::new(p);
    let out = s.run("import.notes", &json!({ "path": folder.to_string_lossy() })).unwrap();
    assert_eq!(out["notes"], 200);
    assert_eq!(out["word_check"], "every word arrived");
    let unresolved = out["links_unresolved"].as_array().unwrap();
    assert_eq!(unresolved.len(), 3, "{out}");
    assert!(unresolved.iter().all(|u| u["name"].as_str().unwrap().starts_with("Nobody Called")));
    // 2 links in each note plus the 3 unresolved ones: every other link found its entry.
    assert_eq!(out["links_resolved"], 400);
    assert_eq!(s.project.entries.len(), 200);
    // Notes landed in the bible's type folders (the novel template's, not new ones).
    let characters = s.project.binder.area(Area::Bible).iter().find(|i| i.title == "Characters").unwrap();
    assert_eq!(characters.children.len(), 120);
    assert_eq!(s.project.binder.area(Area::Bible).iter().filter(|i| i.title == "Characters").count(), 1);
    let any = s.project.entries.values().find(|e| e.kind == "character" && e.fields.contains_key("age")).unwrap();
    assert!(!any.body.is_empty());
    // One undo step takes it all back.
    s.undo();
    assert!(s.project.entries.is_empty());
}
