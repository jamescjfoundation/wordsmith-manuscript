// SPDX-License-Identifier: MIT OR Apache-2.0
//! Saving through the app: save, auto-save, backups on close, recovery offered on open.

use std::path::{Path, PathBuf};

use jcj_shell::App;
use serde_json::json;
use wordsmith_manuscript_model::{Area, Project};
use wordsmith_manuscript_ui::Manuscript;
use wordsmith_manuscript_ui::session::Session;

fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("session-tests").join(name);
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

#[test]
fn save_reopen_and_back_up_on_close() {
    let base = scratch("close");
    let root = base.join("Novel.wsm");
    let mut s = Session::create(&root, Project::new()).unwrap();
    let out = s.run("binder.new", &json!({ "area": "manuscript", "title": "Opening" })).unwrap();
    let id = out["id"].as_str().unwrap().to_owned();
    s.run("text.set", &json!({ "id": id, "text": "It began at sea.\n" })).unwrap();
    assert!(s.is_dirty());
    let mut app = Manuscript::with_session(s);
    app.autosave().unwrap();
    assert!(!app.is_dirty());
    app.close().unwrap();
    let backups = wordsmith_manuscript_format::pack::backups(&base.join("Backups"), "Novel");
    assert!(!backups.is_empty(), "closing makes a backup");
    let reopened = Session::open(&root.join("project.toml")).unwrap();
    assert_eq!(reopened.project.binder.area(Area::Manuscript)[0].title, "Opening");
    assert_eq!(reopened.project.manuscript_words(), 4);
}

#[test]
fn unsaved_work_is_offered_back_after_a_crash() {
    let root = scratch("crash").join("Novel.wsm");
    let mut s = Session::create(&root, Project::new()).unwrap();
    s.run("binder.new", &json!({ "area": "notes", "title": "Idea" })).unwrap();
    // The app keeps recovery while editing, then "crashes" before saving.
    s.write_recovery().unwrap();
    drop(s);
    let mut app = Manuscript::new();
    app.open(&root).unwrap();
    assert!(app.recovered.is_some(), "the unsaved work is offered");
    assert!(app.session.as_ref().unwrap().project.binder.area(Area::Notes).is_empty());
    app.accept_recovered();
    assert_eq!(app.session.as_ref().unwrap().project.binder.area(Area::Notes)[0].title, "Idea");
    // Undo puts the saved version back.
    app.session.as_mut().unwrap().undo();
    assert!(app.session.as_ref().unwrap().project.binder.area(Area::Notes).is_empty());
}

#[test]
fn a_packed_copy_opens_as_a_new_folder() {
    let base = scratch("packed");
    let root = base.join("Book.wsm");
    let mut s = Session::create(&root, Project::new()).unwrap();
    s.run("binder.new", &json!({ "area": "manuscript", "title": "One" })).unwrap();
    s.save().unwrap();
    let packed = base.join("Book copy.wsmz");
    s.save_packed(&packed).unwrap();
    let mut app = Manuscript::new();
    app.open(&packed).unwrap();
    assert_eq!(app.session.as_ref().unwrap().project, s.project);
    assert!(base.join("Book copy.wsm").join("project.toml").exists());
}
