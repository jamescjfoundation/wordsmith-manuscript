// SPDX-License-Identifier: MIT OR Apache-2.0
//! Export through an external converter (EXP-7): the program the writer names is run directly
//! with the arguments filled in, and the result is checked word for word when it can be read.
//!
//! The stand-in converters are `cp` and `false`, programs every Unix system has, so the whole
//! file is for Unix: on Windows it holds no test, and nothing in it is built there.

#![cfg(unix)]

use serde_json::json;
use wordsmith_manuscript_model::{Area, Item, ItemId, ItemText, Kind, Place, Project, Role, Text};
use wordsmith_manuscript_ui::session::Session;

fn session() -> Session {
    let mut p = Project::new();
    let id = ItemId::from_counter(1);
    p.binder.insert(Place::end_of(Area::Manuscript), Item::with_id(id, Kind::Text, "Scene").role(Role::Scene)).unwrap();
    p.texts.insert(id, ItemText::with_body(Text::from_paragraphs(&["The lamp was lit at dusk."])));
    Session::new(p)
}

#[test]
fn a_converter_is_run_and_its_result_checked() {
    let dir = std::env::temp_dir().join(format!("wsm-convert-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let mut s = session();
    // A stand-in converter: copying the DOCX it is given is a "conversion" to DOCX.
    let out = dir.join("My Book.docx");
    let r = s.run("export.convert", &json!({ "path": out.to_str(), "program": "cp", "args": "{input} {output}" })).unwrap();
    assert!(r["check"].as_str().unwrap().starts_with("every word matches"), "{r}");
    assert!(out.exists());
    // A format Manuscript cannot read back is written but not checked.
    let other = dir.join("My Book.xyz");
    let r = s.run("export.convert", &json!({ "path": other.to_str(), "program": "cp", "args": "{input} {output}" })).unwrap();
    assert!(r["check"].as_str().unwrap().starts_with("not checked"));
    // Failures are reported.
    assert!(s.run("export.convert", &json!({ "path": other.to_str(), "program": "no-such-converter-program", "args": "{input} {output}" })).is_err());
    assert!(
        s.run("export.convert", &json!({ "path": other.to_str(), "program": "cp", "args": "{input}" })).is_err(),
        "the arguments must name the output"
    );
    assert!(s.run("export.convert", &json!({ "path": dir.join("x.docx").to_str(), "program": "false", "args": "{input} {output}" })).is_err());
    let _ = std::fs::remove_dir_all(&dir);
}
