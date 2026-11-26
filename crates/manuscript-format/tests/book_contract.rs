// SPDX-License-Identifier: MIT OR Apache-2.0
//! The contract with Wordsmith Book, as code (`docs/book-handoff.md`, "What Book may use").
//!
//! Wordsmith Book builds on this crate and on `wordsmith-manuscript-model`, at a tag of this
//! repository. [`SURFACE`] names every item the contract lists and pairs it with a function
//! that uses it the way Book does. So a change Book would not survive fails here first:
//!
//! - an item removed, renamed or moved does not compile;
//! - a struct Book builds field by field is built field by field, so a new field does not
//!   compile;
//! - an enum Book matches in full is matched in full, so a new variant does not compile;
//! - what Book relies on at run time (what the book view keeps and leaves out, which schemas
//!   read, which schema is written) is asserted.
//!
//! `the_document_and_this_list_name_the_same_items` keeps the list here and the list in the
//! document the same, in both directions. The workflow `book-consumer.yml` goes further and
//! runs Book's own gate against this checkout.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use wordsmith_manuscript_format as format;
use wordsmith_manuscript_model as model;

/// Some items of the contract, by the names the document gives them, and a use of them.
type Uses = (&'static [&'static str], fn());

/// Every item of the contract. `model::` is `wordsmith_manuscript_model::` and `format::` is
/// `wordsmith_manuscript_format::`.
const SURFACE: &[Uses] = &[
    // The project and its binder.
    (&["model::Project", "model::Project::new", "model::Project::text", "model::Project::included_manuscript"], project),
    (&["model::BookInfo"], book_info),
    (
        &["model::Binder::insert", "model::Binder::get", "model::Binder::get_mut", "model::Binder::remove", "model::Binder::rows_of", "model::Row"],
        binder,
    ),
    (&["model::Item", "model::Item::with_id", "model::Item::role"], item),
    (&["model::ItemId", "model::ItemId::from_counter", "model::ItemId::as_text", "model::BadId"], item_id),
    (&["model::ItemText", "model::ItemText::with_body"], item_text),
    (&["model::Kind", "model::Area", "model::Place::end_of", "model::Place::inside"], kinds_areas_and_places),
    (&["model::Role"], role),
    // The text of an item.
    (
        &[
            "model::Text",
            "model::Text::from_paragraphs",
            "model::Text::plain_paragraphs",
            "model::Text::inline_runs",
            "model::Text::inline_runs_mut",
            "model::Text::normalize",
        ],
        text,
    ),
    (&["model::Block"], block),
    (&["model::Inline", "model::Inline::plain", "model::Inline::marked", "model::Inlines"], inline),
    (&["model::Marks", "model::Marks::italic"], marks),
    (&["model::Note"], note),
    (&["model::Fence"], fence),
    (&["model::ImageRole", "model::ImageRole::key", "model::ImageRole::from_key"], image_role),
    (&["model::text::plain", "model::text::normalize_inlines"], text_functions),
    (&["model::words::words"], words),
    (&["model::series::relative"], series_relative),
    // Reading and writing a project.
    (&["format::SCHEMA"], schemas),
    (&["format::Error"], errors),
    (
        &[
            "format::folder::Folder",
            "format::folder::Folder::open",
            "format::folder::Folder::create",
            "format::folder::Folder::save",
            "format::folder::Folder::outside_changes",
        ],
        folder,
    ),
    (
        &["format::pack::MAX_PACKED", "format::pack::pack_folder", "format::pack::folder_files", "format::pack::unpack", "format::pack::read_packed"],
        packed,
    ),
    // The book view.
    (
        &[
            "format::book::VERSION",
            "format::book::Book",
            "format::book::Metadata",
            "format::book::Section",
            "format::book::Asset",
            "format::book::from_project",
            "format::book::outline",
            "format::book::counts",
            "format::book::missing_metadata",
        ],
        book_view,
    ),
    (&["format::book::FrontKind"], front_kind),
    // Fixes offered back.
    (
        &[
            "format::offers::VERSION",
            "format::offers::MAX_FILE",
            "format::offers::Offer",
            "format::offers::OffersFile",
            "format::offers::Decision",
            "format::offers::offers_path",
            "format::offers::read",
            "format::offers::write",
            "format::offers::decisions",
            "format::offers::decide",
        ],
        offers,
    ),
];

