// SPDX-License-Identifier: MIT OR Apache-2.0
//! Drafts (DRF-1): a new draft keeps the old one, drafts compare scene by scene, any draft can be
//! written again or exported, every step undoes, and a document set's earlier drafts come in as
//! drafts.

use serde_json::json;
use wordsmith_manuscript_io::compile::Options;
use wordsmith_manuscript_io::export::{Target, export};
use wordsmith_manuscript_model::{Area, Item, ItemId, ItemText, Kind, Place, Project, Role, Text, drafts};
use wordsmith_manuscript_ui::session::Session;

fn id(n: u64) -> ItemId {
    ItemId::from_counter(n)
}

fn session() -> Session {
    let mut p = Project::new();
    let ch = Item::with_id(id(10), Kind::Folder, "One")
        .role(Role::Chapter)
        .with_children(vec![Item::with_id(id(1), Kind::Text, "Dock").role(Role::Scene), Item::with_id(id(2), Kind::Text, "Inn").role(Role::Scene)]);
    p.binder.insert(Place::end_of(Area::Manuscript), ch).unwrap();
    p.texts.insert(id(1), ItemText::with_body(Text::from_paragraphs(&["The boat came in."])));
    p.texts.insert(id(2), ItemText::with_body(Text::from_paragraphs(&["The inn was full."])));
    Session::new(p)
}

fn words(p: &Project) -> String {
    let out = export(&drafts::exported(p), &Options::default(), Target::Text).unwrap();
    String::from_utf8(out.bytes).unwrap()
}

#[test]
fn drafts_keep_compare_switch_and_export() {
    let mut s = session();
    let kept = s.run("draft.new", &json!({ "name": "Second Draft" })).unwrap()["kept"].as_str().unwrap().to_owned();
    s.run("text.set", &json!({ "id": id(1).as_text(), "text": "The boat came in late, in the rain.\n" })).unwrap();
    s.run("binder.trash", &json!({ "id": id(2).as_text() })).unwrap();
    let list = s.run("draft.list", &json!({})).unwrap();
    assert_eq!(list["drafts"][0]["name"], "Second Draft");
    assert_eq!(list["drafts"][1]["name"], "First Draft");
    assert_eq!(list["drafts"][1]["words"], 8);
    let cmp = s.run("draft.compare", &json!({ "old": kept })).unwrap();
    let states: Vec<(&str, &str)> =
        cmp["items"].as_array().unwrap().iter().map(|i| (i["title"].as_str().unwrap(), i["state"].as_str().unwrap())).collect();
    assert_eq!(states, [("One", "same"), ("Dock", "changed"), ("Inn", "cut")]);
    assert_eq!(cmp["items"][1]["added"], 4);
    // Export the first draft without switching to it.
    assert!(words(&s.project).contains("in the rain"));
    s.run("draft.setExport", &json!({ "id": kept })).unwrap();
    let first = words(&s.project);
    assert!(first.contains("The boat came in.") && first.contains("The inn was full."), "{first}");
    assert_eq!(
        wordsmith_manuscript_format::book::words(&wordsmith_manuscript_format::book::from_project(&s.project)).len(),
        9,
        "Book sees the exported draft (with its chapter title)"
    );
    // Write the first draft again; undo puts everything back.
    let before = s.project.clone();
    s.run("draft.switch", &json!({ "id": kept })).unwrap();
    assert_eq!(drafts::working_name(&s.project), "First Draft");
    assert_eq!(s.project.text(id(2)).unwrap().body, Text::from_paragraphs(&["The inn was full."]));
    s.undo();
    assert_eq!(s.project, before);
    s.redo();
    assert_eq!(drafts::working_name(&s.project), "First Draft");
    s.run("draft.rename", &json!({ "id": kept, "name": "Rain Draft" })).unwrap();
    assert!(s.run("draft.new", &json!({ "name": "Rain Draft" })).is_err());
    s.run("draft.delete", &json!({ "id": kept })).unwrap();
    assert!(s.project.drafts.is_empty());
    s.undo();
    assert_eq!(s.project.drafts.len(), 1);
}

#[test]
fn a_document_sets_earlier_drafts_come_in_as_drafts() {
    let dir = std::env::temp_dir().join(format!("wsm-draft-set-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    // Written for the test.
    std::fs::write(dir.join("River (First Draft).txt"), "Chapter 1\n\nThe river rose.\n\nChapter 2\n\nThe town slept.\n").unwrap();
    std::fs::write(
        dir.join("River (Second Draft).txt"),
        "Chapter 1\n\nThe river rose in the night.\n\nChapter 2\n\nThe town slept.\n\nChapter 3\n\nMorning.\n",
    )
    .unwrap();
    let mut s = Session::new(Project::new());
    s.run("import.set", &json!({ "folder": dir.to_str() })).unwrap();
    assert_eq!(drafts::working_name(&s.project), "Second Draft");
    let list = s.run("draft.list", &json!({})).unwrap();
    assert_eq!(list["drafts"][1]["name"], "First Draft");
    let kept = list["drafts"][1]["id"].as_str().unwrap().to_owned();
    let cmp = s.run("draft.compare", &json!({ "old": kept })).unwrap();
    let states: Vec<(&str, &str)> =
        cmp["items"].as_array().unwrap().iter().map(|i| (i["title"].as_str().unwrap(), i["state"].as_str().unwrap())).collect();
    assert!(states.contains(&("Chapter 1", "changed")), "{states:?}");
    assert!(states.contains(&("Chapter 2", "same")), "{states:?}");
    assert!(states.contains(&("Chapter 3", "added")), "{states:?}");
    assert!(s.project.binder.area(Area::Research).is_empty(), "earlier drafts no longer go to Research");
    let _ = std::fs::remove_dir_all(&dir);
}
