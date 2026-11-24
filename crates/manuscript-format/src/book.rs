// SPDX-License-Identifier: MIT OR Apache-2.0
//! The book view (plan section 9.2, `docs/book-handoff.md`): a read-only view of a project for
//! Wordsmith Book. Book opens a project folder or a packed `.wsmz` through this module only, at a
//! release tag of this repository, and never writes the project.
//!
//! What it gives: the book's metadata; its structure (front matter, parts, chapters and scenes,
//! back matter), each with its role, title, number and stable id; the content as semantic text
//! (the text dialect's blocks and marks, notes kept as notes); and the assets.
//!
//! The view is versioned ([`VERSION`]): a change Book must follow raises it and is a minor
//! release of this repository with a "What Book must do" note.

use std::collections::BTreeMap;
use std::path::Path;

use wordsmith_manuscript_model::{Area, Block, BookInfo, Item, ItemId, Kind, Note, Project, Role, Text};

use crate::Error;

/// The book view's version. Book declares which versions it reads.
pub const VERSION: u32 = 1;

/// The book's metadata.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Metadata {
    /// Title, subtitle, author (legal name), pen name, series, number, language, ISBNs per
    /// edition, publisher, copyright year, rights, description, subjects and keywords, as the
    /// writer set them.
    pub info: BookInfo,
    /// The name the book is published under.
    pub byline: String,
}

/// What a front matter item is, from its title.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrontKind {
    /// The title page.
    TitlePage,
    /// The copyright page.
    Copyright,
    /// A dedication.
    Dedication,
    /// An epigraph.
    Epigraph,
    /// The table of contents.
    Contents,
    /// Acknowledgements.
    Acknowledgements,
    /// About the author.
    AboutTheAuthor,
    /// A list of the author's other books.
    AlsoBy,
    /// Anything else.
    Other,
}

impl FrontKind {
    /// The kind a title names.
    pub fn of(title: &str) -> FrontKind {
        let t = title.trim().to_lowercase();
        match t.as_str() {
            "title page" | "title" => FrontKind::TitlePage,
            "copyright" | "copyright page" => FrontKind::Copyright,
            "dedication" => FrontKind::Dedication,
            "epigraph" => FrontKind::Epigraph,
            "contents" | "table of contents" => FrontKind::Contents,
            "acknowledgements" | "acknowledgments" => FrontKind::Acknowledgements,
            "about the author" => FrontKind::AboutTheAuthor,
            "also by" => FrontKind::AlsoBy,
            _ if t.starts_with("also by") => FrontKind::AlsoBy,
            _ => FrontKind::Other,
        }
    }
}

/// One section of the book: a part, chapter, scene, a front or back matter page, or a plain
/// item, with its content and the sections under it.
#[derive(Clone, Debug, PartialEq)]
pub struct Section {
    /// The stable id (Book keys its own layer by it).
    pub id: ItemId,
    /// The role.
    pub role: Role,
    /// For front and back matter: what it is.
    pub kind: FrontKind,
    /// The title.
    pub title: String,
    /// The number: parts and chapters are numbered in order, each counting on its own.
    pub number: Option<u32>,
    /// The content: blocks of the text dialect.
    pub content: Vec<Block>,
    /// The notes the content refers to (footnote or endnote is Book's choice).
    pub notes: Vec<Note>,
    /// The sections under it.
    pub children: Vec<Section>,
}

/// A file the book uses (pictures, the cover).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Asset {
    /// The id of the item that holds it.
    pub id: ItemId,
    /// Its path inside the project (`media/<id>.png`).
    pub path: String,
    /// Its title.
    pub title: String,
}

/// The book view of a project.
#[derive(Clone, Debug, PartialEq)]
pub struct Book {
    /// The view's version ([`VERSION`]).
    pub version: u32,
    /// Metadata.
    pub metadata: Metadata,
    /// Front matter, in order.
    pub front: Vec<Section>,
    /// Parts, chapters and the rest of the body, in order.
    pub body: Vec<Section>,
    /// Back matter, in order.
    pub back: Vec<Section>,
    /// Pictures and other files the project holds.
    pub assets: Vec<Asset>,
}

/// Every id the book view gives, with its role, in reading order (for checks).
pub fn outline(book: &Book) -> Vec<(ItemId, Role)> {
    fn walk(s: &Section, out: &mut Vec<(ItemId, Role)>) {
        out.push((s.id, s.role));
        for c in &s.children {
            walk(c, out);
        }
    }
    let mut out = Vec::new();
    for s in book.front.iter().chain(&book.body).chain(&book.back) {
        walk(s, &mut out);
    }
    out
}

/// The book's words in reading order: each section's title when it is a part or chapter, then
/// its content (notes left out).
pub fn words(book: &Book) -> Vec<String> {
    fn walk(s: &Section, out: &mut Vec<String>) {
        if matches!(s.role, Role::Part | Role::Chapter) {
            out.extend(wordsmith_manuscript_model::words::words(&s.title));
        }
        let t = Text { blocks: s.content.clone(), notes: Vec::new() };
        out.extend(t.plain_paragraphs().iter().flat_map(|p| wordsmith_manuscript_model::words::words(p)));
        for c in &s.children {
            walk(c, out);
        }
    }
    let mut out = Vec::new();
    for s in book.front.iter().chain(&book.body).chain(&book.back) {
        walk(s, &mut out);
    }
    out
}