/// The names in [`SURFACE`].
fn listed() -> Vec<&'static str> {
    SURFACE.iter().flat_map(|(names, _)| names.iter().copied()).collect()
}

fn id(n: u64) -> model::ItemId {
    model::ItemId::from_counter(n)
}

fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("book-contract").join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// A small book: a title page, a part holding a chapter of two scenes, a chapter left out of
/// export, an afterword, and a picture in Research.
fn sample() -> model::Project {
    use model::{Area, Item, ItemText, Kind, Place, Role};
    let mut p = model::Project::new();
    p.settings.book.title = "The Lighthouse".into();
    p.settings.book.author = "A. Writer".into();
    let place = Place::end_of(Area::Manuscript);
    p.binder.insert(place, Item::with_id(id(1), Kind::Text, "Title Page").role(Role::FrontMatter)).unwrap();
    p.binder.insert(Place::end_of(Area::Manuscript), Item::with_id(id(2), Kind::Folder, "The Rock").role(Role::Part)).unwrap();
    p.binder.insert(Place::inside(Area::Manuscript, id(2)), Item::with_id(id(3), Kind::Folder, "Arrival").role(Role::Chapter)).unwrap();
    p.binder.insert(Place::inside(Area::Manuscript, id(3)), Item::with_id(id(4), Kind::Text, "Dusk").role(Role::Scene)).unwrap();
    p.binder.insert(Place::inside(Area::Manuscript, id(3)), Item::with_id(id(5), Kind::Text, "Night").role(Role::Scene)).unwrap();
    let mut cut = Item::with_id(id(6), Kind::Text, "Cut").role(Role::Chapter);
    cut.include = false;
    p.binder.insert(Place::end_of(Area::Manuscript), cut).unwrap();
    p.binder.insert(Place::end_of(Area::Manuscript), Item::with_id(id(7), Kind::Text, "Afterword").role(Role::BackMatter)).unwrap();
    let mut picture = Item::with_id(id(8), Kind::Media, "Map");
    picture.file = Some("png".into());
    p.binder.insert(Place::end_of(Area::Research), picture).unwrap();
    let noted = model::Text {
        blocks: vec![model::Block::Para(vec![
            model::Inline::plain("The keeper "),
            model::Inline::Span {
                text: "climbed".into(),
                marks: model::Marks { comment: Some("c1".into()), rev: Some(0), ..model::Marks::default() },
            },
            model::Inline::plain(" the rock"),
            model::Inline::NoteRef { label: "1".into(), marks: model::Marks::default() },
            model::Inline::plain("."),
        ])],
        notes: vec![model::Note { label: "1".into(), paras: vec![vec![model::Inline::plain("At dusk.")]] }],
    };
    p.texts.insert(id(4), ItemText::with_body(noted));
    p.texts.insert(id(5), ItemText::with_body(model::Text::from_paragraphs(&["The sea was quiet."])));
    p.texts.insert(id(6), ItemText::with_body(model::Text::from_paragraphs(&["Never typeset."])));
    p.texts.insert(id(7), ItemText::with_body(model::Text::from_paragraphs(&["Thank you."])));
    p
}

fn project() {
    let mut p: model::Project = model::Project::new();
    // The three fields Book reads and writes.
    let _: &model::Settings = &p.settings;
    p.binder.insert(model::Place::end_of(model::Area::Manuscript), model::Item::with_id(id(1), model::Kind::Text, "One")).unwrap();
    p.texts.insert(id(1), model::ItemText::with_body(model::Text::from_paragraphs(&["Words."])));
    let text: Option<&model::ItemText> = p.text(id(1));
    assert_eq!(text.map(|t| t.body.plain_paragraphs()), Some(vec!["Words.".to_owned()]));
    assert!(p.text(id(2)).is_none());
    // The items an export holds, in reading order: those left out are not among them.
    let sample = sample();
    let included: Vec<model::ItemId> = sample.included_manuscript();
    assert!(included.contains(&id(4)) && !included.contains(&id(6)), "{included:?}");
    let clone = sample.clone();
    assert_eq!(clone, sample, "Book keeps a project to undo to, and compares two");
}

