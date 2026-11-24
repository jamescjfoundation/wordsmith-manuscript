// SPDX-License-Identifier: MIT OR Apache-2.0
//! Plain exports: every format, read back word for word; options shape the headings.

use std::path::Path;

use wordsmith_manuscript_format::dialect;
use wordsmith_manuscript_io::compile::{self, Options, Piece, Replace, Titles};
use wordsmith_manuscript_io::export::{Target, export};
use wordsmith_manuscript_io::import::{TextRules, plan_file};
use wordsmith_manuscript_io::mapping::Rules;
use wordsmith_manuscript_model::{Area, Item, ItemId, ItemText, Kind, Place, Project, Role};

fn fixture(p: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures").join(p)
}

/// A project made from an import plan.
pub fn project_from(path: &Path) -> Project {
    let plan = plan_file(path, &Rules::default(), &TextRules::default()).unwrap();
    let mut p = Project::new();
    let mut parents: Vec<Option<ItemId>> = vec![None];
    for it in plan.items {
        let item = Item::new(it.kind, it.title).role(it.role);
        let id = item.id;
        let place = parents.get(it.depth).copied().flatten().map_or(Place::end_of(Area::Manuscript), |x| Place::inside(Area::Manuscript, x));
        p.binder.insert(place, item).unwrap();
        p.texts.insert(id, it.text);
        parents.truncate(it.depth + 1);
        parents.push(Some(id));
    }
    p
}

fn small() -> Project {
    let mut p = Project::new();
    let ch = Item::with_id(ItemId::from_counter(1), Kind::Folder, "The Arrival").role(Role::Chapter);
    let s1 = Item::with_id(ItemId::from_counter(2), Kind::Text, "Dock").role(Role::Scene);
    let s2 = Item::with_id(ItemId::from_counter(3), Kind::Text, "Inn").role(Role::Scene);
    p.binder.insert(Place::end_of(Area::Manuscript), ch.with_children(vec![s1, s2])).unwrap();
    let mut skip = Item::with_id(ItemId::from_counter(4), Kind::Text, "Cut").role(Role::Chapter);
    skip.include = false;
    p.binder.insert(Place::end_of(Area::Manuscript), skip).unwrap();
    p.texts.insert(
        ItemId::from_counter(2),
        ItemText::with_body(dialect::parse("She *stepped* ashore[^1].\n\n> A quoted letter.\n\n***\n\nLater.\n\n[^1]: At dawn.\n")),
    );
    p.texts.insert(ItemId::from_counter(3), ItemText::with_body(dialect::parse("The inn was \"full\".\n")));
    p.texts.insert(ItemId::from_counter(4), ItemText::with_body(dialect::parse("Never exported.\n")));
    p
}

#[test]
fn every_plain_format_reads_back_word_for_word() {
    let novel = project_from(&fixture("documents/pride-and-prejudice.docx"));
    for target in [Target::Odt, Target::Rtf, Target::Markdown, Target::Text, Target::Html, Target::Docx] {
        let out = export(&novel, &Options::default(), target).unwrap();
        assert!(out.check.passed(), "{target:?}: {}", out.check);
        let out = export(&small(), &Options { end_mark: true, ..Options::default() }, target).unwrap();
        assert!(out.check.passed(), "{target:?} (small): {}", out.check);
    }
}

#[test]
fn options_shape_headings_breaks_and_replacements() {
    let p = small();
    let c = compile::compile(&p, &Options { chapters: Titles::Words, ..Options::default() });
    assert!(matches!(&c.pieces[0], Piece::Heading { text, .. } if text == "Chapter One"));
    // A scene break between the two scenes, and the one inside the first.
    assert_eq!(c.pieces.iter().filter(|x| **x == Piece::SceneBreak).count(), 2);
    assert!(!c.words().contains(&"exported.".to_owned()), "items left out of export stay out");
    let c = compile::compile(
        &p,
        &Options { chapters: Titles::NumberedWithTitle, replace: vec![Replace { find: "inn".into(), with: "tavern".into() }], ..Options::default() },
    );
    assert!(matches!(&c.pieces[0], Piece::Heading { text, .. } if text == "Chapter 1: The Arrival"));
    assert!(c.words().contains(&"tavern".to_owned()));
    let c = compile::compile(&p, &Options { chapters: Titles::None, scene_titles: true, ..Options::default() });
    assert!(matches!(&c.pieces[0], Piece::Heading { text, level: 3, .. } if text == "Dock"));
    // Export never changes the project.
    let before = p.clone();
    let _ = export(&p, &Options::default(), Target::Markdown).unwrap();
    assert_eq!(p, before);
}

