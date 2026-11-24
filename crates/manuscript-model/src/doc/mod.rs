// SPDX-License-Identifier: MIT OR Apache-2.0
//! An item's text as a `jcj_doc::Document`, for editing with the core's engine, and back.
//!
//! Manuscript's styles live on the document as typed extension data in the `manuscript.`
//! namespace (core decision 0006): a paragraph's block kind ([`BLOCK`]), and a run's link,
//! comment and note reference. Italic, bold, small caps, strikethrough and language use the
//! document's own character properties, so the core's commands, spelling and exports see them.
//! Only what the text dialect can hold is read back: any other formatting (a font, a color, an
//! underline pasted in) is dropped, and the words always stay.

use jcj_doc::{CharProps, Document, ExtKey, Paragraph};
use serde::{Deserialize, Serialize};

use crate::text::normalize_inlines;
use crate::{Block, Fence, ImageRole, Inline, Inlines, Marks, Note, Text};

/// What kind of block a paragraph belongs to. Absent means a body paragraph.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum BlockTag {
    /// A body paragraph (written only where it must be explicit).
    Body,
    /// A heading of level 1 to 3.
    Heading {
        /// 1 to 3.
        level: u8,
    },
    /// A scene break (the paragraph's text is empty).
    SceneBreak,
    /// A paragraph of a block quote; paragraphs with the same group are one quote.
    Quote {
        /// The quote it belongs to.
        group: u32,
    },
    /// A stanza of verse; the same group is one verse block.
    Verse {
        /// The verse block it belongs to.
        group: u32,
    },
    /// A paragraph of an aside.
    Aside {
        /// The aside it belongs to.
        group: u32,
        /// The aside's kind (`letter`).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        aside: Option<String>,
    },
    /// A verse or aside block with no paragraphs (the paragraph itself holds no text).
    EmptyFence {
        /// Verse, else an aside.
        verse: bool,
        /// The aside's kind.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        aside: Option<String>,
        /// The block it is.
        group: u32,
    },
    /// A picture.
    Image {
        /// Alternative text.
        alt: String,
        /// The path in the project.
        src: String,
        /// How it sits.
        role: ImageRole,
    },
    /// A raw block.
    Raw {
        /// The target format.
        format: String,
        /// The content.
        text: String,
    },
    /// A paragraph of a note; the same group is one note.
    Note {
        /// The note's label.
        label: String,
        /// The note it belongs to.
        group: u32,
    },
    /// A note with no paragraphs.
    EmptyNote {
        /// The note's label.
        label: String,
        /// The note it is.
        group: u32,
    },
}

/// A paragraph's block kind.
pub const BLOCK: ExtKey<BlockTag> = ExtKey::new("manuscript.block");
/// A run's link target (`wsm:<id>` or a web address).
pub const LINK: ExtKey<String> = ExtKey::new("manuscript.link");
/// The id of the comment anchored to a run.
pub const COMMENT: ExtKey<String> = ExtKey::new("manuscript.comment");
/// A run that stands for a note reference (its text is the label, shown raised).
pub const NOTE_REF: ExtKey<String> = ExtKey::new("manuscript.note");
/// On a document: it stands for an empty text (its one paragraph is only there to type into).
pub const BLANK: ExtKey<bool> = ExtKey::new("manuscript.blank");

/// The character properties for a set of marks.
pub fn props_of(m: &Marks) -> CharProps {
    let mut p = CharProps::default();
    let on = |b: bool| b.then_some(true);
    p.italic = on(m.italic);
    p.bold = on(m.bold);
    p.small_caps = on(m.small_caps);
    p.strike = on(m.strike);
    p.lang.clone_from(&m.lang);
    if let Some(l) = &m.link {
        let _ = LINK.set(&mut p, l);
    }
    if let Some(c) = &m.comment {
        let _ = COMMENT.set(&mut p, c);
    }
    p
}

/// The marks the dialect keeps from a run's properties.
pub fn marks_of(p: &CharProps) -> Marks {
    Marks {
        italic: p.italic == Some(true),
        bold: p.bold == Some(true),
        small_caps: p.small_caps == Some(true),
        strike: p.strike == Some(true),
        lang: p.lang.clone().filter(|l| !l.is_empty()),
        link: LINK.get_or_none(p),
        comment: COMMENT.get_or_none(p),
    }
}

