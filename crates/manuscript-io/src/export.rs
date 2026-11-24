// SPDX-License-Identifier: MIT OR Apache-2.0
//! Writing the compiled manuscript (EXP-3, EXP-4): ODT, RTF, Markdown, plain text and HTML
//! through the core's `jcj-interchange`, each checked word for word by reading the file back.

use std::collections::BTreeMap;
use std::sync::Arc;

use jcj_doc::para::InlineObject;
use jcj_doc::{Align, CharProps, Document, Paragraph, Part, PartKind};
use jcj_interchange::Format;
use serde::{Deserialize, Serialize};
use wordsmith_manuscript_model::doc as mdoc;
use wordsmith_manuscript_model::{Inline, Inlines, ItemId, Project};

use crate::compile::{self, Compiled, Options, ParaKind, Piece};
use crate::verify::{self, Check};

/// The formats Manuscript exports through the core.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Target {
    /// Word processor (DOCX), in the compiled layout (the standard manuscript layout is M6.2's).
    Docx,
    /// OpenDocument text.
    Odt,
    /// Rich text.
    Rtf,
    /// Markdown.
    Markdown,
    /// Plain text.
    Text,
    /// A web page.
    Html,
}

impl Target {
    /// The core's format.
    pub fn format(self) -> Format {
        match self {
            Target::Docx => Format::Docx,
            Target::Odt => Format::Odt,
            Target::Rtf => Format::Rtf,
            Target::Markdown => Format::Markdown,
            Target::Text => Format::Text,
            Target::Html => Format::Html,
        }
    }

    /// The target for a file extension.
    pub fn from_extension(ext: &str) -> Option<Target> {
        match ext.to_ascii_lowercase().as_str() {
            "docx" => Some(Target::Docx),
            "odt" => Some(Target::Odt),
            "rtf" => Some(Target::Rtf),
            "md" | "markdown" => Some(Target::Markdown),
            "txt" | "text" => Some(Target::Text),
            "html" | "htm" => Some(Target::Html),
            _ => None,
        }
    }
}

fn styled(mut p: Paragraph, style: &str) -> Paragraph {
    p.props.style = Some(style.into());
    p
}

