// SPDX-License-Identifier: MIT OR Apache-2.0
//! Importing documents into a project: preview without change, import as one undo step, every
//! word of the source in the project afterwards.

use std::path::{Path, PathBuf};

use serde_json::json;
use wordsmith_manuscript_io::verify;
use wordsmith_manuscript_model::{Area, Project, Role};
use wordsmith_manuscript_ui::session::Session;

fn fixture(p: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures").join(p)
}

/// The project's manuscript words in reading order: titles of items that came from headings, then
/// texts.
fn manuscript_words(p: &Project) -> Vec<String> {
    let mut out = Vec::new();
    for r in p.binder.rows_of(Area::Manuscript) {
        if matches!(r.item.role, Role::Part | Role::Chapter) {
            out.extend(verify::words(&r.item.title));
        }
        if let Some(t) = p.texts.get(&r.item.id) {
            let body = wordsmith_manuscript_model::Text { blocks: t.body.blocks.clone(), notes: vec![] };
            out.extend(body.plain_paragraphs().iter().flat_map(|x| verify::words(x)));
        }
    }
    out
}

#[test]
fn preview_changes_nothing_and_import_is_one_undo_step() {
    let mut s = Session::new(Project::new());
    let path = fixture("documents/pride-and-prejudice.docx");
    let before = s.project.clone();
    let preview = s.run("import.preview", &json!({ "path": path.to_string_lossy() })).unwrap();
    assert_eq!(s.project, before);
    assert!(preview["word_check"].as_str().unwrap().starts_with("every word matches"));
    let out = s.run("import.document", &json!({ "path": path.to_string_lossy() })).unwrap();
    assert!(out["made"].as_u64().unwrap() > 60);
    // Every word of the source is in the project, in order.
    let doc = wordsmith_manuscript_io::import::document(&path, &Default::default()).unwrap();
    let source = wordsmith_manuscript_io::mapping::source_words(&doc, &Default::default());
    assert_eq!(verify::compare(&source, &manuscript_words(&s.project)).map_err(|d| d.to_string()), Ok(source.len()));
    s.undo();
    assert_eq!(s.project, before);
}

#[test]
fn rules_given_once_are_kept_for_the_next_import() {
    let mut s = Session::new(Project::new());
    let path = fixture("documents/pride-and-prejudice.rtf");
    let rules = json!({ "chapter_lines": false, "split_scenes": false, "scene_marks": ["[break]"], "underline_as_italic": true });
    s.run("import.document", &json!({ "path": path.to_string_lossy(), "rules": rules })).unwrap();
    assert_eq!(s.project.settings.extra["import"]["split_scenes"], false);
    let preview = s.run("import.preview", &json!({ "path": path.to_string_lossy() })).unwrap();
    assert!(preview["word_check"].as_str().unwrap().starts_with("every word matches"));
}
