// SPDX-License-Identifier: MIT OR Apache-2.0
//! Revision passes (PRG-4): text typed in revision mode carries the pass, out of it does not;
//! the marks are saved, read back, cleared without touching words, and never exported.

use jcj_shell::{App, Cmd, Shell};
use serde_json::json;
use wordsmith_manuscript_format::dialect;
use wordsmith_manuscript_model::{Area, Item, ItemId, ItemText, Kind, Place, Project, Role, Text};
use wordsmith_manuscript_ui::Manuscript;
use wordsmith_manuscript_ui::session::Session;

fn id(n: u64) -> ItemId {
    ItemId::from_counter(n)
}

fn app() -> (Manuscript, Shell) {
    let mut p = Project::new();
    p.binder.insert(Place::end_of(Area::Manuscript), Item::with_id(id(1), Kind::Text, "Scene").role(Role::Scene)).unwrap();
    p.texts.insert(id(1), ItemText::with_body(Text::from_paragraphs(&["The keeper slept."])));
    (Manuscript::with_session(Session::new(p)), Shell::new("Test", "0", wordsmith_manuscript_ui::brand::tokens()))
}

fn body(app: &Manuscript) -> String {
    dialect::write(&app.session.as_ref().unwrap().project.text(id(1)).unwrap().body)
}

#[test]
fn typing_in_a_pass_is_marked_and_kept() {
    let (mut app, mut sh) = app();
    app.open_item(id(1));
    let out = app.session.as_mut().unwrap().run("revision.start", &json!({ "name": "Tighten" })).unwrap();
    assert_eq!(out["pass"], 0);
    app.mark_revisions();
    let ed = app.editor.as_mut().unwrap();
    ed.set_caret(0, 11, false);
    ed.type_text("and the lamp ").unwrap();
    app.write_back();
    assert_eq!(body(&app), "The keeper [and the lamp ]{rev=0}slept.\n");
    // Out of revision mode, typing next to revised text is not revised.
    app.run(&Cmd::new("revision.mode"), &mut sh).unwrap();
    let ed = app.editor.as_mut().unwrap();
    ed.set_caret(0, 24, false);
    ed.type_text("both ").unwrap();
    app.write_back();
    assert_eq!(body(&app), "The keeper [and the lamp ]{rev=0}both slept.\n");
    let s = app.session.as_mut().unwrap();
    let list = s.run("revision.list", &json!({})).unwrap();
    assert_eq!(list["passes"][0]["name"], "Tighten");
    assert_eq!(list["passes"][0]["items"], 1);
    assert_eq!(list["mode"], false);
    // Saved and read back; exports and the book view leave the marks out.
    let dir = std::env::temp_dir().join(format!("wsm-revisions-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    s.save_as(&dir).unwrap();
    let back = Session::open(&dir).unwrap();
    assert_eq!(back.project.text(id(1)), s.project.text(id(1)));
    assert!(std::fs::read_to_string(dir.join("project.toml")).unwrap().starts_with("schema = 3"));
    let md = wordsmith_manuscript_io::export::export(&s.project, &Default::default(), wordsmith_manuscript_io::export::Target::Markdown).unwrap();
    assert!(!String::from_utf8_lossy(&md.bytes).contains("rev="));
    let book = wordsmith_manuscript_format::book::from_project(&s.project);
    assert!(wordsmith_manuscript_format::book::words(&book).contains(&"lamp".to_owned()));
    assert!(format!("{:?}", book.body).contains("rev: None") && !format!("{:?}", book.body).contains("rev: Some"));
    // Clearing keeps every word.
    s.run("revision.clear", &json!({})).unwrap();
    assert_eq!(dialect::write(&s.project.text(id(1)).unwrap().body), "The keeper and the lamp both slept.\n");
    s.undo();
    assert!(dialect::write(&s.project.text(id(1)).unwrap().body).contains("{rev=0}"));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn passes_follow_the_cycle_and_mode_needs_a_pass() {
    let mut s = Session::new(Project::new());
    assert!(s.run("revision.mode", &json!({ "on": true })).is_err());
    let a = s.run("revision.start", &json!({})).unwrap();
    let b = s.run("revision.start", &json!({})).unwrap();
    assert_ne!(a["name"], b["name"]);
    assert_eq!(b["pass"], 1);
    assert!(s.run("revision.start", &json!({ "name": a["name"] })).is_err());
    assert!(s.run("revision.clear", &json!({ "pass": 7 })).is_err());
}
