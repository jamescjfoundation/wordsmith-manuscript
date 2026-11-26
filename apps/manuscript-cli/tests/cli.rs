// SPDX-License-Identifier: MIT OR Apache-2.0
//! The command line: convert, export, counts, verify, report and run, on the fixtures.

use std::path::{Path, PathBuf};
use std::process::Command;

fn cli(args: &[&str]) -> (bool, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_manuscript-cli")).args(args).output().unwrap();
    (out.status.success(), String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr))
}

fn scratch(name: &str) -> PathBuf {
    let d = Path::new(env!("CARGO_TARGET_TMPDIR")).join("cli").join(name);
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn fixture(p: &str) -> String {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures").join(p).to_string_lossy().into_owned()
}

#[test]
fn convert_export_count_verify_and_report() {
    let dir = scratch("all");
    let project = dir.join("Pride.wsm");
    let p = project.to_string_lossy().into_owned();
    let (ok, out) = cli(&["convert", &fixture("documents/pride-and-prejudice.docx"), &p]);
    assert!(ok, "{out}");
    assert!(out.contains("every word matches"), "{out}");
    assert!(project.join("project.toml").exists());
    let (ok, out) = cli(&["counts", &p]);
    assert!(ok && out.contains("\"words\""), "{out}");
    let md = dir.join("pride.md").to_string_lossy().into_owned();
    let (ok, out) = cli(&["export", &p, &md]);
    assert!(ok && out.contains("every word matches"), "{out}");
    // The project's words equal the source's.
    let (ok, out) = cli(&["verify", &fixture("documents/pride-and-prejudice.docx"), &p]);
    assert!(ok, "{out}");
    // A changed word is found and shown with its context.
    let other = dir.join("other.txt");
    std::fs::write(&other, "It is a truth universally acknowledged.").unwrap();
    let (ok, out) = cli(&["verify", &fixture("documents/pride-and-prejudice.docx"), &other.to_string_lossy()]);
    assert!(!ok && out.contains("word 1 differs"), "{out}");
    let (ok, out) = cli(&["report", &p]);
    assert!(ok && out.contains("missing_book_metadata"), "{out}");
    let (ok, out) = cli(&["run", &p, "binder.new", r#"{"area": "notes", "title": "From the command line"}"#]);
    assert!(ok, "{out}");
    let (_, out) = cli(&["run", &p, "binder.list", r#"{"area": "notes"}"#]);
    assert!(out.contains("From the command line"), "saved by run: {out}");
    let (ok, out) = cli(&["commands"]);
    assert!(ok && out.contains("binder.new"));
}

#[test]
fn convert_a_document_set_folder_and_into_a_packed_file() {
    let dir = scratch("set");
    let packed = dir.join("Great Expectations.wsmz").to_string_lossy().into_owned();
    let (ok, out) = cli(&["convert", &fixture("document-set"), &packed]);
    assert!(ok, "{out}");
    let (ok, out) = cli(&["counts", &packed]);
    assert!(ok && out.contains("\"words\": 18"), "{out}");
    let pdf = dir.join("ge.pdf").to_string_lossy().into_owned();
    let (ok, out) = cli(&["export", &packed, &pdf, "--format", "manuscript-pdf"]);
    assert!(ok && out.contains("every word matches"), "{out}");
}