fn book_info() {
    let mut p = model::Project::new();
    let info: &mut model::BookInfo = &mut p.settings.book;
    info.title = "The Lighthouse".into();
    info.subtitle = "A Story".into();
    info.author = "A. Writer".into();
    info.pen_name = "A. Penname".into();
    info.language = "en-GB".into();
    info.series = "The Rock".into();
    info.series_number = Some(2);
    info.isbn.insert("print".into(), "978-0-306-40615-7".into());
    info.publisher = "A Press".into();
    info.copyright_year = Some(2026);
    info.rights = "All rights reserved.".into();
    info.description = "A keeper and the sea.".into();
    info.subjects = vec!["Fiction".into()];
    info.keywords = vec!["sea".into()];
    let copy: model::BookInfo = info.clone();
    assert_eq!(&copy, info);
    // The book view carries all of it, and the name the book is published under.
    let view = format::book::from_project(&p);
    assert_eq!(view.metadata.info, copy);
    assert_eq!(view.metadata.byline, "A. Penname");
}

fn binder() {
    use model::{Area, Item, Kind, Place};
    let mut p = model::Project::new();
    let inserted: Result<(), model::BinderError> = p.binder.insert(Place::end_of(Area::Manuscript), Item::with_id(id(1), Kind::Folder, "Part"));
    assert!(inserted.is_ok());
    p.binder.insert(Place::inside(Area::Manuscript, id(1)), Item::with_id(id(2), Kind::Text, "Chapter")).unwrap();
    let got: Option<&Item> = p.binder.get(id(2));
    assert_eq!(got.map(|i| i.title.as_str()), Some("Chapter"));
    let changed: Option<&mut Item> = p.binder.get_mut(id(2));
    if let Some(i) = changed {
        i.title = "Renamed".into();
    }
    let rows: Vec<model::Row<'_>> = p.binder.rows_of(Area::Manuscript);
    let titles: Vec<&str> = rows.iter().map(|row| row.item.title.as_str()).collect();
    assert_eq!(titles, ["Part", "Renamed"], "rows are in reading order, parents before children");
    assert_eq!(rows.iter().map(|row| row.item.id).collect::<Vec<_>>(), [id(1), id(2)]);
    let removed: Option<(Place, Item)> = p.binder.remove(id(2));
    assert!(removed.is_some() && p.binder.get(id(2)).is_none());
}

fn item() {
    let item: model::Item = model::Item::with_id(id(9), model::Kind::Text, "A Title").role(model::Role::Chapter);
    // The fields Book reads, and the one it sets on a fixture.
    let (item_id, title, include): (model::ItemId, &String, bool) = (item.id, &item.title, item.include);
    assert_eq!((item_id, title.as_str(), include), (id(9), "A Title", true));
    assert_eq!((item.kind, item.role), (model::Kind::Text, model::Role::Chapter));
    assert!(item.children.is_empty());
}

fn item_id() {
    let a: model::ItemId = model::ItemId::from_counter(7);
    // The same number always gives the same id, so a fixture is stable.
    assert_eq!(a, model::ItemId::from_counter(7));
    assert_ne!(a, model::ItemId::from_counter(8));
    let text: String = a.as_text();
    assert_eq!(text, a.to_string());
    assert_eq!(text.len(), 36, "lowercase and hyphenated: {text}");
    let back: Result<model::ItemId, model::BadId> = text.parse();
    assert_eq!(back, Ok(a));
    let bad: model::BadId = "not an id".parse::<model::ItemId>().unwrap_err();
    assert!(!bad.to_string().is_empty());
    // Book keys its own layer by it: copied, ordered, hashed, and written as text.
    let copy = a;
    let ordered: BTreeSet<model::ItemId> = [copy, a].into_iter().collect();
    let hashed: std::collections::HashSet<model::ItemId> = [copy, a].into_iter().collect();
    assert_eq!((ordered.len(), hashed.len()), (1, 1));
}

fn item_text() {
    let t: model::ItemText = model::ItemText::with_body(model::Text::from_paragraphs(&["Words."]));
    let body: &model::Text = &t.body;
    assert_eq!(body.plain_paragraphs(), ["Words."]);
}

fn kinds_areas_and_places() {
    use model::{Area, Kind, Place};
    let _: [Kind; 2] = [Kind::Folder, Kind::Text];
    let _: [Area; 2] = [Area::Manuscript, Area::Research];
    let end: Place = Place::end_of(Area::Manuscript);
    let inside: Place = Place::inside(Area::Manuscript, id(1));
    assert_eq!((end.area, end.parent), (Area::Manuscript, None));
    assert_eq!((inside.area, inside.parent), (Area::Manuscript, Some(id(1))));
}

