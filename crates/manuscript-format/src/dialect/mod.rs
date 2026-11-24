// SPDX-License-Identifier: MIT OR Apache-2.0
//! Manuscript Markdown: the text dialect of `text/<id>.md` and `bible/<id>.md` (`docs/format.md`).
//!
//! A strict subset of CommonMark with a few extensions, read and written by our own line-based
//! code (decision 0003): one paragraph per line, a blank line between blocks, `*italic*`,
//! `**bold**`, `~~strike~~`, `[text]{.smallcaps}`, `[text]{lang=fr}`, `[text](wsm:<id>)`,
//! `[text]{.comment id=c1}`, `***` scene breaks, `#` headings, `>` quotes, `::: verse` and
//! `::: aside` fences, `![alt](media/x.png){role=wide}` images, raw blocks and `[^n]` notes.
//!
//! [`write`] gives canonical text and [`parse`] reads it back exactly; parsing never panics and
//! never loses a character of hand-written text it cannot interpret (unmatched markers stay as
//! text).

mod inline;
mod parse;
mod write;

pub use parse::parse;
pub use write::write;

use wordsmith_manuscript_model::text::normalize_inlines;
use wordsmith_manuscript_model::{Block, Fence, Inline, Inlines, Marks, Text};

/// A line holding only this is an empty paragraph.
pub const EMPTY_PARA: &str = "\\";

/// The deepest marks may nest in hand-written text before the rest of a paragraph is read as
/// plain text.
pub const MAX_NESTING: usize = 32;

/// How many times a paragraph with unmatched markers is re-read before all its markers are taken
/// as plain text.
pub const MAX_RETRIES: usize = 64;

/// A raw block's fence: three backticks, or more than the longest run in the text.
fn fence_for(text: &str) -> String {
    let mut longest = 0;
    let mut run = 0;
    for c in text.chars() {
        if c == '`' {
            run += 1;
            longest = longest.max(run);
        } else {
            run = 0;
        }
    }
    "`".repeat((longest + 1).max(3))
}

fn word_like(s: &str) -> String {
    let cleaned: String = s.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '-' }).collect();
    cleaned
}

fn one_line(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn canonical_marks(m: &mut Marks) {
    if let Some(l) = &mut m.lang {
        *l = word_like(l);
    }
    if let Some(c) = &mut m.comment {
        *c = word_like(c);
    }
}

fn canonical_inlines(c: &mut Inlines) {
    for i in c.iter_mut() {
        match i {
            Inline::Span { marks, .. } => canonical_marks(marks),
            Inline::NoteRef { label, marks } => {
                *label = word_like(label);
                if label.is_empty() {
                    *label = "n".into();
                }
                canonical_marks(marks);
            }
        }
    }
    normalize_inlines(c);
}

/// Put a text in the exact form the dialect writes, so model → text → model is the identity:
/// merged spans, machine values (language tags, ids, labels, kinds) in their allowed characters,
/// heading levels 1 to 3, a quote with at least one paragraph.
pub fn canonical(t: &mut Text) {
    t.normalize();
    for b in &mut t.blocks {
        match b {
            Block::Para(c) => canonical_inlines(c),
            Block::Heading { level, content } => {
                *level = (*level).clamp(1, 3);
                canonical_inlines(content);
            }
            Block::Quote(ps) => {
                if ps.is_empty() {
                    ps.push(Vec::new());
                }
                ps.iter_mut().for_each(canonical_inlines);
            }
            Block::Fenced { kind, paras } => {
                if let Fence::Aside(Some(k)) = kind {
                    let w = one_line(k);
                    *kind = Fence::Aside((!w.is_empty()).then_some(w));
                }
                paras.iter_mut().for_each(canonical_inlines);
            }
            Block::Raw { format, .. } => {
                *format = word_like(&one_line(format));
                if format.is_empty() {
                    *format = "text".into();
                }
            }
            Block::Image { .. } | Block::SceneBreak => {}
        }
    }
    for n in &mut t.notes {
        n.label = word_like(&n.label);
        if n.label.is_empty() {
            n.label = "n".into();
        }
        n.paras.iter_mut().for_each(canonical_inlines);
    }
}

#[cfg(test)]
mod tests;