/// The core document for a compiled manuscript, with real footnotes.
pub fn document(c: &Compiled, o: &Options) -> (Document, BTreeMap<String, usize>) {
    let mut doc = jcj_doc::new_document();
    doc.body.clear();
    let mut left_out: BTreeMap<String, usize> = BTreeMap::new();
    // Each item's notes by label.
    let mut notes: BTreeMap<(ItemId, String), Vec<Inlines>> = BTreeMap::new();
    let mut current: Option<ItemId> = None;
    for piece in &c.pieces {
        match piece {
            Piece::Para { item, .. } | Piece::Heading { item, .. } => current = Some(*item),
            Piece::Notes(ns) => {
                if let Some(item) = current {
                    for n in ns {
                        notes.insert((item, n.label.clone()), n.paras.clone());
                    }
                }
            }
            _ => {}
        }
    }
    let mut next_part = 1u32;
    for piece in &c.pieces {
        let para = match piece {
            Piece::Heading { level, text, .. } => {
                let mut p = styled(Paragraph::with_text(text, CharProps::default()), &format!("Heading{level}"));
                p.props.outline_level = Some(level.saturating_sub(1));
                Some(p)
            }
            Piece::Para { content, kind, item } => {
                let mut p = Paragraph::new();
                for i in content {
                    let end = p.text.len();
                    match i {
                        Inline::Span { text, marks } => {
                            let _ = p.insert_text(end, text, &mdoc::props_of(&compile::export_marks(marks)));
                        }
                        Inline::NoteRef { label, .. } => {
                            let id = next_part;
                            next_part += 1;
                            let mut part_blocks = Vec::new();
                            for np in notes.get(&(*item, label.clone())).cloned().unwrap_or_default() {
                                let mut q = Paragraph::new();
                                for ni in np {
                                    if let Inline::Span { text, marks } = ni {
                                        let e = q.text.len();
                                        let _ = q.insert_text(e, &text, &mdoc::props_of(&compile::export_marks(&marks)));
                                    }
                                }
                                part_blocks.push(Arc::new(jcj_doc::Block::Para(styled(q, "FootnoteText"))));
                            }
                            doc.parts.insert(id, Part { kind: PartKind::Footnote, blocks: part_blocks, body: Default::default() });
                            let _ = p.insert_object(
                                end,
                                InlineObject::NoteRef { kind: Default::default(), id, custom: String::new() },
                                &CharProps::default(),
                            );
                        }
                    }
                }
                p.normalize();
                Some(match kind {
                    ParaKind::Body { .. } => p,
                    ParaKind::Quote | ParaKind::Verse | ParaKind::Aside => styled(p, "Quote"),
                    ParaKind::Heading(l) => styled(p, &format!("Heading{}", (*l).clamp(3, 6))),
                })
            }
            Piece::SceneBreak => {
                let mut p = Paragraph::with_text(&o.scene_break, CharProps::default());
                p.props.align = Some(Align::Center);
                Some(p)
            }
            Piece::End => {
                let mut p = Paragraph::with_text("END", CharProps::default());
                p.props.align = Some(Align::Center);
                Some(p)
            }
            Piece::Image { .. } => {
                *left_out.entry("pictures".into()).or_default() += 1;
                None
            }
            Piece::Raw { .. } => {
                *left_out.entry("raw blocks".into()).or_default() += 1;
                None
            }
            Piece::Notes(_) => None,
        };
        if let Some(p) = para {
            doc.body.push(Arc::new(jcj_doc::Block::Para(p)));
        }
    }
    if doc.body.is_empty() {
        doc.body.push(Arc::new(jcj_doc::Block::Para(Paragraph::new())));
    }
    (doc, left_out)
}

/// An export's result: the file's bytes and its checks.
#[derive(Clone, Debug)]
pub struct Exported {
    /// The bytes to write.
    pub bytes: Vec<u8>,
    /// The word-for-word check of the main text, read back from the bytes.
    pub check: Check,
    /// Content not exported (pictures, raw blocks), with counts.
    pub left_out: BTreeMap<String, usize>,
}

/// The words of an exported file read back: its main text, then its notes (as the formats keep
/// them).
pub fn read_back(format: Format, bytes: &[u8]) -> Result<Vec<String>, String> {
    let doc = jcj_interchange::import(format, bytes).map_err(|e| format!("the file written could not be read back: {e}"))?;
    let mut words = verify::document_words(&doc);
    for part in doc.parts.values().filter(|p| matches!(p.kind, PartKind::Footnote | PartKind::Endnote)) {
        for b in &part.blocks {
            if let Some(p) = b.as_para() {
                words.extend(verify::paragraph_words(p));
            }
        }
    }
    Ok(words)
}