fn role() {
    // Matched in full: a new role is a new kind of section, which Book must decide how to set.
    for role in model::Role::ALL {
        let _: &str = match role {
            model::Role::None => "none",
            model::Role::Part => "part",
            model::Role::Chapter => "chapter",
            model::Role::Scene => "scene",
            model::Role::FrontMatter => "front matter",
            model::Role::BackMatter => "back matter",
        };
    }
    let copy = model::Role::Part;
    assert_eq!(copy, model::Role::Part);
}

fn text() {
    // Built field by field.
    let mut t = model::Text {
        blocks: vec![
            model::Block::Para(vec![model::Inline::plain("One "), model::Inline::plain("two.")]),
            model::Block::Quote(vec![vec![model::Inline::plain("Quoted.")]]),
        ],
        notes: vec![model::Note { label: "1".into(), paras: vec![vec![model::Inline::plain("A note.")]] }],
    };
    // Every paragraph's content in order: the blocks', then the notes'.
    let runs: Vec<&model::Inlines> = t.inline_runs();
    assert_eq!(runs.len(), 3);
    for run in t.inline_runs_mut() {
        let run: &mut model::Inlines = run;
        run.push(model::Inline::plain(""));
    }
    // The canonical form: neighbouring spans with the same marks are one, empty ones are gone.
    t.normalize();
    assert_eq!(t.blocks[0], model::Block::Para(vec![model::Inline::plain("One two.")]));
    // The paragraphs as a reader sees them: the blocks', then the notes'.
    assert_eq!(t.plain_paragraphs(), ["One two.", "Quoted.", "A note."]);
    let plain: model::Text = model::Text::from_paragraphs(&["First.", "Second."]);
    assert_eq!(plain.blocks.len(), 2);
    assert!(plain.notes.is_empty());
    assert_eq!(model::Text::default(), model::Text { blocks: Vec::new(), notes: Vec::new() });
}

fn block() {
    let span = || vec![model::Inline::plain("x")];
    let all = [
        model::Block::Para(span()),
        model::Block::Heading { level: 2, content: span() },
        model::Block::SceneBreak,
        model::Block::Quote(vec![span()]),
        model::Block::Fenced { kind: model::Fence::Verse, paras: vec![span()] },
        model::Block::Image { alt: "A map".into(), src: "media/map.png".into(), role: model::ImageRole::Wide },
        model::Block::Raw { format: "html".into(), text: "<hr/>".into() },
    ];
    // Matched in full, each with every field: a new kind of block, or a new field of one, is
    // something Book must learn to typeset.
    for b in &all {
        let _: usize = match b {
            model::Block::Para(content) => content.len(),
            model::Block::Heading { level, content } => usize::from(*level) + content.len(),
            model::Block::SceneBreak => 0,
            model::Block::Quote(paras) => paras.len(),
            model::Block::Fenced { kind: _, paras } => paras.len(),
            model::Block::Image { alt, src, role: _ } => alt.len() + src.len(),
            model::Block::Raw { format, text } => format.len() + text.len(),
        };
    }
    assert_eq!(all[2].clone(), model::Block::SceneBreak);
}

fn inline() {
    let content: model::Inlines = vec![
        model::Inline::plain("Plain "),
        model::Inline::marked("marked", model::Marks::italic()),
        model::Inline::NoteRef { label: "1".into(), marks: model::Marks::default() },
    ];
    for piece in &content {
        // Matched in full, each with every field.
        let _: &model::Marks = match piece {
            model::Inline::Span { text: _, marks } => marks,
            model::Inline::NoteRef { label: _, marks } => marks,
        };
    }
    assert_eq!(content[0], model::Inline::Span { text: "Plain ".into(), marks: model::Marks::default() });
    assert_eq!(content[1], model::Inline::Span { text: "marked".into(), marks: model::Marks { italic: true, ..model::Marks::default() } });
}

fn marks() {
    // Built field by field: a new mark is something Book must learn to draw, or to leave out.
    let m = model::Marks { italic: true, bold: false, small_caps: false, strike: false, lang: None, link: None, comment: None, rev: None };
    assert_eq!(m, model::Marks::italic());
    assert_ne!(m, model::Marks::default());
    let all = model::Marks {
        italic: true,
        bold: true,
        small_caps: true,
        strike: true,
        lang: Some("fr".into()),
        link: Some("https://example.org".into()),
        comment: Some("c1".into()),
        rev: Some(1),
    };
    assert_eq!(all.clone(), all);
}