fn paragraph(content: &[Inline], tag: Option<&BlockTag>) -> Paragraph {
    let mut p = Paragraph::new();
    for inline in content {
        let (text, props) = match inline {
            Inline::Span { text, marks } => (text.clone(), props_of(marks)),
            Inline::NoteRef { label, marks } => {
                let mut props = props_of(marks);
                let _ = NOTE_REF.set(&mut props, label);
                (label.clone(), props)
            }
        };
        if text.is_empty() {
            continue;
        }
        let end = p.text.len();
        let _ = p.insert_text(end, &text, &props);
    }
    p.normalize();
    if let Some(t) = tag {
        let _ = BLOCK.set(&mut p, t);
    }
    p
}

/// The editable document for a text.
pub fn to_document(text: &Text) -> Document {
    let mut doc = jcj_doc::new_document();
    let mut paras: Vec<Paragraph> = Vec::new();
    let mut group = 0u32;
    for block in &text.blocks {
        group += 1;
        match block {
            Block::Para(c) => paras.push(paragraph(c, None)),
            Block::Heading { level, content } => paras.push(paragraph(content, Some(&BlockTag::Heading { level: *level }))),
            Block::SceneBreak => paras.push(paragraph(&[], Some(&BlockTag::SceneBreak))),
            Block::Quote(ps) => paras.extend(ps.iter().map(|p| paragraph(p, Some(&BlockTag::Quote { group })))),
            Block::Fenced { kind, paras: ps } => {
                if ps.is_empty() {
                    let (verse, aside) = match kind {
                        Fence::Verse => (true, None),
                        Fence::Aside(a) => (false, a.clone()),
                    };
                    paras.push(paragraph(&[], Some(&BlockTag::EmptyFence { verse, aside, group })));
                }
                for p in ps {
                    let tag = match kind {
                        Fence::Verse => BlockTag::Verse { group },
                        Fence::Aside(a) => BlockTag::Aside { group, aside: a.clone() },
                    };
                    paras.push(paragraph(p, Some(&tag)));
                }
            }
            Block::Image { alt, src, role } => paras.push(paragraph(&[], Some(&BlockTag::Image { alt: alt.clone(), src: src.clone(), role: *role }))),
            Block::Raw { format, text } => paras.push(paragraph(&[], Some(&BlockTag::Raw { format: format.clone(), text: text.clone() }))),
        }
    }
    for note in &text.notes {
        group += 1;
        if note.paras.is_empty() {
            paras.push(paragraph(&[], Some(&BlockTag::EmptyNote { label: note.label.clone(), group })));
        }
        for p in &note.paras {
            paras.push(paragraph(p, Some(&BlockTag::Note { label: note.label.clone(), group })));
        }
    }
    if paras.is_empty() {
        paras.push(Paragraph::new());
        let _ = BLANK.set(&mut doc, &true);
    }
    doc.body = paras.into_iter().map(|p| std::sync::Arc::new(jcj_doc::Block::Para(p))).collect();
    doc
}

/// A paragraph's content as inlines (only the dialect's marks).
pub fn inlines_of(p: &Paragraph) -> Inlines {
    let mut out: Inlines = Vec::new();
    let mut last_note: Option<(usize, String, Marks)> = None;
    for (range, props) in p.run_ranges() {
        let Some(text) = p.text.get(range.clone()) else { continue };
        let marks = marks_of(props);
        if let Some(label) = NOTE_REF.get_or_none(props) {
            // One reference per run of the same note, whatever its text became.
            if last_note.as_ref().is_some_and(|(end, l, m)| *end == range.start && *l == label && *m == marks) {
                last_note = Some((range.end, label, marks));
                continue;
            }
            out.push(Inline::NoteRef { label: label.clone(), marks: marks.clone() });
            last_note = Some((range.end, label, marks));
            continue;
        }
        last_note = None;
        out.push(Inline::Span { text: text.replace(jcj_doc::para::OBJ, ""), marks });
    }
    normalize_inlines(&mut out);
    out
}

/// Every paragraph of a document's main text, tables' cells included, in reading order.
fn paragraphs(doc: &Document) -> Vec<&Paragraph> {
    let mut out = Vec::new();
    for b in &doc.body {
        match &**b {
            jcj_doc::Block::Para(p) => out.push(p),
            jcj_doc::Block::Table(t) => {
                for row in &t.rows {
                    for cell in &row.cells {
                        for cb in &cell.blocks {
                            if let jcj_doc::Block::Para(p) = &**cb {
                                out.push(p);
                            }
                        }
                    }
                }
            }
        }
    }
    out
}