/// The compiled manuscript as semantic text (for Markdown): notes relabelled so labels never
/// repeat across items.
pub fn as_text(c: &Compiled, o: &Options) -> wordsmith_manuscript_model::Text {
    use wordsmith_manuscript_model::{Block, Fence, Note, Text};
    let mut t = Text::new();
    let mut n = 0usize;
    let mut labels: BTreeMap<(usize, String), String> = BTreeMap::new();
    let mut item_no = 0usize;
    let mut last_item: Option<ItemId> = None;
    let mut open: Option<(ParaKind, Vec<Inlines>)> = None;
    let flush = |open: &mut Option<(ParaKind, Vec<Inlines>)>, t: &mut Text| {
        if let Some((k, ps)) = open.take() {
            t.blocks.push(match k {
                ParaKind::Quote => Block::Quote(ps),
                ParaKind::Verse => Block::Fenced { kind: Fence::Verse, paras: ps },
                _ => Block::Fenced { kind: Fence::Aside(None), paras: ps },
            });
        }
    };
    for piece in &c.pieces {
        if let Piece::Para { item, .. } | Piece::Heading { item, .. } = piece
            && last_item != Some(*item)
        {
            last_item = Some(*item);
            item_no += 1;
        }
        match piece {
            Piece::Para { content, kind: kind @ (ParaKind::Quote | ParaKind::Verse | ParaKind::Aside), .. } => {
                let content = relabel(content, item_no, &mut labels, &mut n);
                match open.as_mut() {
                    Some((k, ps)) if k == kind => ps.push(content),
                    _ => {
                        flush(&mut open, &mut t);
                        open = Some((*kind, vec![content]));
                    }
                }
                continue;
            }
            _ => flush(&mut open, &mut t),
        }
        match piece {
            Piece::Heading { level, text, .. } => t.blocks.push(Block::Heading { level: *level, content: vec![Inline::plain(text)] }),
            Piece::Para { content, kind, .. } => {
                let content = relabel(content, item_no, &mut labels, &mut n);
                t.blocks.push(match kind {
                    ParaKind::Heading(l) => Block::Heading { level: (*l).clamp(1, 3), content },
                    _ => Block::Para(content),
                });
            }
            Piece::SceneBreak => t.blocks.push(Block::SceneBreak),
            Piece::End => t.blocks.push(Block::Para(vec![Inline::plain("END")])),
            Piece::Image { alt, src } => t.blocks.push(Block::Image { alt: alt.clone(), src: src.clone(), role: Default::default() }),
            Piece::Raw { format, text } => t.blocks.push(Block::Raw { format: format.clone(), text: text.clone() }),
            Piece::Notes(ns) => {
                for note in ns {
                    if let Some(l) = labels.get(&(item_no, note.label.clone())) {
                        let paras = note
                            .paras
                            .iter()
                            .map(|p| {
                                p.iter()
                                    .map(|i| match i {
                                        Inline::Span { text, marks } => Inline::Span { text: text.clone(), marks: compile::export_marks(marks) },
                                        other => other.clone(),
                                    })
                                    .collect()
                            })
                            .collect();
                        t.notes.push(Note { label: l.clone(), paras });
                    }
                }
            }
        }
    }
    flush(&mut open, &mut t);
    let _ = o;
    t.normalize();
    t
}

fn relabel(content: &Inlines, item: usize, labels: &mut BTreeMap<(usize, String), String>, n: &mut usize) -> Inlines {
    content
        .iter()
        .map(|i| match i {
            Inline::NoteRef { label, marks } => {
                let l = labels.entry((item, label.clone())).or_insert_with(|| {
                    *n += 1;
                    n.to_string()
                });
                Inline::NoteRef { label: l.clone(), marks: compile::export_marks(marks) }
            }
            Inline::Span { text, marks } => Inline::Span { text: text.clone(), marks: compile::export_marks(marks) },
        })
        .collect()
}

