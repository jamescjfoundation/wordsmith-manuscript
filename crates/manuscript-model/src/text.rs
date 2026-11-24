// SPDX-License-Identifier: MIT OR Apache-2.0
//! The semantic text of one item: what the text dialect (`docs/format.md`) can hold, and nothing
//! else. Styling is semantic only ("italic", "scene break", "verse"), never fonts or sizes.
//!
//! A [`Text`] is kept in its *canonical* form ([`Text::normalize`]): neighbouring spans with the
//! same marks are merged and empty spans dropped. The format writes canonical text and reads it
//! back exactly, so model → text → model is the identity on canonical texts.

use serde::{Deserialize, Serialize};

/// Character-level marks on a span of text.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Marks {
    /// Italic (emphasis).
    #[serde(default, skip_serializing_if = "is_false")]
    pub italic: bool,
    /// Bold (strong emphasis).
    #[serde(default, skip_serializing_if = "is_false")]
    pub bold: bool,
    /// Small capitals.
    #[serde(default, skip_serializing_if = "is_false")]
    pub small_caps: bool,
    /// Strikethrough.
    #[serde(default, skip_serializing_if = "is_false")]
    pub strike: bool,
    /// A language tag (BCP 47, such as `fr` or `de-CH`) for text in another language.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lang: Option<String>,
    /// A link target: `wsm:<id>` for a binder item or bible entry, or a web address.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub link: Option<String>,
    /// The id of the comment anchored to this text (the comment is in the item's metadata).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
}

fn is_false(b: &bool) -> bool {
    !*b
}

impl Marks {
    /// No marks at all.
    pub fn is_plain(&self) -> bool {
        *self == Marks::default()
    }

    /// Plain marks with italic set.
    pub fn italic() -> Marks {
        Marks { italic: true, ..Marks::default() }
    }

    /// Plain marks with bold set.
    pub fn bold() -> Marks {
        Marks { bold: true, ..Marks::default() }
    }
}

/// A piece of a paragraph.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Inline {
    /// Text with marks. A `\n` inside the text is a line break within the paragraph (verse lines,
    /// addresses); it never starts a new paragraph.
    Span {
        /// The words, exactly as typed.
        text: String,
        /// The marks on all of it.
        marks: Marks,
    },
    /// A reference to a note of this text, by label (`[^1]`).
    NoteRef {
        /// The note's label.
        label: String,
        /// Marks around the reference (a link or comment over it, for example).
        marks: Marks,
    },
}

impl Inline {
    /// A plain span.
    pub fn plain(text: impl Into<String>) -> Inline {
        Inline::Span { text: text.into(), marks: Marks::default() }
    }

    /// A span with marks.
    pub fn marked(text: impl Into<String>, marks: Marks) -> Inline {
        Inline::Span { text: text.into(), marks }
    }

    /// The marks on this piece.
    pub fn marks(&self) -> &Marks {
        match self {
            Inline::Span { marks, .. } | Inline::NoteRef { marks, .. } => marks,
        }
    }
}

/// The content of one paragraph.
pub type Inlines = Vec<Inline>;

/// The kind of a fenced block.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Fence {
    /// Verse: line breaks inside a paragraph are kept; paragraphs are stanzas.
    Verse,
    /// An aside such as a letter, a note or a message; the word is the aside's kind, if any.
    Aside(Option<String>),
}

/// How an image sits in the text.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ImageRole {
    /// In the flow of the text, at its own size.
    #[default]
    Inline,
    /// As wide as the text block.
    Wide,
    /// A page of its own.
    Page,
}

impl ImageRole {
    /// The role's word in the dialect.
    pub fn key(self) -> &'static str {
        match self {
            ImageRole::Inline => "inline",
            ImageRole::Wide => "wide",
            ImageRole::Page => "page",
        }
    }

    /// The role for a word.
    pub fn from_key(k: &str) -> Option<ImageRole> {
        match k {
            "inline" => Some(ImageRole::Inline),
            "wide" => Some(ImageRole::Wide),
            "page" => Some(ImageRole::Page),
            _ => None,
        }
    }
}

/// A block of the text.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Block {
    /// A body paragraph (possibly empty).
    Para(Inlines),
    /// A heading, level 1 to 3, used only where an item's text needs it (front matter pages).
    Heading {
        /// 1 to 3.
        level: u8,
        /// The heading's text.
        content: Inlines,
    },
    /// A scene break. Its printed symbol is an export setting.
    SceneBreak,
    /// A block quote of one or more paragraphs.
    Quote(Vec<Inlines>),
    /// A fenced block: verse or an aside, of one or more paragraphs.
    Fenced {
        /// Verse or an aside.
        kind: Fence,
        /// The paragraphs (stanzas, for verse).
        paras: Vec<Inlines>,
    },
    /// A picture from the project's media.
    Image {
        /// The alternative text.
        alt: String,
        /// The path inside the project (`media/<id>.png`).
        src: String,
        /// How it sits in the text.
        role: ImageRole,
    },
    /// Content the dialect cannot express, kept for one target format.
    Raw {
        /// The target format (`html`, `latex`, ...).
        format: String,
        /// The content, exactly.
        text: String,
    },
}

/// A note of the text (a footnote or endnote: which one is decided at export or in Book).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Note {
    /// The label used by references (`1`, `a`, `river`).
    pub label: String,
    /// The note's paragraphs.
    pub paras: Vec<Inlines>,
}

