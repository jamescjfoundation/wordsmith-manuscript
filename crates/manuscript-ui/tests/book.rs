// SPDX-License-Identifier: MIT OR Apache-2.0
//! The book view: the v0.1 test project read through the book view only reproduces every word,
//! role and id; the book package reads back the same.

use std::path::Path;

use serde_json::json;
use wordsmith_manuscript_format::{book, pack};
use wordsmith_manuscript_model::{Area, Project, Role};
use wordsmith_manuscript_ui::session::Session;

fn v01_project(dir: &Path) -> Session {
    let _ = std::fs::remove_dir_all(dir);
    let set = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/document-set");
    let mut p = Project::new();
    p.settings.book.title = "Great Expectations".into();
    p.settings.book.author = "Charles Dickens".into();
    let mut s = Session::create(&dir.join("Great Expectations.wsm"), p).unwrap();
    s.run("import.set", &json!({ "folder": set.to_string_lossy() })).unwrap();
    s.save().unwrap();
    s
}

#[test]
fn the_v01_project_through_the_book_view_keeps_every_word_role_and_id() {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("book-view");
    let s = v01_project(&dir);
    let root = s.root().unwrap().to_path_buf();
    // Through the book view only: the folder, and the packed file.
    let from_folder = book::open_folder(&root).unwrap();
    let packed = pack::pack_folder(&root).unwrap();
    let from_packed = book::open_packed(&packed).unwrap();
    assert_eq!(from_folder, from_packed);
    assert_eq!(from_folder.version, book::VERSION);
    // Every word, in order, against the project's manuscript.
    let mut project_words = Vec::new();
    for r in s.project.binder.rows_of(Area::Manuscript) {
        if matches!(r.item.role, Role::Part | Role::Chapter) {
            project_words.extend(wordsmith_manuscript_model::words::words(&r.item.title));
        }
        if let Some(t) = s.project.texts.get(&r.item.id) {
            let body = wordsmith_manuscript_model::Text { blocks: t.body.blocks.clone(), notes: vec![] };
            project_words.extend(body.words());
        }
    }
    assert_eq!(book::words(&from_packed), project_words);
    assert!(project_words.len() > 180_000);
    // Every role and id.
    let ids: Vec<_> = s.project.binder.rows_of(Area::Manuscript).iter().map(|r| (r.item.id, r.item.role)).collect();
    assert_eq!(book::outline(&from_packed), ids);
    // Chapters are numbered in order.
    let numbers: Vec<u32> = from_packed.body.iter().filter(|x| x.role == Role::Chapter).filter_map(|x| x.number).collect();
    assert_eq!(numbers, (1..=59).collect::<Vec<u32>>());
    assert_eq!(from_packed.front.len(), 1);
    assert_eq!(from_packed.metadata.byline, "Charles Dickens");
}

#[test]
fn the_book_package_export_reads_back_and_lists_missing_metadata() {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("book-package");
    let mut s = v01_project(&dir);
    let out = s.run("export.file", &json!({ "path": dir.join("For Book").to_string_lossy(), "format": "book" })).unwrap();
    assert!(out["check"].as_str().unwrap().starts_with("every word matches"));
    assert!(dir.join("For Book.wsmz").exists());
    let missing: Vec<&str> = out["missing_metadata"].as_array().unwrap().iter().filter_map(|v| v.as_str()).collect();
    assert!(missing.contains(&"copyright year") && !missing.contains(&"title"));
}
