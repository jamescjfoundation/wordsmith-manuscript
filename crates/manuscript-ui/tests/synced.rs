// SPDX-License-Identifier: MIT OR Apache-2.0
//! Safe in synced folders (FIL-4): changes made elsewhere are read in; when both sides changed,
//! both versions are kept; a save never writes over a change it has not seen without a copy.

use serde_json::json;
use wordsmith_manuscript_format::{dialect, files};
use wordsmith_manuscript_model::{Area, Item, ItemId, ItemText, Kind, Place, Project, Text};
use wordsmith_manuscript_ui::session::Session;

fn id(n: u64) -> ItemId {
    ItemId::from_counter(n)
}

fn setup(name: &str) -> (std::path::PathBuf, Session) {
    let root = std::env::temp_dir().join(format!("wsm-synced-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let mut p = Project::new();
    for n in 1..=2 {
        p.binder.insert(Place::end_of(Area::Manuscript), Item::with_id(id(n), Kind::Text, format!("Scene {n}"))).unwrap();
        p.texts.insert(id(n), ItemText::with_body(Text::from_paragraphs(&[&format!("Scene {n} as first written.")])));
    }
    let s = Session::create(&root, p).unwrap();
    (root, s)
}

/// Another device edits a document.
fn edit_elsewhere(root: &std::path::Path, n: u64, words: &str) {
    let t = ItemText::with_body(Text::from_paragraphs(&[words]));
    std::fs::write(root.join(format!("text/{}.md", id(n))), files::write_item(&t).unwrap()).unwrap();
}

fn conflicts(root: &std::path::Path) -> Vec<String> {
    std::fs::read_dir(root.join("conflicts")).map(|d| d.flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect()).unwrap_or_default()
}

#[test]
fn a_change_made_only_elsewhere_is_read_in() {
    let (root, mut s) = setup("read");
    assert!(s.check_outside().unwrap().is_empty());
    edit_elsewhere(&root, 1, "Scene 1 as the laptop changed it.");
    let o = s.check_outside().unwrap();
    assert_eq!(o.reloaded, vec![id(1)]);
    assert_eq!(s.project.text(id(1)).unwrap().body, Text::from_paragraphs(&["Scene 1 as the laptop changed it."]));
    assert!(s.check_outside().unwrap().is_empty(), "taken in once");
    s.save().unwrap();
    assert!(conflicts(&root).is_empty());
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn when_both_sides_changed_both_versions_are_kept() {
    let (root, mut s) = setup("both");
    s.run("text.set", &json!({ "id": id(1).as_text(), "text": "Scene 1 as changed here.\n" })).unwrap();
    edit_elsewhere(&root, 1, "Scene 1 as the laptop changed it.");
    let o = s.check_outside().unwrap();
    assert_eq!(o.conflicts, vec![id(1)]);
    assert!(o.summary().contains("yours kept"));
    assert_eq!(dialect::write(&s.project.text(id(1)).unwrap().body), "Scene 1 as changed here.\n", "this version stays");
    let snap = s.project.snapshots[&id(1)].last().unwrap();
    assert_eq!(snap.name, "Changed elsewhere");
    assert_eq!(snap.text, Text::from_paragraphs(&["Scene 1 as the laptop changed it."]));
    s.save().unwrap();
    let back = Session::open(&root).unwrap();
    assert_eq!(dialect::write(&back.project.text(id(1)).unwrap().body), "Scene 1 as changed here.\n");
    assert_eq!(back.project.snapshots[&id(1)].last().unwrap().text, Text::from_paragraphs(&["Scene 1 as the laptop changed it."]));
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_save_never_writes_over_an_unseen_change_without_a_copy() {
    let (root, mut s) = setup("save");
    s.run("text.set", &json!({ "id": id(2).as_text(), "text": "Scene 2 changed here.\n" })).unwrap();
    edit_elsewhere(&root, 2, "Scene 2 as the phone changed it.");
    s.save().unwrap();
    let copies = conflicts(&root);
    assert_eq!(copies.len(), 1, "{copies:?}");
    let kept = std::fs::read_to_string(root.join("conflicts").join(&copies[0])).unwrap();
    assert!(kept.contains("Scene 2 as the phone changed it."));
    // A packed copy carries the conflicts folder too.
    let packed = root.with_extension("wsmz");
    s.save_packed(&packed).unwrap();
    let files = wordsmith_manuscript_format::pack::unpack(&std::fs::read(&packed).unwrap()).unwrap();
    assert!(files.keys().any(|k| k.starts_with("conflicts/")));
    let _ = std::fs::remove_file(&packed);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn the_binder_changed_elsewhere_reloads_or_is_kept() {
    let (root, mut s) = setup("binder");
    // Nothing unsaved here: the whole project is read again.
    let mut other = Session::open(&root).unwrap();
    other.run("binder.rename", &json!({ "id": id(2).as_text(), "title": "The Storm" })).unwrap();
    other.save().unwrap();
    let o = s.check_outside().unwrap();
    assert!(o.project_reloaded);
    assert_eq!(s.project.binder.get(id(2)).unwrap().title, "The Storm");
    // Unsaved changes here: ours kept, theirs copied.
    s.run("binder.rename", &json!({ "id": id(1).as_text(), "title": "Arrival" })).unwrap();
    other.run("binder.rename", &json!({ "id": id(2).as_text(), "title": "The Gale" })).unwrap();
    other.save().unwrap();
    let o = s.check_outside().unwrap();
    assert!(!o.project_reloaded);
    assert_eq!(o.copies.len(), 1);
    assert!(conflicts(&root)[0].ends_with("binder.toml"));
    s.save().unwrap();
    let back = Session::open(&root).unwrap();
    assert_eq!(back.project.binder.get(id(1)).unwrap().title, "Arrival");
    // A file removed elsewhere is written again.
    std::fs::remove_file(root.join(format!("text/{}.md", id(1)))).unwrap();
    let o = s.check_outside().unwrap();
    assert_eq!(o.restored.len(), 1);
    s.save().unwrap();
    assert!(root.join(format!("text/{}.md", id(1))).exists());
    let _ = std::fs::remove_dir_all(&root);
}
