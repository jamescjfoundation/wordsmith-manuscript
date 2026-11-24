// SPDX-License-Identifier: MIT OR Apache-2.0
//! Export commands: files written only when every word matches, presets kept in the project.

use std::path::Path;

use serde_json::json;
use wordsmith_manuscript_model::Project;
use wordsmith_manuscript_ui::session::Session;

#[test]
fn exports_write_checked_files_and_presets_are_kept() {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("ui-export");
    let _ = std::fs::remove_dir_all(&dir);
    let mut s = Session::new(Project::new());
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/documents/pride-and-prejudice.odt");
    s.run("import.document", &json!({ "path": fixture.to_string_lossy() })).unwrap();
    let before = s.project.clone();
    for ext in ["odt", "rtf", "md", "txt", "html", "docx"] {
        let path = dir.join(format!("out.{ext}"));
        let out = s.run("export.file", &json!({ "path": path.to_string_lossy() })).unwrap();
        assert!(out["check"].as_str().unwrap().starts_with("every word matches"), "{ext}: {out}");
        assert!(path.exists());
    }
    assert_eq!(s.project, before, "export never changes the project");
    s.run("export.savePreset", &json!({ "name": "Readers", "options": { "chapters": "words", "end_mark": true } })).unwrap();
    assert_eq!(s.run("export.presets", &json!({})).unwrap()["Readers"]["chapters"], "words");
    let path = dir.join("readers.txt");
    s.run("export.file", &json!({ "path": path.to_string_lossy(), "preset": "Readers" })).unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains("Chapter One") && text.trim_end().ends_with("END"));
    assert!(s.run("export.file", &json!({ "path": dir.join("x.xyz").to_string_lossy() })).is_err());
    s.project.settings.book.author = "Test Author".into();
    let out = s.run("export.file", &json!({ "path": dir.join("std.pdf").to_string_lossy(), "format": "manuscript-pdf", "standard": { "paper": "a4", "face": "mono" } })).unwrap();
    assert!(out["pages"].as_u64().unwrap() > 300, "{out}");
    assert!(out["check"].as_str().unwrap().starts_with("every word matches"));
    let out = s.run("export.file", &json!({ "path": dir.join("std.docx").to_string_lossy(), "format": "manuscript-docx" })).unwrap();
    assert!(out["check"].as_str().unwrap().starts_with("every word matches"));
}