#[test]
fn comments_become_review_comments_in_docx_and_never_text() {
    let mut p = small();
    let id = ItemId::from_counter(3);
    let t = p.texts.get_mut(&id).unwrap();
    t.body = dialect::parse("The inn was [full and loud]{.comment id=c1}.\n\n[It stayed]{.comment id=c2} [quiet]{.comment id=c2} all night.\n");
    t.meta
        .comments
        .insert("c1".into(), wordsmith_manuscript_model::Comment { text: "Too plain?".into(), by: "A. Writer".into(), date: "2026-01-02".into() });
    t.meta.comments.insert("c2".into(), wordsmith_manuscript_model::Comment { text: "Check the timing.".into(), ..Default::default() });
    let with = export(&p, &Options { comments: true, ..Options::default() }, Target::Docx).unwrap();
    assert!(with.check.passed(), "{}", with.check);
    let doc = jcj_interchange::import(jcj_interchange::Format::Docx, &with.bytes).unwrap();
    assert_eq!(doc.comments.len(), 2, "one review comment each, even when its words are split by a space");
    let texts: Vec<String> = doc
        .comments
        .values()
        .map(|c| {
            doc.parts
                .get(&c.part)
                .map(|part| part.blocks.iter().filter_map(|b| b.as_para()).map(|q| q.plain_text()).collect::<String>())
                .unwrap_or_default()
        })
        .collect();
    assert!(texts.contains(&"Too plain?".to_owned()) && texts.contains(&"Check the timing.".to_owned()), "{texts:?}");
    assert!(doc.comments.values().any(|c| c.author == "A. Writer"));
    // Off by default: the words only.
    let without = export(&p, &Options::default(), Target::Docx).unwrap();
    assert!(without.check.passed());
    assert!(jcj_interchange::import(jcj_interchange::Format::Docx, &without.bytes).unwrap().comments.is_empty());
    // ODT and RTF have review comments too.
    for target in [Target::Odt, Target::Rtf] {
        let out = export(&p, &Options { comments: true, ..Options::default() }, target).unwrap();
        assert!(out.check.passed(), "{target:?}: {}", out.check);
        assert_eq!(jcj_interchange::import(target.format(), &out.bytes).unwrap().comments.len(), 2, "{target:?}");
    }
    // The other formats carry the words, never the comment's text.
    for target in [Target::Markdown, Target::Text, Target::Html] {
        let out = export(&p, &Options { comments: true, ..Options::default() }, target).unwrap();
        assert!(out.check.passed(), "{target:?}: {}", out.check);
        assert!(!String::from_utf8_lossy(&out.bytes).contains("Too plain"), "{target:?}");
    }
}

#[test]
fn an_epub_reading_copy_reads_back_word_for_word() {
    let mut novel = project_from(&fixture("documents/pride-and-prejudice.docx"));
    novel.settings.book.title = "Pride and Prejudice".into();
    novel.settings.book.author = "Jane Austen".into();
    let out = export(&novel, &Options::default(), Target::Epub).unwrap();
    assert!(out.check.passed(), "{}", out.check);
    assert!(out.bytes.starts_with(b"PK"));
    let back = jcj_interchange::import(jcj_interchange::Format::Epub, &out.bytes).unwrap();
    assert_eq!(back.core.title, "Pride and Prejudice");
    assert_eq!(back.core.creator, "Jane Austen");
    // One reading document per chapter, so a reader's contents lists them.
    let names: Vec<String> = {
        let mut z = zip::ZipArchive::new(std::io::Cursor::new(&out.bytes)).unwrap();
        (0..z.len()).map(|i| z.by_index(i).unwrap().name().to_owned()).collect()
    };
    assert!(names.iter().filter(|n| n.ends_with(".xhtml")).count() > 50, "{names:?}");
    let small = export(&small(), &Options { end_mark: true, ..Options::default() }, Target::Epub).unwrap();
    assert!(small.check.passed(), "{}", small.check);
    // Notes are kept, as numbered notes at the end of their chapter.
    let mut z = zip::ZipArchive::new(std::io::Cursor::new(&small.bytes)).unwrap();
    let mut all = String::new();
    for i in 0..z.len() {
        let mut f = z.by_index(i).unwrap();
        let _ = std::io::Read::read_to_string(&mut f, &mut all);
    }
    assert!(all.contains("1. At dawn."), "the note is in the reading copy");
    assert_eq!(Target::from_extension("EPUB"), Some(Target::Epub));
}

#[test]
fn an_epub_comes_back_as_an_editable_manuscript() {
    // A reading copy made by the export above, read back as a manuscript (IMP-9).
    let novel = project_from(&fixture("documents/pride-and-prejudice.docx"));
    let out = export(&novel, &Options::default(), Target::Epub).unwrap();
    let dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("epub-import");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("Pride and Prejudice.epub");
    std::fs::write(&path, &out.bytes).unwrap();
    let plan = plan_file(&path, &Rules::default(), &TextRules::default()).unwrap();
    assert!(plan.report.check.as_ref().is_some_and(|c| c.passed()), "{:?}", plan.report.check);
    let chapters = plan.items.iter().filter(|i| i.role == Role::Chapter).count();
    let original = novel.binder.rows_of(Area::Manuscript).iter().filter(|r| r.item.role == Role::Chapter).count();
    assert_eq!(chapters, original, "every chapter comes back as a chapter");
    let words = |p: &Project| p.texts.values().map(|t| t.body.word_count()).sum::<usize>();
    let back: usize = plan.items.iter().map(|i| i.text.body.word_count()).sum::<usize>()
        + plan.items.iter().filter(|i| i.title_from_source).map(|i| i.title.split_whitespace().count()).sum::<usize>();
    assert!(back >= words(&novel), "{back} >= {}", words(&novel));
}
