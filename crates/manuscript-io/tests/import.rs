// SPDX-License-Identifier: MIT OR Apache-2.0
//! Importing the fixtures: DOCX, ODT and RTF become chapters with every word checked; plain text
//! and Markdown split by headings and separators; scene breaks split chapters.

use std::path::{Path, PathBuf};

use wordsmith_manuscript_io::import::{TextRules, plan_file, text_document};
use wordsmith_manuscript_io::mapping::{self, Rules};
use wordsmith_manuscript_model::{Block, Kind, Role};

fn fixture(p: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures").join(p)
}

#[test]
fn docx_odt_and_rtf_become_chapters_with_every_word_checked() {
    for f in ["documents/pride-and-prejudice.docx", "documents/pride-and-prejudice.odt", "documents/pride-and-prejudice.rtf"] {
        let plan = plan_file(&fixture(f), &Rules::default(), &TextRules::default()).unwrap();
        let check = plan.report.check.clone().unwrap();
        assert!(check.passed(), "{f}: {check}");
        assert!(plan.report.notes_check.as_ref().unwrap().passed());
        let chapters = plan.items.iter().filter(|i| i.role == Role::Chapter).count();
        assert!(chapters >= 61, "{f}: {chapters} chapters");
        assert_eq!(plan.items[0].role, Role::FrontMatter, "{f}: the text before the first chapter is front matter");
        // Italic survives.
        let italic = plan.items.iter().any(|i| i.text.body.blocks.iter().any(|b| matches!(b, Block::Para(c) if c.iter().any(|x| x.marks().italic))));
        assert!(italic, "{f}");
    }
}

#[test]
fn plain_text_joins_wrapped_lines_and_finds_chapters() {
    let plan = plan_file(&fixture("novels/great-expectations.txt"), &Rules::default(), &TextRules::default()).unwrap();
    assert!(plan.report.check.as_ref().unwrap().passed(), "{}", plan.report.check.as_ref().unwrap());
    let chapters: Vec<&str> = plan.items.iter().filter(|i| i.role == Role::Chapter).map(|i| i.title.as_str()).collect();
    assert_eq!(chapters.len(), 59, "Chapter lines start chapters without heading styles");
    assert_eq!(chapters[0], "Chapter I.");
}

#[test]
fn scene_breaks_split_chapters_or_stay() {
    let doc = text_document("Chapter 1\n\nOne.\n\n* * *\n\nTwo.\n\n#\n\nThree.\n\nChapter 2\n\nAlone.\n", &TextRules::default());
    let p = mapping::plan(&doc, &Rules::default());
    assert!(p.report.check.as_ref().unwrap().passed());
    let shape: Vec<(usize, Role, Kind, String)> = p.items.iter().map(|i| (i.depth, i.role, i.kind, i.title.clone())).collect();
    assert_eq!(
        shape,
        vec![
            (0, Role::Chapter, Kind::Folder, "Chapter 1".into()),
            (1, Role::Scene, Kind::Text, "Scene 1".into()),
            (1, Role::Scene, Kind::Text, "Scene 2".into()),
            (1, Role::Scene, Kind::Text, "Scene 3".into()),
            (0, Role::Chapter, Kind::Text, "Chapter 2".into()),
        ]
    );
    let keep = Rules { split_scenes: false, ..Rules::default() };
    let p = mapping::plan(&doc, &keep);
    assert_eq!(p.items.len(), 2);
    assert_eq!(p.items[0].text.body.blocks.iter().filter(|b| **b == Block::SceneBreak).count(), 2);
}

#[test]
fn markdown_headings_make_parts_and_chapters() {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("import-md");
    std::fs::create_dir_all(&dir).unwrap();
    let f = dir.join("book.md");
    std::fs::write(
        &f,
        "# Part One\n\nOpening words.\n\n## The Arrival\n\nShe *came* home.\n\n## The Storm\n\nRain.\n\n# Part Two\n\n## Calm\n\nQuiet.\n",
    )
    .unwrap();
    let plan = plan_file(&f, &Rules::default(), &TextRules::default()).unwrap();
    assert!(plan.report.check.as_ref().unwrap().passed(), "{}", plan.report.check.as_ref().unwrap());
    let shape: Vec<(usize, Role, String)> = plan.items.iter().map(|i| (i.depth, i.role, i.title.clone())).collect();
    assert_eq!(
        shape,
        vec![
            (0, Role::Part, "Part One".into()),
            (1, Role::Chapter, "The Arrival".into()),
            (1, Role::Chapter, "The Storm".into()),
            (0, Role::Part, "Part Two".into()),
            (1, Role::Chapter, "Calm".into()),
        ]
    );
}

#[test]
fn hostile_files_fail_cleanly() {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("import-hostile");
    std::fs::create_dir_all(&dir).unwrap();
    for (name, bytes) in
        [("x.docx", b"PK\x03\x04garbage".to_vec()), ("y.rtf", b"{\\rtf1 {{{{{{{{{{".to_vec()), ("z.odt", vec![0u8; 100]), ("w.bin", vec![1, 2, 3])]
    {
        let f = dir.join(name);
        std::fs::write(&f, bytes).unwrap();
        let _ = plan_file(&f, &Rules::default(), &TextRules::default());
    }
    assert!(plan_file(&dir.join("missing.docx"), &Rules::default(), &TextRules::default()).is_err());
}

#[test]
fn text_without_chapters_splits_at_separators_or_stays_one() {
    let doc = text_document("First piece.\n\n***\n\nSecond piece.\n\n#\n\nThird.\n", &TextRules::default());
    let p = mapping::plan(&doc, &Rules::default());
    assert!(p.report.check.as_ref().unwrap().passed());
    let titles: Vec<(&str, Role)> = p.items.iter().map(|i| (i.title.as_str(), i.role)).collect();
    assert_eq!(titles, [("Chapter 1", Role::Chapter), ("Chapter 2", Role::Chapter), ("Chapter 3", Role::Chapter)]);
    let doc = text_document("Just one\nwrapped paragraph.\n\nAnd another.\n", &TextRules::default());
    let p = mapping::plan(&doc, &Rules::default());
    assert_eq!(p.items.len(), 1);
    assert_eq!(p.items[0].text.body.blocks.len(), 2);
    // A chosen separator.
    let doc = text_document("A.\n\n-- next --\n\nB.\n", &TextRules::default());
    let p = mapping::plan(&doc, &Rules { scene_marks: vec!["-- next --".into()], ..Rules::default() });
    assert_eq!(p.items.len(), 2);
    // Lines kept as paragraphs when asked.
    let doc = text_document("one\ntwo\n", &TextRules { join_lines: false });
    assert_eq!(mapping::plan(&doc, &Rules::default()).items[0].text.body.blocks.len(), 2);
}

#[test]
fn front_matter_before_chapters_keeps_its_breaks() {
    let doc = text_document("Title\n\n***\n\nDedication.\n\nChapter 1\n\nText.\n", &TextRules::default());
    let p = mapping::plan(&doc, &Rules::default());
    assert_eq!(p.items[0].role, Role::FrontMatter);
    assert!(p.items[0].text.body.blocks.contains(&Block::SceneBreak));
    assert!(p.report.check.as_ref().unwrap().passed());
}