struct Counter {
    parts: u32,
    chapters: u32,
}

fn section(p: &Project, item: &Item, n: &mut Counter) -> Option<Section> {
    if !item.include || matches!(item.kind, Kind::Screenplay | Kind::Media | Kind::Link | Kind::Entry) {
        return None;
    }
    let number = match item.role {
        Role::Part => {
            n.parts += 1;
            Some(n.parts)
        }
        Role::Chapter => {
            n.chapters += 1;
            Some(n.chapters)
        }
        _ => None,
    };
    let mut text = p.text(item.id).map(|t| t.body.clone()).unwrap_or_default();
    // Revision pass marks are the writer's working marks, not part of the book.
    for run in text.inline_runs_mut() {
        for i in run.iter_mut() {
            match i {
                wordsmith_manuscript_model::Inline::Span { marks, .. } | wordsmith_manuscript_model::Inline::NoteRef { marks, .. } => {
                    marks.rev = None
                }
            }
        }
    }
    text.normalize();
    let kind = if matches!(item.role, Role::FrontMatter | Role::BackMatter) { FrontKind::of(&item.title) } else { FrontKind::Other };
    Some(Section {
        id: item.id,
        role: item.role,
        kind,
        title: item.title.clone(),
        number,
        content: text.blocks,
        notes: text.notes,
        children: item.children.iter().filter_map(|c| section(p, c, n)).collect(),
    })
}

/// The book view of a project in memory: of the draft chosen for export (DRF-1).
pub fn from_project(p: &Project) -> Book {
    let exported = wordsmith_manuscript_model::drafts::exported(p);
    let p = &exported;
    let mut n = Counter { parts: 0, chapters: 0 };
    let mut front = Vec::new();
    let mut body = Vec::new();
    let mut back = Vec::new();
    for item in p.binder.area(Area::Manuscript) {
        // A folder of front or back matter (role none, children with the role) is unwrapped.
        let all_front = !item.children.is_empty() && item.role == Role::None && item.children.iter().all(|c| c.role == Role::FrontMatter);
        let all_back = !item.children.is_empty() && item.role == Role::None && item.children.iter().all(|c| c.role == Role::BackMatter);
        if (all_front || all_back) && p.text(item.id).is_none_or(|t| t.body.is_empty()) {
            if !item.include {
                continue;
            }
            for c in &item.children {
                if let Some(s) = section(p, c, &mut n) {
                    if all_front { front.push(s) } else { back.push(s) }
                }
            }
            continue;
        }
        let Some(s) = section(p, item, &mut n) else { continue };
        match item.role {
            Role::FrontMatter => front.push(s),
            Role::BackMatter => back.push(s),
            _ => body.push(s),
        }
    }
    let assets = p
        .binder
        .rows()
        .iter()
        .filter(|r| r.item.kind == Kind::Media && r.area != Area::Trash)
        .filter_map(|r| crate::files::path_of(r.item).map(|path| Asset { id: r.item.id, path, title: r.item.title.clone() }))
        .collect();
    let info = p.settings.book.clone();
    Book { version: VERSION, metadata: Metadata { byline: info.byline().to_owned(), info }, front, body, back, assets }
}

/// Open a project folder as a book.
pub fn open_folder(root: &Path) -> Result<Book, Error> {
    crate::folder::Folder::open(root).map(|(_, p)| from_project(&p))
}

/// Open a packed project (`.wsmz`) as a book, in memory.
pub fn open_packed(bytes: &[u8]) -> Result<Book, Error> {
    crate::pack::read_packed(bytes).map(|p| from_project(&p))
}

/// The metadata Book needs that the writer has not set yet, by name.
pub fn missing_metadata(book: &Book) -> Vec<&'static str> {
    let i = &book.metadata.info;
    let mut out = Vec::new();
    for (name, empty) in [
        ("title", i.title.trim().is_empty() || i.title == "Untitled"),
        ("author or pen name", book.metadata.byline.trim().is_empty()),
        ("language", i.language.trim().is_empty()),
        ("copyright year", i.copyright_year.is_none()),
        ("rights statement", i.rights.trim().is_empty()),
        ("description", i.description.trim().is_empty()),
    ] {
        if empty {
            out.push(name);
        }
    }
    out
}

/// How many sections of each role the book has (for reports).
pub fn counts(book: &Book) -> BTreeMap<&'static str, usize> {
    let mut out = BTreeMap::new();
    for (_, r) in outline(book) {
        *out.entry(r.title()).or_default() += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn front_matter_kinds_by_title() {
        assert_eq!(FrontKind::of("Dedication"), FrontKind::Dedication);
        assert_eq!(FrontKind::of("Table of Contents"), FrontKind::Contents);
        assert_eq!(FrontKind::of("Also by the Author"), FrontKind::AlsoBy);
        assert_eq!(FrontKind::of("Map"), FrontKind::Other);
    }
}