/// The text a document holds.
pub fn from_document(doc: &Document) -> Text {
    let paras = paragraphs(doc);
    if BLANK.get_or_none(doc) == Some(true) && paras.len() == 1 && paras.first().is_some_and(|p| p.text.is_empty() && !BLOCK.is_set(*p)) {
        return Text::new();
    }
    let mut text = Text::new();
    // The open multi-paragraph block: (tag kind, group) and its paragraphs.
    let mut open: Option<(BlockTag, Vec<Inlines>)> = None;
    let flush = |open: &mut Option<(BlockTag, Vec<Inlines>)>, text: &mut Text| {
        if let Some((tag, ps)) = open.take() {
            match tag {
                BlockTag::Quote { .. } => text.blocks.push(Block::Quote(ps)),
                BlockTag::Verse { .. } => text.blocks.push(Block::Fenced { kind: Fence::Verse, paras: ps }),
                BlockTag::Aside { aside, .. } => text.blocks.push(Block::Fenced { kind: Fence::Aside(aside), paras: ps }),
                BlockTag::Note { label, .. } => text.notes.push(Note { label, paras: ps }),
                _ => {}
            }
        }
    };
    for p in paras {
        let tag = BLOCK.get_or_none(p).unwrap_or(BlockTag::Body);
        let content = inlines_of(p);
        let same = match (&open, &tag) {
            (Some((BlockTag::Quote { group: a }, _)), BlockTag::Quote { group: b }) => a == b,
            (Some((BlockTag::Verse { group: a }, _)), BlockTag::Verse { group: b }) => a == b,
            (Some((BlockTag::Aside { group: a, aside: x }, _)), BlockTag::Aside { group: b, aside: y }) => a == b && x == y,
            (Some((BlockTag::Note { group: a, label: x }, _)), BlockTag::Note { group: b, label: y }) => a == b && x == y,
            _ => false,
        };
        if same {
            if let Some((_, ps)) = open.as_mut() {
                ps.push(content);
            }
            continue;
        }
        flush(&mut open, &mut text);
        match tag {
            BlockTag::Body => text.blocks.push(Block::Para(content)),
            BlockTag::Heading { level } => text.blocks.push(Block::Heading { level, content }),
            // A scene break, picture or raw block keeps any words typed into it as a paragraph.
            BlockTag::SceneBreak => {
                text.blocks.push(Block::SceneBreak);
                if !content.is_empty() {
                    text.blocks.push(Block::Para(content));
                }
            }
            BlockTag::Image { alt, src, role } => {
                text.blocks.push(Block::Image { alt, src, role });
                if !content.is_empty() {
                    text.blocks.push(Block::Para(content));
                }
            }
            BlockTag::Raw { format, text: raw } => {
                text.blocks.push(Block::Raw { format, text: raw });
                if !content.is_empty() {
                    text.blocks.push(Block::Para(content));
                }
            }
            BlockTag::EmptyFence { verse, aside, .. } => {
                let kind = if verse { Fence::Verse } else { Fence::Aside(aside) };
                let paras = if content.is_empty() { Vec::new() } else { vec![content] };
                text.blocks.push(Block::Fenced { kind, paras });
            }
            BlockTag::EmptyNote { label, .. } => {
                let paras = if content.is_empty() { Vec::new() } else { vec![content] };
                text.notes.push(Note { label, paras });
            }
            t @ (BlockTag::Quote { .. } | BlockTag::Verse { .. } | BlockTag::Aside { .. } | BlockTag::Note { .. }) => open = Some((t, vec![content])),
        }
    }
    flush(&mut open, &mut text);
    text
}

/// The block kind of a paragraph (body when untagged).
pub fn tag_of(p: &Paragraph) -> BlockTag {
    BLOCK.get_or_none(p).unwrap_or(BlockTag::Body)
}

/// Set a paragraph's block kind (`Body` removes the tag).
pub fn set_tag(p: &mut Paragraph, tag: &BlockTag) {
    if *tag == BlockTag::Body {
        BLOCK.remove(p);
    } else {
        let _ = BLOCK.set(p, tag);
    }
}

/// Whether a paragraph holds text the writer types (not a scene break, picture or raw block).
pub fn is_text(tag: &BlockTag) -> bool {
    !matches!(tag, BlockTag::SceneBreak | BlockTag::Image { .. } | BlockTag::Raw { .. })
}

#[cfg(test)]
mod tests;