fn note() {
    let n = model::Note { label: "river".into(), paras: vec![vec![model::Inline::plain("It runs north.")]] };
    assert_eq!((n.label.as_str(), n.paras.len()), ("river", 1));
}

fn fence() {
    for f in [model::Fence::Verse, model::Fence::Aside(None), model::Fence::Aside(Some("letter".into()))] {
        let _: Option<&str> = match &f {
            model::Fence::Verse => None,
            model::Fence::Aside(kind) => kind.as_deref(),
        };
    }
}

fn image_role() {
    for role in [model::ImageRole::Inline, model::ImageRole::Wide, model::ImageRole::Page] {
        let word: &'static str = match role {
            model::ImageRole::Inline => "inline",
            model::ImageRole::Wide => "wide",
            model::ImageRole::Page => "page",
        };
        // Book stores the word and reads it back.
        assert_eq!(role.key(), word);
        assert_eq!(model::ImageRole::from_key(word), Some(role));
    }
    assert_eq!(model::ImageRole::from_key("poster"), None);
    assert_eq!(model::ImageRole::default(), model::ImageRole::Inline);
}

fn text_functions() {
    let mut content: model::Inlines = vec![
        model::Inline::plain("The "),
        model::Inline::plain("old "),
        model::Inline::marked("harbour", model::Marks::italic()),
        model::Inline::plain(""),
    ];
    // What a reader sees of a paragraph: its words without their marks.
    let plain: String = model::text::plain(&content);
    assert_eq!(plain, "The old harbour");
    model::text::normalize_inlines(&mut content);
    assert_eq!(content, vec![model::Inline::plain("The old "), model::Inline::marked("harbour", model::Marks::italic())]);
    assert_eq!(model::text::plain(&content), plain, "the canonical form holds the same words");
}

fn words() {
    // The one word rule of every word-for-word check: split at white space, punctuation kept
    // on its word, a piece with no letter or digit is no word.
    let words: Vec<String> = model::words::words("“Hello,” she said — again.\tl’été 1867 …");
    assert_eq!(words, ["“Hello,”", "she", "said", "again.", "l’été", "1867"]);
    assert!(model::words::words("  — … ").is_empty());
}

fn series_relative() {
    // The path from one folder to another, as a project stores it: through their common
    // folder, and with `/` whatever the system.
    let rel: String = model::series::relative(Path::new("/books/One.wsbk"), Path::new("/books/drafts/One.wsm"));
    assert_eq!(rel, "../drafts/One.wsm");
    assert_eq!(model::series::relative(Path::new("/books/One.wsbk"), Path::new("/shared/One.wsm")), "../../shared/One.wsm");
}

/// The fixture project of schema 1 or 2, copied where a test may write.
fn schema_fixture(schema: u32, name: &str) -> PathBuf {
    fn copy(from: &Path, to: &Path) {
        std::fs::create_dir_all(to).unwrap();
        for e in std::fs::read_dir(from).unwrap().flatten() {
            if e.path().is_dir() {
                copy(&e.path(), &to.join(e.file_name()));
            } else {
                std::fs::copy(e.path(), to.join(e.file_name())).unwrap();
            }
        }
    }
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../../tests/fixtures/projects/schema-{schema}/The Lighthouse.wsm"));
    let dst = scratch(name).join("The Lighthouse.wsm");
    copy(&src, &dst);
    dst
}

fn schemas() {
    // The contract promises: schemas 1, 2 and 3 are read; 3 is written; a newer one is refused.
    let newest: u32 = format::SCHEMA;
    assert_eq!(newest, 3, "a new schema is a change to the contract: docs/book-handoff.md, \"Versions\"");
    for schema in [1, 2] {
        let root = schema_fixture(schema, &format!("schema-{schema}"));
        assert!(std::fs::read_to_string(root.join("project.toml")).unwrap().starts_with(&format!("schema = {schema}\n")));
        let (_, project) = format::folder::Folder::open(&root).unwrap();
        let view = format::book::from_project(&project);
        assert_eq!(view.metadata.info.title, "The Lighthouse");
        assert!(!view.body.is_empty(), "schema {schema}: the book view has the manuscript");
        // Packed, it reads the same.
        let packed = format::pack::pack_folder(&root).unwrap();
        assert_eq!(format::book::from_project(&format::pack::read_packed(&packed).unwrap()), view);
    }
    let root = scratch("schema-written").join("Book.wsm");
    format::folder::Folder::create(&root, &sample()).unwrap();
    for file in ["project.toml", "binder.toml"] {
        assert!(std::fs::read_to_string(root.join(file)).unwrap().starts_with("schema = 3\n"), "{file} says which schema it is, first");
    }
}