/// The text of one item: blocks in order, then its notes.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Text {
    /// The blocks, in reading order.
    pub blocks: Vec<Block>,
    /// The notes referred to from the blocks.
    pub notes: Vec<Note>,
}

impl Text {
    /// An empty text.
    pub fn new() -> Text {
        Text::default()
    }

    /// A text of plain body paragraphs.
    pub fn from_paragraphs<S: AsRef<str>>(paras: &[S]) -> Text {
        let blocks = paras.iter().map(|p| Block::Para(vec![Inline::plain(p.as_ref())])).collect();
        let mut t = Text { blocks, notes: Vec::new() };
        t.normalize();
        t
    }

    /// Whether the text has no blocks and no notes.
    pub fn is_empty(&self) -> bool {
        self.blocks.is_empty() && self.notes.is_empty()
    }

    /// Put the text in canonical form: merge neighbouring spans with equal marks, drop empty
    /// spans, and make empty language, link and comment values absent.
    pub fn normalize(&mut self) {
        for block in &mut self.blocks {
            match block {
                Block::Para(c) | Block::Heading { content: c, .. } => normalize_inlines(c),
                Block::Quote(ps) | Block::Fenced { paras: ps, .. } => ps.iter_mut().for_each(normalize_inlines),
                Block::SceneBreak | Block::Image { .. } | Block::Raw { .. } => {}
            }
        }
        for note in &mut self.notes {
            note.paras.iter_mut().for_each(normalize_inlines);
        }
    }

    /// The paragraphs of the text in reading order, each as its plain text: blocks first, then
    /// notes. Scene breaks, images and raw blocks have no words and are skipped.
    pub fn plain_paragraphs(&self) -> Vec<String> {
        let mut out = Vec::new();
        for block in &self.blocks {
            match block {
                Block::Para(c) | Block::Heading { content: c, .. } => out.push(plain(c)),
                Block::Quote(ps) | Block::Fenced { paras: ps, .. } => out.extend(ps.iter().map(|p| plain(p))),
                Block::SceneBreak | Block::Image { .. } | Block::Raw { .. } => {}
            }
        }
        for note in &self.notes {
            out.extend(note.paras.iter().map(|p| plain(p)));
        }
        out
    }

    /// The words of the text in reading order (see [`crate::words`]).
    pub fn words(&self) -> Vec<String> {
        self.plain_paragraphs().iter().flat_map(|p| crate::words::words(p)).collect()
    }

    /// How many words the text has.
    pub fn word_count(&self) -> usize {
        self.plain_paragraphs().iter().map(|p| crate::words::count(p)).sum()
    }
}

/// The plain text of a paragraph's content (note references count as nothing).
pub fn plain(content: &[Inline]) -> String {
    let mut s = String::new();
    for i in content {
        if let Inline::Span { text, .. } = i {
            s.push_str(text);
        }
    }
    s
}

fn clean(marks: &mut Marks) {
    for v in [&mut marks.lang, &mut marks.link, &mut marks.comment] {
        if v.as_deref() == Some("") {
            *v = None;
        }
    }
}

/// The object replacement character, which stands for embedded objects in documents and is never
/// part of a text.
pub const OBJECT: char = '\u{FFFC}';

/// Merge neighbouring spans with equal marks, drop empty spans, a note reference repeating the one
/// right before it, and object replacement characters.
pub fn normalize_inlines(content: &mut Inlines) {
    let mut out: Inlines = Vec::with_capacity(content.len());
    for mut inline in content.drain(..) {
        if matches!(inline, Inline::NoteRef { .. }) && out.last() == Some(&inline) {
            continue;
        }
        match &mut inline {
            Inline::Span { text, marks } => {
                clean(marks);
                if text.contains(OBJECT) {
                    text.retain(|c| c != OBJECT);
                }
                if text.is_empty() {
                    continue;
                }
                if let Some(Inline::Span { text: prev, marks: pm }) = out.last_mut()
                    && pm == marks
                {
                    prev.push_str(text);
                    continue;
                }
            }
            Inline::NoteRef { marks, .. } => clean(marks),
        }
        out.push(inline);
    }
    *content = out;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_merges_and_drops() {
        let mut c = vec![
            Inline::plain("a"),
            Inline::plain(""),
            Inline::plain("b"),
            Inline::marked("c", Marks::italic()),
            Inline::marked("d", Marks::italic()),
        ];
        normalize_inlines(&mut c);
        assert_eq!(c, vec![Inline::plain("ab"), Inline::marked("cd", Marks::italic())]);
    }

    #[test]
    fn empty_values_are_absent() {
        let mut c = vec![Inline::marked("x", Marks { lang: Some(String::new()), ..Marks::default() })];
        normalize_inlines(&mut c);
        assert_eq!(c, vec![Inline::plain("x")]);
    }

    #[test]
    fn words_skip_breaks_and_include_notes() {
        let t = Text {
            blocks: vec![Block::Para(vec![Inline::plain("One two")]), Block::SceneBreak, Block::Quote(vec![vec![Inline::plain("three")]])],
            notes: vec![Note { label: "1".into(), paras: vec![vec![Inline::plain("four")]] }],
        };
        assert_eq!(t.words(), vec!["One", "two", "three", "four"]);
        assert_eq!(t.word_count(), 4);
    }
}