/// Plain text: headings and paragraphs separated by blank lines, the scene-break symbol on its
/// own line, notes as `[n]` with their text after each part that refers to them.
pub fn plain_text(c: &Compiled, o: &Options) -> String {
    let t = as_text(c, o);
    let mut out = String::new();
    let para = |c: &Inlines| -> String {
        c.iter()
            .map(|i| match i {
                Inline::Span { text, .. } => text.clone(),
                Inline::NoteRef { label, .. } => format!("[{label}]"),
            })
            .collect()
    };
    use wordsmith_manuscript_model::Block;
    for b in &t.blocks {
        let s = match b {
            Block::Heading { content, .. } => para(content),
            Block::Para(c) => para(c),
            Block::SceneBreak => o.scene_break.clone(),
            Block::Quote(ps) | Block::Fenced { paras: ps, .. } => ps.iter().map(|p| format!("    {}", para(p))).collect::<Vec<_>>().join("\n\n"),
            Block::Image { .. } | Block::Raw { .. } => continue,
        };
        out.push_str(&s);
        out.push_str("\n\n");
    }
    for n in &t.notes {
        let body = n.paras.iter().map(|p| para(p)).collect::<Vec<_>>().join(" ");
        out.push_str(&format!("[{}] {body}\n\n", n.label));
    }
    out
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn html_inlines(c: &Inlines) -> String {
    let mut out = String::new();
    for i in c {
        match i {
            Inline::Span { text, marks } => {
                let mut open = String::new();
                let mut close = String::new();
                let mut wrap = |tag: &str, attrs: String| {
                    open.push_str(&format!("<{tag}{attrs}>"));
                    close.insert_str(0, &format!("</{tag}>"));
                };
                if let Some(l) = &marks.lang {
                    wrap("span", format!(" lang=\"{}\"", esc(l)));
                }
                if marks.small_caps {
                    wrap("span", " style=\"font-variant: small-caps\"".into());
                }
                if marks.bold {
                    wrap("strong", String::new());
                }
                if marks.italic {
                    wrap("em", String::new());
                }
                if marks.strike {
                    wrap("s", String::new());
                }
                out.push_str(&open);
                out.push_str(&esc(text).replace('\n', "<br>"));
                out.push_str(&close);
            }
            Inline::NoteRef { label, .. } => out.push_str(&format!("<sup><a href=\"#note-{0}\" id=\"ref-{0}\">{0}</a></sup>", esc(label))),
        }
    }
    out
}

/// A web page of the compiled manuscript: headings, paragraphs, quotes, verse, scene breaks, and
/// the notes at the end.
pub fn html(c: &Compiled, o: &Options, title: &str) -> String {
    use wordsmith_manuscript_model::Block;
    let t = as_text(c, o);
    let mut out = format!("<!DOCTYPE html>\n<html>\n<head>\n<meta charset=\"utf-8\">\n<title>{}</title>\n</head>\n<body>\n", esc(title));
    for b in &t.blocks {
        match b {
            Block::Heading { level, content } => out.push_str(&format!("<h{level}>{}</h{level}>\n", html_inlines(content))),
            Block::Para(cn) => out.push_str(&format!("<p>{}</p>\n", html_inlines(cn))),
            Block::SceneBreak => out.push_str(&format!("<p class=\"scene-break\" style=\"text-align: center\">{}</p>\n", esc(&o.scene_break))),
            Block::Quote(ps) | Block::Fenced { paras: ps, .. } => {
                out.push_str("<blockquote>\n");
                for p in ps {
                    out.push_str(&format!("<p>{}</p>\n", html_inlines(p)));
                }
                out.push_str("</blockquote>\n");
            }
            Block::Image { alt, src, .. } => out.push_str(&format!("<p><img src=\"{}\" alt=\"{}\"></p>\n", esc(src), esc(alt))),
            Block::Raw { format, text } if format == "html" => {
                out.push_str(text);
                out.push('\n');
            }
            Block::Raw { .. } => {}
        }
    }
    if !t.notes.is_empty() {
        out.push_str("<section class=\"notes\">\n<ol>\n");
        for n in &t.notes {
            let body = n.paras.iter().map(html_inlines).collect::<Vec<_>>().join(" ");
            out.push_str(&format!("<li id=\"note-{0}\">{body}</li>\n", esc(&n.label)));
        }
        out.push_str("</ol>\n</section>\n");
    }
    out.push_str("</body>\n</html>\n");
    out
}

/// The words of a page this module wrote: tags removed, entities read, note numbers left out.
fn html_words(page: &str) -> Vec<String> {
    let body = page.split_once("<body>").map_or(page, |(_, b)| b);
    let mut text = String::new();
    let mut in_tag = false;
    let mut in_sup = 0;
    let mut i = 0;
    while let Some(ch) = body[i..].chars().next() {
        if ch == '<' {
            let tag_end = body[i..].find('>').map_or(body.len(), |e| i + e);
            let tag = &body[i + 1..tag_end];
            if tag.starts_with("sup") {
                in_sup += 1;
            } else if tag.starts_with("/sup") {
                in_sup -= 1;
            }
            // Block tags separate words; inline tags do not.
            if ["p", "/p", "h1", "/h1", "h2", "/h2", "h3", "/h3", "li", "/li", "br", "blockquote", "/blockquote"]
                .iter()
                .any(|t| tag == *t || tag.starts_with(&format!("{t} ")))
            {
                text.push(' ');
            }
            in_tag = false;
            i = tag_end + 1;
            continue;
        }
        if !in_tag && in_sup == 0 {
            text.push(ch);
        }
        i += ch.len_utf8();
    }
    let text = text.replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&amp;", "&");
    verify::words(&text)
}

/// Words of a file Manuscript wrote itself, read back: note markers (`[12]`) taken out.
fn own_words(text: &str) -> Vec<String> {
    verify::words(text)
        .into_iter()
        .map(|w| {
            let mut out = String::new();
            let mut rest = w.as_str();
            while let Some(a) = rest.find('[') {
                match rest[a + 1..].find(']') {
                    Some(b) if !rest[a + 1..a + 1 + b].is_empty() && rest[a + 1..a + 1 + b].chars().all(|c| c.is_ascii_digit()) => {
                        out.push_str(&rest[..a]);
                        rest = &rest[a + 2 + b..];
                    }
                    _ => {
                        out.push_str(&rest[..=a]);
                        rest = &rest[a + 1..];
                    }
                }
            }
            out.push_str(rest);
            out
        })
        .filter(|w| w.chars().any(char::is_alphanumeric))
        .collect()
}

/// Export the project to a format, checked.
pub fn export(p: &Project, o: &Options, target: Target) -> Result<Exported, String> {
    let c = compile::compile(p, o);
    let mut expected = c.words();
    expected.extend(c.note_words());
    if matches!(target, Target::Markdown | Target::Text | Target::Html) {
        let (bytes, found) = if target == Target::Html {
            let page = html(&c, o, &p.settings.book.title);
            let words = html_words(&page);
            (page.into_bytes(), words)
        } else if target == Target::Markdown {
            let md = wordsmith_manuscript_format::dialect::write(&as_text(&c, o));
            let back = wordsmith_manuscript_format::dialect::parse(&md);
            let words: Vec<String> = back.plain_paragraphs().iter().flat_map(|x| verify::words(x)).collect();
            (md.into_bytes(), words)
        } else {
            let txt = plain_text(&c, o);
            let words = own_words(&txt);
            (txt.into_bytes(), words)
        };
        let check = Check::of(&sorted_if_notes(&expected, &c), &sorted_if_notes(&found, &c));
        let mut left_out = BTreeMap::new();
        for piece in &c.pieces {
            if matches!(piece, Piece::Image { .. }) && target == Target::Text {
                *left_out.entry("pictures".to_owned()).or_default() += 1;
            }
        }
        return Ok(Exported { bytes, check, left_out });
    }
    let (doc, left_out) = document(&c, o);
    let bytes = jcj_interchange::export(target.format(), &doc).map_err(|e| e.to_string())?;
    let found = read_back(target.format(), &bytes)?;
    // Notes come after the text in some formats and at their references in others: the check
    // compares the words as a whole, in the order each format keeps.
    let check = Check::of(&sorted_if_notes(&expected, &c), &sorted_if_notes(&found, &c));
    Ok(Exported { bytes, check, left_out })
}

/// With notes, their place in the read-back order differs by format: compare as sorted lists
/// (every word, each as often) rather than in order. Without notes, the order is compared.
fn sorted_if_notes(words: &[String], c: &Compiled) -> Vec<String> {
    let mut v = words.to_vec();
    if !c.note_words().is_empty() {
        v.sort();
    }
    v
}
