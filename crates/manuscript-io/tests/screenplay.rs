// SPDX-License-Identifier: MIT OR Apache-2.0
//! The `.wss` reader against one file per version of Wordsmith Script's format
//! (`tests/fixtures/scripts/`, `docs/fixtures.md`): every version the reader was written for
//! reads with every word, and a newer one is refused with both versions named.
//!
//! Script owns the format. When it adds a version, the reader, `NEWEST_FORMAT` and this folder
//! change together: `every_format_the_reader_accepts_has_a_fixture` fails until they do.

use std::path::{Path, PathBuf};

use wordsmith_manuscript_io::screenplay::{self, NEWEST_FORMAT};

fn dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/scripts")
}

/// The fixtures by the format version their first line declares.
fn fixtures() -> Vec<(u32, String, Vec<u8>)> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir()).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("wss") {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let bytes = std::fs::read(&path).unwrap();
        let first = String::from_utf8_lossy(&bytes).lines().next().unwrap_or("").to_owned();
        let version: u32 = first.strip_prefix("Wordsmith Script ").and_then(|v| v.parse().ok()).unwrap_or_else(|| panic!("{name}: no version line"));
        assert!(name.starts_with(&format!("v{version}-")), "{name} declares format {version}");
        out.push((version, name, bytes));
    }
    out.sort();
    out
}

/// What each fixture must read as: its title, its words (counted by hand: a piece with no
/// letter or digit, such as a lone dash, is not a word), then each paragraph's element and plain
/// text.
fn expected(name: &str) -> (&'static str, usize, Vec<(&'static str, &'static str)>) {
    match name {
        // Format 1: no marks in braces yet.
        "v1-short.wss" => ("Fixture", 6, vec![("SceneHeading", "EXT. YARD - DAY"), ("Action", "Wind."), ("Character", "BOB"), ("Dialogue", "Hi.")]),
        // Format 2: a note's anchor, an escaped brace, an empty line inside a paragraph.
        "v2-notes.wss" => (
            "Fixture 2",
            13,
            vec![("SceneHeading", "INT. HALL {B} - NIGHT"), ("Action", "Rain hits the window.\n\nA second line."), ("NewAct", "ACT TWO")],
        ),
        // Format 3: a locked page's start, revised text, locked scene numbers.
        "v3-revisions.wss" => ("Fixture 3", 8, vec![("SceneHeading", "INT. A - DAY"), ("Action", "Rain. Hard"), ("SceneHeading", "EXT. B - NIGHT")]),
        // Format 4: style marks and tracked changes. A tracked deletion's words are still in
        // the file, so they are still shown and counted.
        "v4-styles.wss" => (
            "Fixture 4",
            14,
            vec![
                ("SceneHeading", "INT. HALL - DAY"),
                ("Action", "Rain falls. Hard."),
                ("Action", "Struck double marked raised"),
                ("Character", "ANNA"),
                ("Dialogue", "Hi."),
                ("Character", "BEN"),
                ("Dialogue", "Hello."),
            ],
        ),
        // Format 5: the settings' `ext` and `trackChanges`; the text as in format 4.
        "v5-ext.wss" => (
            "Harbour Lights",
            23,
            vec![
                ("SceneHeading", "EXT. HARBOUR - NIGHT"),
                ("Action", "Rain hammers the pier. A *star* and a {brace} show."),
                ("Character", "MARA"),
                ("Parenthetical", "(low)"),
                ("Dialogue", "We go now.\n\nBefore the tide."),
                ("Transition", "CUT TO:"),
            ],
        ),
        other => panic!("{other} has no expected reading: add one"),
    }
}

#[test]
fn every_format_the_reader_accepts_has_a_fixture() {
    let have: Vec<u32> = fixtures().iter().map(|(v, ..)| *v).collect();
    let want: Vec<u32> = (1..=NEWEST_FORMAT + 1).collect();
    assert_eq!(have, want, "one fixture per format 1 to {NEWEST_FORMAT}, and one for format {}", NEWEST_FORMAT + 1);
}

#[test]
fn each_accepted_format_reads_with_every_word() {
    for (version, name, bytes) in fixtures().into_iter().filter(|(v, ..)| *v <= NEWEST_FORMAT) {
        assert!(screenplay::is_script(&bytes), "{name}");
        let s = screenplay::read(&bytes).unwrap_or_else(|e| panic!("{name}: {e}"));
        let (title, words, paragraphs) = expected(&name);
        assert_eq!(s.version, version, "{name}");
        assert_eq!(s.title, title, "{name}");
        let got: Vec<(&str, &str)> = s.paragraphs.iter().map(|p| (p.element.as_str(), p.text.as_str())).collect();
        assert_eq!(got, paragraphs, "{name}");
        assert_eq!(s.words(), words, "{name}");
        assert_eq!(s.scenes(), paragraphs.iter().filter(|(e, _)| *e == "SceneHeading").count(), "{name}");
    }
}

#[test]
fn a_newer_format_is_refused_with_both_versions_named() {
    let newer: Vec<_> = fixtures().into_iter().filter(|(v, ..)| *v > NEWEST_FORMAT).collect();
    assert_eq!(newer.len(), 1);
    for (version, name, bytes) in newer {
        let e = screenplay::read(&bytes).expect_err(&name);
        assert!(e.contains(&format!("format {version}")), "{name}: {e}");
        assert!(e.contains(&format!("formats 1 to {NEWEST_FORMAT}")), "{name}: {e}");
    }
}
