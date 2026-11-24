// SPDX-License-Identifier: MIT OR Apache-2.0
//! Compiling the manuscript for export (SPEC 16.1): the included items in order, with parts and
//! chapters titled and numbered by the options, scene breaks between scenes, front and back
//! matter, and replace-text rules. Export never changes the project.
//!
//! The result is a list of [`Piece`]s, from which each format is written, and the words the
//! export must hold, which the word-for-word check compares with the file written.

use serde::{Deserialize, Serialize};
use wordsmith_manuscript_model::{Block, Inline, Inlines, ItemId, Marks, Note, Project, Role, Text};

use crate::verify;

/// How a part or chapter is titled.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Titles {
    /// "Chapter 1"
    Numbered,
    /// "Chapter One"
    Words,
    /// "One"
    WordOnly,
    /// The item's title.
    #[default]
    Title,
    /// "Chapter 1: The Title"
    NumberedWithTitle,
    /// No heading.
    None,
}

/// A text replacement made on export (curly quotes to straight, a name changed for a draft).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Replace {
    /// What to find (plain text).
    pub find: String,
    /// What to put instead.
    pub with: String,
}

/// Export options; kept as named presets in the project.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Options {
    /// The items to export (with what is under them); empty means the included manuscript.
    pub items: Vec<ItemId>,
    /// How chapters are titled.
    pub chapters: Titles,
    /// How parts are titled.
    pub parts: Titles,
    /// Scenes get their titles as headings.
    pub scene_titles: bool,
    /// The scene-break symbol.
    pub scene_break: String,
    /// Front matter included.
    pub front_matter: bool,
    /// Back matter included.
    pub back_matter: bool,
    /// Replacements, applied in order to the text.
    pub replace: Vec<Replace>,
    /// "END" after the last paragraph.
    pub end_mark: bool,
    /// Comments go into the file as review comments where the format has them (DOCX).
    pub comments: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            items: Vec::new(),
            chapters: Titles::Title,
            parts: Titles::Title,
            scene_titles: false,
            scene_break: "#".into(),
            front_matter: true,
            back_matter: true,
            replace: Vec::new(),
            end_mark: false,
            comments: false,
        }
    }
}

/// One piece of the compiled manuscript.
#[derive(Clone, Debug, PartialEq)]
pub enum Piece {
    /// A heading: level 1 for parts, 2 for chapters, 3 for scenes; whether it starts a page.
    Heading {
        /// 1 to 3.
        level: u8,
        /// The text.
        text: String,
        /// The item it titles.
        item: ItemId,
        /// The item's role.
        role: Role,
    },
    /// A paragraph of text (body, quote, verse, aside or a heading inside an item).
    Para {
        /// The content.
        content: Inlines,
        /// The block kind it came from.
        kind: ParaKind,
        /// The item it came from.
        item: ItemId,
    },
    /// A scene break.
    SceneBreak,
    /// A picture.
    Image {
        /// Alt text.
        alt: String,
        /// Path in the project.
        src: String,
    },
    /// A raw block for one target format.
    Raw {
        /// The format.
        format: String,
        /// The content.
        text: String,
    },
    /// A note's text, after the paragraph referring to it is placed (its label is the reference's).
    Notes(Vec<Note>),
    /// The end mark.
    End,
}

/// The kind of a compiled paragraph.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParaKind {
    /// Body text; `first` after a heading or break.
    Body {
        /// The first paragraph after a heading or a break (no indent).
        first: bool,
    },
    /// A block quote.
    Quote,
    /// Verse.
    Verse,
    /// An aside.
    Aside,
    /// A heading inside an item's text.
    Heading(u8),
}

const ONES: [&str; 20] = [
    "Zero",
    "One",
    "Two",
    "Three",
    "Four",
    "Five",
    "Six",
    "Seven",
    "Eight",
    "Nine",
    "Ten",
    "Eleven",
    "Twelve",
    "Thirteen",
    "Fourteen",
    "Fifteen",
    "Sixteen",
    "Seventeen",
    "Eighteen",
    "Nineteen",
];
const TENS: [&str; 10] = ["", "", "Twenty", "Thirty", "Forty", "Fifty", "Sixty", "Seventy", "Eighty", "Ninety"];

/// A number in English words, capitalized ("Twenty-One").
pub fn number_words(n: usize) -> String {
    match n {
        0..=19 => ONES.get(n).copied().unwrap_or("").to_owned(),
        20..=99 => {
            let t = TENS.get(n / 10).copied().unwrap_or("");
            if n % 10 == 0 { t.to_owned() } else { format!("{t}-{}", ONES.get(n % 10).copied().unwrap_or("")) }
        }
        100..=999 => {
            let rest = n % 100;
            let h = format!("{} Hundred", ONES.get(n / 100).copied().unwrap_or(""));
            if rest == 0 { h } else { format!("{h} and {}", number_words(rest)) }
        }
        _ => n.to_string(),
    }
}

fn title(style: Titles, word: &str, n: usize, title: &str) -> Option<String> {
    match style {
        Titles::Numbered => Some(format!("{word} {n}")),
        Titles::Words => Some(format!("{word} {}", number_words(n))),
        Titles::WordOnly => Some(number_words(n)),
        Titles::Title => Some(title.to_owned()).filter(|t| !t.trim().is_empty()),
        Titles::NumberedWithTitle => Some(if title.trim().is_empty() { format!("{word} {n}") } else { format!("{word} {n}: {title}") }),
        Titles::None => None,
    }
}

fn replace_inlines(c: &mut Inlines, rules: &[Replace]) {
    for i in c.iter_mut() {
        if let Inline::Span { text, .. } = i {
            for r in rules.iter().filter(|r| !r.find.is_empty()) {
                if text.contains(&r.find) {
                    *text = text.replace(&r.find, &r.with);
                }
            }
        }
    }
}