fn errors() {
    // A project saved by a newer Manuscript is refused, never guessed at, and the error says
    // both versions, so Book can word its own message.
    let root = scratch("newer").join("Book.wsm");
    format::folder::Folder::create(&root, &sample()).unwrap();
    let toml = std::fs::read_to_string(root.join("project.toml")).unwrap();
    std::fs::write(root.join("project.toml"), toml.replacen("schema = 3", "schema = 4", 1)).unwrap();
    let refused: format::Error = format::folder::Folder::open(&root).unwrap_err();
    assert_eq!(refused, format::Error::Newer { found: 4, supported: 3 });
    match &refused {
        format::Error::Newer { found, supported } => assert!(found > supported),
        other => panic!("{other}"),
    }
    let packed = format::pack::pack_folder(&root).and_then(|bytes| format::pack::read_packed(&bytes));
    assert_eq!(packed, Err(format::Error::Newer { found: 4, supported: 3 }), "packed, it is refused the same way");
    // Every error reads as a sentence for the person at the screen.
    let said: String = refused.to_string();
    assert!(said.contains("format 4") && said.contains("reads up to 3"), "{said}");
    let not_a_project: format::Error = format::folder::Folder::open(&scratch("empty")).unwrap_err();
    assert_eq!(not_a_project, format::Error::NotAProject);
    assert!(format::pack::read_packed(b"not a zip").is_err());
}

fn folder() {
    let root = scratch("folder").join("Book.wsm");
    let project = sample();
    // An owned source: Book makes the folder, and saves into it again.
    let made: format::folder::Folder = format::folder::Folder::create(&root, &project).unwrap();
    drop(made);
    assert!(format::folder::Folder::create(&root, &project).is_err(), "a folder that holds a project is not written over");
    let (mut folder, read): (format::folder::Folder, model::Project) = format::folder::Folder::open(&root).unwrap();
    assert_eq!(read, project, "what was saved reads back the same");
    // Opening writes nothing.
    assert!(folder.outside_changes().unwrap().is_empty());
    let mut changed = read.clone();
    changed.settings.book.subtitle = "A Story".into();
    folder.save(&changed).unwrap();
    assert_eq!(format::folder::Folder::open(&root).unwrap().1, changed);
    assert!(folder.outside_changes().unwrap().is_empty(), "its own save is not a change from outside");
    // A linked source: Manuscript saves while Book has the project open, and Book notices.
    let text = root.join(format!("text/{}.md", id(5)));
    std::fs::write(&text, "The sea was loud.\n").unwrap();
    let outside = folder.outside_changes().unwrap();
    assert_eq!(outside.len(), 1, "{outside:?}");
}

fn packed() {
    let root = scratch("packed").join("Book.wsm");
    let project = sample();
    format::folder::Folder::create(&root, &project).unwrap();
    std::fs::create_dir_all(root.join("media")).unwrap();
    std::fs::write(root.join(format!("media/{}.png", id(8))), [1u8, 2, 3]).unwrap();
    // A linked source packed into a bundle, and a packed project read.
    let bytes: Vec<u8> = format::pack::pack_folder(&root).unwrap();
    assert!(bytes.len() as u64 <= format::pack::MAX_PACKED);
    assert_eq!(format::pack::pack_folder(&root).unwrap(), bytes, "the same project gives the same bytes");
    let read: model::Project = format::pack::read_packed(&bytes).unwrap();
    assert_eq!(read, project);
    // The files inside, pictures included, by their path in the project.
    let files: std::collections::BTreeMap<String, Vec<u8>> = format::pack::unpack(&bytes).unwrap();
    assert_eq!(files.get(&format!("media/{}.png", id(8))), Some(&vec![1u8, 2, 3]));
    assert!(files.contains_key("project.toml") && files.contains_key("binder.toml"));
    let on_disk: std::collections::BTreeMap<String, Vec<u8>> = format::pack::folder_files(&root).unwrap();
    assert_eq!(on_disk, files, "the files of the folder are the files of the packed project");
}

