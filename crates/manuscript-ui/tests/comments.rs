// SPDX-License-Identifier: MIT OR Apache-2.0
//! Comments (ED-7): added on the selection, listed and edited in the inspector's commands,
//! removed without touching the words, saved and read back.

use jcj_shell::{App, Cmd, Shell};
use serde_json::json;
use wordsmith_manuscript_model::{Area, Item, ItemId, ItemText, Kind, Place, Project, Role, Text};
use wordsmith_manuscript_ui::Manuscript;
use wordsmith_manuscript_ui::session::Session;

fn id(n: u64) -> ItemId {
    ItemId::from_counter(n)
}

fn app() -> (Manuscript, Shell) {
    let mut p = Project::new();
    let mut ch = Item::with_id(id(100), Kind::Folder, "Chapter").role(Role::Chapter);
    for n in 1..=2 {
        ch.children.push(Item::with_id(id(n), Kind::Text, format!("Scene {n}")).role(Role::Scene));
        p.texts.insert(id(n), ItemText::with_body(Text::from_paragraphs(&[&format!("Scene {n} has a lamp.")])));
    }
    p.binder.insert(Place::end_of(Area::Manuscript), ch).unwrap();
    (Manuscript::with_session(Session::new(p)), Shell::new("Test", "0", wordsmith_manuscript_ui::brand::BRAND.tokens()))
}

#[test]
fn a_comment_on_the_selection_in_a_stitched_view_lands_in_its_item() {
    let (mut app, mut sh) = app();
    app.run(&Cmd::with("view.stitch", json!({ "id": id(100).as_text() })), &mut sh).unwrap();
    let ed = app.editor.as_mut().unwrap();
    let i = (0..ed.paragraphs().len()).find(|i| ed.paragraph(*i).is_some_and(|p| p.text == "Scene 2 has a lamp.")).unwrap();
    ed.set_caret(i, 14, false);
    ed.set_caret(i, 18, true);
    app.run(&Cmd::new("write.comment"), &mut sh).unwrap();
    assert_eq!(app.binder.selected, Some(id(2)));
    let s = app.session.as_mut().unwrap();
    s.run("comment.set", &json!({ "id": id(2).as_text(), "comment": "c1", "text": "Which lamp?" })).unwrap();
    let list = s.run("comment.list", &json!({ "id": id(2).as_text() })).unwrap();
    assert_eq!(list["comments"][0]["words"], "lamp");
    assert_eq!(list["comments"][0]["text"], "Which lamp?");
    assert!(s.run("comment.list", &json!({ "id": id(1).as_text() })).unwrap()["comments"].as_array().unwrap().is_empty());
    // Removing it leaves the words exactly.
    s.run("comment.remove", &json!({ "id": id(2).as_text(), "comment": "c1" })).unwrap();
    assert_eq!(s.project.text(id(2)).unwrap().body, Text::from_paragraphs(&["Scene 2 has a lamp."]));
    assert!(s.project.text(id(2)).unwrap().meta.comments.is_empty());
}

#[test]
fn comments_by_quote_save_and_read_back_and_undo() {
    let (mut app, _) = app();
    let s = app.session.as_mut().unwrap();
    let out = s.run("comment.add", &json!({ "id": id(1).as_text(), "quote": "a lamp", "text": "Earlier it was a candle.", "by": "Me" })).unwrap();
    assert_eq!(out["comment"], "c1");
    assert!(s.run("comment.add", &json!({ "id": id(1).as_text(), "quote": "a torch", "text": "x" })).is_err());
    let dir = std::env::temp_dir().join(format!("wsm-comments-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    s.save_as(&dir).unwrap();
    let (_, back) = wordsmith_manuscript_format::folder::Folder::open(&dir).unwrap();
    let t = back.text(id(1)).unwrap();
    assert_eq!(t.meta.comments["c1"].text, "Earlier it was a candle.");
    assert_eq!(t.meta.comments["c1"].by, "Me");
    assert_eq!(wordsmith_manuscript_ui::comments::anchored(&t.body), vec![("c1".to_owned(), "a lamp".to_owned())]);
    s.undo();
    assert!(s.project.text(id(1)).unwrap().meta.comments.is_empty());
    assert_eq!(s.project.text(id(1)).unwrap().body, Text::from_paragraphs(&["Scene 1 has a lamp."]));
    let _ = std::fs::remove_dir_all(&dir);
}