/// The compiled manuscript.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Compiled {
    /// The pieces, in order.
    pub pieces: Vec<Piece>,
    /// The items exported, in order.
    pub items: Vec<ItemId>,
    /// The exported items' comments, by item and comment id.
    pub comments: std::collections::BTreeMap<(ItemId, String), wordsmith_manuscript_model::Comment>,
}

/// Compile the project for export.
pub fn compile(p: &Project, o: &Options) -> Compiled {
    let mut out = Compiled::default();
    let wanted: Vec<ItemId> = if o.items.is_empty() {
        p.included_manuscript()
    } else {
        o.items
            .iter()
            .filter_map(|i| p.binder.get(*i))
            .flat_map(|i| i.walk().into_iter().filter(|c| c.include).map(|c| c.id).collect::<Vec<_>>())
            .collect()
    };
    let (mut parts, mut chapters) = (0usize, 0usize);
    let mut last_was_scene = false;
    let mut first = true;
    for id in wanted {
        let Some(item) = p.binder.get(id) else { continue };
        match item.role {
            Role::FrontMatter if !o.front_matter => continue,
            Role::BackMatter if !o.back_matter => continue,
            _ => {}
        }
        let heading = match item.role {
            Role::Part => {
                parts += 1;
                title(o.parts, "Part", parts, &item.title).map(|t| (1u8, t))
            }
            Role::Chapter => {
                chapters += 1;
                title(o.chapters, "Chapter", chapters, &item.title).map(|t| (2u8, t))
            }
            Role::Scene if o.scene_titles => Some((3u8, item.title.clone())).filter(|(_, t)| !t.trim().is_empty()),
            _ => None,
        };
        if item.role == Role::Scene && last_was_scene && heading.is_none() {
            out.pieces.push(Piece::SceneBreak);
            first = true;
        }
        if let Some((level, text)) = heading {
            out.pieces.push(Piece::Heading { level, text, item: id, role: item.role });
            first = true;
        } else if matches!(item.role, Role::Part | Role::Chapter) {
            first = true;
        }
        last_was_scene = item.role == Role::Scene;
        if matches!(item.role, Role::Part | Role::Chapter) {
            last_was_scene = false;
        }
        out.items.push(id);
        let Some(t) = p.texts.get(&id) else { continue };
        for (k, c) in &t.meta.comments {
            out.comments.insert((id, k.clone()), c.clone());
        }
        let mut body: Text = t.body.clone();
        for b in &mut body.blocks {
            match b {
                Block::Para(c) | Block::Heading { content: c, .. } => replace_inlines(c, &o.replace),
                Block::Quote(ps) | Block::Fenced { paras: ps, .. } => ps.iter_mut().for_each(|c| replace_inlines(c, &o.replace)),
                _ => {}
            }
        }
        for n in &mut body.notes {
            n.paras.iter_mut().for_each(|c| replace_inlines(c, &o.replace));
        }
        for b in body.blocks {
            match b {
                Block::Para(content) => {
                    out.pieces.push(Piece::Para { content, kind: ParaKind::Body { first }, item: id });
                    first = false;
                }
                Block::Heading { level, content } => {
                    out.pieces.push(Piece::Para { content, kind: ParaKind::Heading(level), item: id });
                    first = true;
                }
                Block::SceneBreak => {
                    out.pieces.push(Piece::SceneBreak);
                    first = true;
                }
                Block::Quote(ps) => {
                    for content in ps {
                        out.pieces.push(Piece::Para { content, kind: ParaKind::Quote, item: id });
                    }
                    first = true;
                }
                Block::Fenced { kind, paras } => {
                    let k = if matches!(kind, wordsmith_manuscript_model::Fence::Verse) { ParaKind::Verse } else { ParaKind::Aside };
                    for content in paras {
                        out.pieces.push(Piece::Para { content, kind: k, item: id });
                    }
                    first = true;
                }
                Block::Image { alt, src, .. } => out.pieces.push(Piece::Image { alt, src }),
                Block::Raw { format, text } => out.pieces.push(Piece::Raw { format, text }),
            }
        }
        if !body.notes.is_empty() {
            out.pieces.push(Piece::Notes(body.notes));
        }
    }
    if o.end_mark {
        out.pieces.push(Piece::End);
    }
    out
}

impl Compiled {
    /// The words an export of this must hold, in reading order: headings, paragraphs, then each
    /// item's notes where they are placed. Scene-break symbols are not words; the end mark is.
    pub fn words(&self) -> Vec<String> {
        let mut out = Vec::new();
        for piece in &self.pieces {
            match piece {
                Piece::Heading { text, .. } => out.extend(verify::words(text)),
                Piece::Para { content, .. } => out.extend(verify::words(&wordsmith_manuscript_model::text::plain(content))),
                Piece::End => out.push("END".into()),
                _ => {}
            }
        }
        out
    }

    /// The words of the notes, in order.
    pub fn note_words(&self) -> Vec<String> {
        self.pieces
            .iter()
            .filter_map(|p| if let Piece::Notes(n) = p { Some(n) } else { None })
            .flatten()
            .flat_map(|n| n.paras.iter().flat_map(|c| verify::words(&wordsmith_manuscript_model::text::plain(c))))
            .collect()
    }
}

/// Marks with the comment taken off (comments are not exported as text).
pub fn export_marks(m: &Marks) -> Marks {
    Marks { comment: None, ..m.clone() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_in_words() {
        assert_eq!(number_words(1), "One");
        assert_eq!(number_words(21), "Twenty-One");
        assert_eq!(number_words(40), "Forty");
        assert_eq!(number_words(105), "One Hundred and Five");
    }
}