fn book_view() {
    use format::book::{Asset, Book, FrontKind, Metadata, Section};
    let project = sample();
    let view: Book = format::book::from_project(&project);
    assert_eq!(view.version, format::book::VERSION);
    assert_eq!(format::book::VERSION, 1, "a new view version is a change to the contract: docs/book-handoff.md, \"Versions\"");
    let Metadata { info, byline } = &view.metadata;
    assert_eq!((info.title.as_str(), byline.as_str()), ("The Lighthouse", "A. Writer"));

    // Front matter, the body and back matter, in reading order; what is left out of export is
    // no section, and neither is a picture.
    let titles = |sections: &[Section]| sections.iter().map(|s| s.title.clone()).collect::<Vec<_>>();
    assert_eq!(titles(&view.front), ["Title Page"]);
    assert_eq!(titles(&view.body), ["The Rock"]);
    assert_eq!(titles(&view.back), ["Afterword"]);
    assert_eq!(view.front[0].kind, FrontKind::TitlePage);
    let part = &view.body[0];
    assert_eq!((part.id, part.role, part.number), (id(2), model::Role::Part, Some(1)));
    let chapter = &part.children[0];
    assert_eq!((chapter.id, chapter.role, chapter.number, chapter.kind), (id(3), model::Role::Chapter, Some(1), FrontKind::Other));
    assert_eq!(titles(&chapter.children), ["Dusk", "Night"]);
    let scene = &chapter.children[0];
    assert_eq!((scene.role, scene.number), (model::Role::Scene, None));

    // The content is the item's text in its canonical form, with its notes as notes.
    let model::Block::Para(content) = &scene.content[0] else { panic!("{:?}", scene.content) };
    assert_eq!(model::text::plain(content), "The keeper climbed the rock.", "a note reference is no word of the text");
    assert!(content.iter().any(|piece| matches!(piece, model::Inline::NoteRef { label, .. } if label == "1")));
    assert_eq!(scene.notes, vec![model::Note { label: "1".into(), paras: vec![vec![model::Inline::plain("At dusk.")]] }]);
    // A revision pass mark is the writer's working mark and is taken off. A comment's anchor
    // stays on the words it is on; the comment itself is not in the view (issue #8).
    let anchored: Vec<&model::Marks> = content.iter().map(model::Inline::marks).filter(|m| m.comment.is_some()).collect();
    assert_eq!(anchored.len(), 1);
    assert_eq!(anchored[0].comment.as_deref(), Some("c1"));
    assert!(content.iter().all(|piece| piece.marks().rev.is_none()));

    // Every id and role in reading order, the count of each role, and what Book needs that
    // the writer has not set.
    let outline: Vec<(model::ItemId, model::Role)> = format::book::outline(&view);
    assert_eq!(outline.iter().map(|(i, _)| *i).collect::<Vec<_>>(), [id(1), id(2), id(3), id(4), id(5), id(7)]);
    let counts: std::collections::BTreeMap<&'static str, usize> = format::book::counts(&view);
    assert_eq!(counts.values().sum::<usize>(), outline.len());
    let missing: Vec<&'static str> = format::book::missing_metadata(&view);
    assert!(missing.contains(&"description") && !missing.contains(&"title"), "{missing:?}");

    // The pictures the project holds, by their path inside it.
    let Asset { id: picture, path, title } = &view.assets[0];
    assert_eq!((*picture, path.as_str(), title.as_str()), (id(8), format!("media/{}.png", id(8)).as_str(), "Map"));

    // Built field by field, as Book builds a box set's view and a section of its own.
    let section = Section {
        id: id(40),
        role: model::Role::Chapter,
        kind: FrontKind::Other,
        title: "Made by Book".into(),
        number: Some(1),
        content: vec![model::Block::SceneBreak],
        notes: Vec::new(),
        children: Vec::new(),
    };
    let own = Book {
        version: view.version,
        metadata: Metadata { info: view.metadata.info.clone(), byline: view.metadata.byline.clone() },
        front: Vec::new(),
        body: vec![section.clone()],
        back: Vec::new(),
        assets: vec![Asset { id: id(41), path: "media/cover.png".into(), title: "Cover".into() }],
    };
    assert_eq!(own.body[0], section);
    assert_eq!(own.clone(), own);
    // The view of a project is the same every time, and the same from a folder or packed.
    assert_eq!(format::book::from_project(&project), view);
}

