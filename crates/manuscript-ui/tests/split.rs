// SPDX-License-Identifier: MIT OR Apache-2.0
//! The split editor (ED-6): two panes, each written back; an item is editable in one pane only.

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
        p.texts.insert(id(n), ItemText::with_body(Text::from_paragraphs(&[&format!("Scene {n} words.")])));
    }
    p.binder.insert(Place::end_of(Area::Manuscript), ch).unwrap();
    p.binder.insert(Place::end_of(Area::Research), Item::with_id(id(50), Kind::Text, "Notes")).unwrap();
    p.texts.insert(id(50), ItemText::with_body(Text::from_paragraphs(&["Research."])));
    (Manuscript::with_session(Session::new(p)), Shell::new("Test", "0", wordsmith_manuscript_ui::brand::tokens()))
}

fn body(app: &Manuscript, n: u64) -> Text {
    app.session.as_ref().unwrap().project.text(id(n)).unwrap().body.clone()
}

#[test]
fn two_panes_each_write_back() {
    let (mut app, mut sh) = app();
    app.open_item(id(1));
    app.run(&Cmd::new("view.split"), &mut sh).unwrap();
    assert_eq!(app.split, jcj_shell::split::Split::Side);
    app.editor.as_mut().unwrap().type_text("A ").unwrap();
    app.run(&Cmd::new("view.otherPane"), &mut sh).unwrap();
    assert_eq!(app.active_pane, 1);
    app.open_item(id(50));
    app.editor.as_mut().unwrap().type_text("B ").unwrap();
    app.write_back();
    assert_eq!(body(&app, 1), Text::from_paragraphs(&["A Scene 1 words."]));
    assert_eq!(body(&app, 50), Text::from_paragraphs(&["B Research."]));
    app.run(&Cmd::new("view.unsplit"), &mut sh).unwrap();
    assert!(app.other.is_none());
    assert_eq!(app.editor.as_ref().unwrap().item, id(50), "the active pane stays");
}

#[test]
fn an_item_in_a_stitched_view_opens_read_only_in_the_other_pane() {
    let (mut app, mut sh) = app();
    app.run(&Cmd::with("view.stitch", json!({ "id": id(100).as_text() })), &mut sh).unwrap();
    app.run(&Cmd::new("view.splitStacked"), &mut sh).unwrap();
    app.run(&Cmd::new("view.otherPane"), &mut sh).unwrap();
    app.open_item(id(2));
    assert!(app.editor.as_ref().unwrap().read_only, "the stitched view holds it");
    assert!(app.editor.as_mut().unwrap().type_text("lost").is_err() || body(&app, 2) == Text::from_paragraphs(&["Scene 2 words."]));
    // Edit it in the stitched view; the read-only pane shows the edit when it is next active.
    app.run(&Cmd::new("view.otherPane"), &mut sh).unwrap();
    assert!(!app.editor.as_ref().unwrap().read_only);
    let ed = app.editor.as_mut().unwrap();
    let i = (0..ed.paragraphs().len()).find(|i| ed.paragraph(*i).is_some_and(|p| p.text == "Scene 2 words.")).unwrap();
    ed.set_caret(i, 0, false);
    ed.type_text("New ").unwrap();
    app.run(&Cmd::new("view.otherPane"), &mut sh).unwrap();
    let ed = app.editor.as_ref().unwrap();
    assert!(ed.read_only);
    assert_eq!(ed.paragraph(0).unwrap().text, "New Scene 2 words.");
    // Back to one pane, this one: the item is editable again.
    app.run(&Cmd::new("view.unsplit"), &mut sh).unwrap();
    assert!(!app.editor.as_ref().unwrap().read_only);
    assert_eq!(body(&app, 2), Text::from_paragraphs(&["New Scene 2 words."]));
}

#[test]
fn the_same_item_is_editable_in_one_pane_only() {
    let (mut app, mut sh) = app();
    app.open_item(id(1));
    app.run(&Cmd::new("view.split"), &mut sh).unwrap();
    app.run(&Cmd::new("view.otherPane"), &mut sh).unwrap();
    app.open_item(id(1));
    assert!(app.editor.as_ref().unwrap().read_only);
}
