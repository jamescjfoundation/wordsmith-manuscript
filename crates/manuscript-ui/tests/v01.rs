// SPDX-License-Identifier: MIT OR Apache-2.0
//! The test of v0.1 (plan section 10): a public-domain novel of over 150,000 words, split across
//! six DOCX files named as a document set, imports as a structured project with every word
//! verified; it is reorganized on the corkboard and in the outline, edited in a stitched view of
//! the whole manuscript, exported to standard manuscript DOCX and PDF and opened through the book
//! view, with every word still matching.

use std::collections::HashSet;
use std::path::Path;

use jcj_shell::{App, Cmd, Shell};
use serde_json::json;
use wordsmith_manuscript_format::book;
use wordsmith_manuscript_io::verify;
use wordsmith_manuscript_model::{Area, Project, Role};
use wordsmith_manuscript_ui::session::Session;
use wordsmith_manuscript_ui::{Manuscript, cork, outline};

fn fixture(p: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures").join(p)
}

fn sorted(mut v: Vec<String>) -> Vec<String> {
    v.sort();
    v
}

/// The project's manuscript words in reading order (part and chapter titles, then text).
fn manuscript_words(p: &Project) -> Vec<String> {
    let mut out = Vec::new();
    for id in p.included_manuscript() {
        let Some(item) = p.binder.get(id) else { continue };
        if matches!(item.role, Role::Part | Role::Chapter) {
            out.extend(verify::words(&item.title));
        }
        if let Some(t) = p.texts.get(&id) {
            let body = wordsmith_manuscript_model::Text { blocks: t.body.blocks.clone(), notes: vec![] };
            out.extend(body.plain_paragraphs().iter().flat_map(|x| verify::words(x)));
        }
    }
    out
}

#[test]
fn the_test_of_v01() {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("v01");
    let _ = std::fs::remove_dir_all(&dir);
    let root = dir.join("Great Expectations.wsm");
    let mut p = Project::new();
    p.settings.book.title = "Great Expectations".into();
    p.settings.book.author = "Charles Dickens".into();
    let mut s = Session::create(&root, p).unwrap();

    // 1. Import the six-file set, every word verified.
    let set = fixture("document-set");
    let files = wordsmith_manuscript_io::docset::folder_files(&set).unwrap();
    assert_eq!(files.len(), 6);
    let out = s.run("import.set", &json!({ "folder": set.to_string_lossy() })).unwrap();
    assert!(out["checks"][0]["check"].as_str().unwrap().starts_with("every word matches"), "{out}");
    let source: Vec<String> = sorted(manuscript_words(&s.project));
    assert!(source.len() > 150_000);
    let chapters: Vec<_> = s.project.binder.area(Area::Manuscript).iter().filter(|i| i.role == Role::Chapter).map(|i| i.id).collect();
    assert_eq!(chapters.len(), 59, "a structured project");

    // 2. Reorganize on the corkboard (a card dragged) and in the outline (an order applied).
    let none = HashSet::new();
    let cards = cork::items(&s.project, None, &none);
    let last = *cards.last().unwrap();
    s.run("binder.move", &json!({ "id": last.as_text(), "area": "manuscript", "index": 1 })).unwrap();
    assert_eq!(cork::items(&s.project, None, &none)[1], last);
    assert_eq!(
        outline::items(&s.project, None, &none).iter().filter(|(_, d)| *d == 0).nth(1).map(|x| x.0),
        Some(last),
        "the outline shows it at once"
    );
    s.run("outline.applySort", &json!({ "column": 3 })).unwrap();
    s.save().unwrap();

    // 3. Edit in a stitched view of the whole manuscript.
    let mut app = Manuscript::with_session(s);
    let mut shell = Shell::new("Test", "0", wordsmith_manuscript_ui::brand::tokens());
    app.run(&Cmd::new("view.stitchManuscript"), &mut shell).unwrap();
    {
        let ed = app.editor.as_mut().unwrap();
        assert!(ed.counts().0 > 150_000);
        let mid = ed.paragraphs().len() / 2;
        let i = (mid..ed.paragraphs().len())
            .find(|i| ed.paragraph(*i).is_some_and(|p| !p.text.is_empty() && !wordsmith_manuscript_ui::stitch::is_divider(p)))
            .unwrap();
        ed.set_caret(i, 0, false);
        for c in "Added in the stitched view. ".chars() {
            ed.type_text(&c.to_string()).unwrap();
        }
    }
    app.autosave().unwrap();
    let s = app.session.as_mut().unwrap();
    let mut expected = source.clone();
    expected.extend(verify::words("Added in the stitched view."));
    let expected = sorted(expected);
    assert_eq!(sorted(manuscript_words(&s.project)), expected, "every word kept through the stitched view");

    // 4. Export to standard manuscript DOCX and PDF, each checked word for word.
    for (name, format) in [("ge.docx", "manuscript-docx"), ("ge.pdf", "manuscript-pdf")] {
        let r = s.run("export.file", &json!({ "path": dir.join(name).to_string_lossy(), "format": format })).unwrap();
        assert!(r["check"].as_str().unwrap().starts_with("every word matches"), "{name}: {r}");
    }

    // 5. Open through the book view, from the saved folder: every word, role and id.
    let b = book::open_folder(&root).unwrap();
    assert_eq!(book::words(&b), manuscript_words(&s.project));
    assert_eq!(sorted(book::words(&b)), expected);
    let ids: Vec<_> = s.project.binder.rows_of(Area::Manuscript).iter().map(|r| (r.item.id, r.item.role)).collect();
    assert_eq!(book::outline(&b), ids);
}