fn front_kind() {
    use format::book::FrontKind;
    // Matched in full: a new kind of front or back matter page is one Book must place.
    let all = [
        FrontKind::TitlePage,
        FrontKind::Copyright,
        FrontKind::Dedication,
        FrontKind::Epigraph,
        FrontKind::Contents,
        FrontKind::Acknowledgements,
        FrontKind::AboutTheAuthor,
        FrontKind::AlsoBy,
        FrontKind::Other,
    ];
    for kind in all {
        let _: u8 = match kind {
            FrontKind::TitlePage => 0,
            FrontKind::Copyright => 1,
            FrontKind::Dedication => 2,
            FrontKind::Epigraph => 3,
            FrontKind::Contents => 4,
            FrontKind::Acknowledgements => 5,
            FrontKind::AboutTheAuthor => 6,
            FrontKind::AlsoBy => 7,
            FrontKind::Other => 8,
        };
    }
}

fn offers() {
    use format::offers::{self, Decision, Offer, OffersFile};
    // Beside the project folder, never in it.
    let path: PathBuf = offers::offers_path(Path::new("/w/My Novel.wsm"));
    assert_eq!(path, Path::new("/w/My Novel.wsm-offers.json"));
    // Built field by field.
    let offer =
        Offer { id: "a".into(), item: id(4), find: "teh rock".into(), replace: "the rock".into(), note: "typo".into(), made: "2026-11-24".into() };
    let file = OffersFile { version: offers::VERSION, offers: vec![offer.clone()] };
    assert_eq!(offers::VERSION, 1, "a new offers file version is a change to the contract");
    let bytes: Vec<u8> = offers::write(&file).unwrap();
    assert!(bytes.len() as u64 <= offers::MAX_FILE);
    let back: Result<OffersFile, String> = offers::read(&bytes);
    assert_eq!(back, Ok(file));
    // The writer's decisions are kept in the project; an offer with none is still waiting.
    let mut project = sample();
    assert!(offers::decisions(&project).is_empty());
    let decided: Result<(), String> = offers::decide(&mut project, &offer.id, Decision::Accepted);
    assert!(decided.is_ok());
    offers::decide(&mut project, "b", Decision::Rejected).unwrap();
    let decisions: std::collections::BTreeMap<String, Decision> = offers::decisions(&project);
    assert_eq!(decisions.get("a"), Some(&Decision::Accepted));
    assert_eq!(decisions.get("b"), Some(&Decision::Rejected));
    for decision in [Decision::Accepted, Decision::Rejected] {
        let _: bool = match decision {
            Decision::Accepted => true,
            Decision::Rejected => false,
        };
    }
}

#[test]
fn every_item_of_the_contract_is_used() {
    for (names, use_them) in SURFACE {
        assert!(!names.is_empty());
        use_them();
    }
    let names = listed();
    assert!(names.iter().all(|name| name.starts_with("model::") || name.starts_with("format::")), "{names:?}");
    assert_eq!(names.iter().collect::<BTreeSet<_>>().len(), names.len(), "an item is listed twice");
}

/// The items `docs/book-handoff.md` lists: the names in backticks that open a table row
/// between the two marker lines.
fn documented() -> BTreeSet<String> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/book-handoff.md");
    let text = std::fs::read_to_string(&path).unwrap();
    let begin = text.find("<!-- BEGIN contract surface").expect("the BEGIN marker line");
    let end = text.find("<!-- END contract surface -->").expect("the END marker line");
    let mut out = BTreeSet::new();
    for line in text[begin..end].lines().filter(|l| l.starts_with("| `")) {
        let cell = line.trim_start_matches("| ").split(" | ").next().unwrap_or_default();
        // A cell may name several items: each in its own backticks.
        for name in cell.split('`').skip(1).step_by(2) {
            assert!(out.insert(name.to_owned()), "docs/book-handoff.md lists {name} twice");
        }
    }
    out
}

#[test]
fn the_document_and_this_list_name_the_same_items() {
    let documented = documented();
    let used: BTreeSet<String> = listed().into_iter().map(str::to_owned).collect();
    let only_documented: Vec<&String> = documented.difference(&used).collect();
    let only_used: Vec<&String> = used.difference(&documented).collect();
    assert!(
        only_documented.is_empty(),
        "docs/book-handoff.md lists items this test does not use: {only_documented:?}. Add a use of each to SURFACE, so a change to it fails here."
    );
    assert!(
        only_used.is_empty(),
        "this test uses items docs/book-handoff.md does not list: {only_used:?}. List each in the document, between the contract surface markers."
    );
    assert!(documented.len() > 60, "the list was read: {} items", documented.len());
}
