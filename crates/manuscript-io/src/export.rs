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
    /// An e-book reading copy (EPUB), to read the draft on a phone or e-reader (EXP-6).
    Epub,
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
            Target::Epub => Format::Epub,
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
            "epub" => Some(Target::Epub),
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
    let mut comment_ids: BTreeMap<(ItemId, String), u32> = BTreeMap::new();
    // Each comment's range: from the first inline marked with it to the last (a comment may run
    // over several paragraphs, or over words split by unmarked ones).
    let mut spans: BTreeMap<(ItemId, String), ((usize, usize), (usize, usize))> = BTreeMap::new();
    if o.comments {
        for (pi, piece) in c.pieces.iter().enumerate() {
            if let Piece::Para { content, item, .. } = piece {
                for (n, i) in content.iter().enumerate() {
                    if let Some(k) = &i.marks().comment {
                        spans.entry((*item, k.clone())).and_modify(|r| r.1 = (pi, n)).or_insert(((pi, n), (pi, n)));
                    }
                }
            }
        }
    }
    for (pi, piece) in c.pieces.iter().enumerate() {
        let para = match piece {
            Piece::Heading { level, text, .. } => {
                let mut p = styled(Paragraph::with_text(text, CharProps::default()), &format!("Heading{level}"));
                p.props.outline_level = Some(level.saturating_sub(1));
                Some(p)
            }
            Piece::Para { content, kind, item } => {
                let mut p = Paragraph::new();
                for (n, i) in content.iter().enumerate() {
                    let here = (pi, n);
                    for (key, &(first, _)) in &spans {
                        if first == here
                            && let Some(com) = c.comments.get(key)
                        {
                            let id = next_part;
                            next_part += 1;
                            comment_ids.insert(key.clone(), id);
                            let blocks = com
                                .text
                                .split("\n\n")
                                .map(|t| Arc::new(jcj_doc::Block::Para(Paragraph::with_text(t, CharProps::default()))))
                                .collect();
                            doc.parts.insert(id, Part { kind: PartKind::Comment, blocks, body: Default::default() });
                            doc.comments
                                .insert(id, jcj_doc::Comment { author: com.by.clone(), date: com.date.clone(), part: id, ..Default::default() });
                            let at = p.text.len();
                            let _ = p.insert_object(at, InlineObject::CommentStart { id }, &CharProps::default());
                        }
                    }
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
                    for (key, &(_, last)) in &spans {
                        if last == here
                            && let Some(&id) = comment_ids.get(key)
                        {
                            let at = p.text.len();
                            let _ = p.insert_object(at, InlineObject::CommentEnd { id }, &CharProps::default());
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

/// For the EPUB reading copy: each chapter's notes as numbered endnotes at its end, the note
/// references as superscript numbers. Returns the words this adds (the numbers and the "Notes"
/// headings), which the word check leaves out.
///
/// core-candidate: open-source-core#33. The core's EPUB writer keeps notes since release 0.1.12
/// (core issue #21), but numbers them from 1 in each chapter file and adds the English word
/// "Back" under each, which a caller can neither change nor learn. This goes when the caller
/// can choose the numbering and the added words and be told them. `docs/core-candidates.md`.
fn notes_as_endnotes(doc: &mut Document) -> Vec<String> {
    let mut added: Vec<String> = Vec::new();
    let mut body: Vec<Arc<jcj_doc::Block>> = Vec::new();
    let mut pending: Vec<(u32, u32)> = Vec::new();
    let mut n = 0u32;
    let parts = doc.parts.clone();
    let flush = |body: &mut Vec<Arc<jcj_doc::Block>>, pending: &mut Vec<(u32, u32)>, added: &mut Vec<String>| {
        if pending.is_empty() {
            return;
        }
        let mut head = Paragraph::with_text("Notes", CharProps { bold: Some(true), ..CharProps::default() });
        head.props.style = None;
        body.push(Arc::new(jcj_doc::Block::Para(head)));
        added.push("Notes".into());
        for (num, part) in pending.drain(..) {
            let paras: Vec<Paragraph> = parts.get(&part).map(|p| p.blocks.iter().filter_map(|b| b.as_para().cloned()).collect()).unwrap_or_default();
            for (k, mut para) in paras.into_iter().enumerate() {
                para.props.style = None;
                if k == 0 {
                    let _ = para.insert_text(0, &format!("{num}. "), &CharProps::default());
                    added.push(num.to_string());
                }
                body.push(Arc::new(jcj_doc::Block::Para(para)));
            }
        }
    };
    for block in std::mem::take(&mut doc.body) {
        let starts_chapter = block.as_para().is_some_and(|p| p.props.style.as_deref() == Some("Heading1"));
        if starts_chapter {
            flush(&mut body, &mut pending, &mut added);
        }
        let mut block = block;
        if let jcj_doc::Block::Para(p) = Arc::make_mut(&mut block) {
            let refs: Vec<(usize, u32)> = p
                .objects_with_offsets()
                .filter_map(|(off, o)| if let InlineObject::NoteRef { id, .. } = o { Some((off, *id)) } else { None })
                .collect();
            for (off, id) in refs.into_iter().rev() {
                n += 1;
                let _ = p.delete(off, off + jcj_doc::para::OBJ.len_utf8());
                let sup = CharProps { vert_align: Some(jcj_doc::props::VertAlign::Superscript), ..CharProps::default() };
                let _ = p.insert_text(off, &n.to_string(), &sup);
                pending.push((n, id));
                added.push(n.to_string());
            }
        }
        body.push(block);
    }
    flush(&mut body, &mut pending, &mut added);
    doc.body = body;
    doc.parts.retain(|_, p| !matches!(p.kind, PartKind::Footnote | PartKind::Endnote));
    added
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
        let body = n.paras.iter().map(&para).collect::<Vec<_>>().join(" ");
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
    let (mut doc, left_out) = document(&c, o);
    if target == Target::Epub {
        // The reading copy's title page in the reader's library.
        let b = &p.settings.book;
        doc.core.title.clone_from(&b.title);
        doc.core.creator = b.byline().to_owned();
        // A reader opens each part and chapter on its own page with its own contents entry:
        // the writer starts a file at each level 1 heading, so parts and chapters are level 1,
        // and the headings under them move up one level.
        for block in doc.body.iter_mut() {
            if let jcj_doc::Block::Para(para) = Arc::make_mut(block) {
                let level = match para.props.style.as_deref() {
                    Some("Heading1" | "Heading2") => Some(1u8),
                    Some(s) => s.strip_prefix("Heading").and_then(|n| n.parse::<u8>().ok()).filter(|n| *n > 2).map(|n| n - 1),
                    None => None,
                };
                if let Some(l) = level {
                    para.props.style = Some(format!("Heading{l}"));
                    para.props.outline_level = Some(l - 1);
                }
            }
        }
    }
    if target == Target::Epub {
        // Two checks: the reading copy's document holds every letter of the manuscript (notes
        // moved to chapter ends, plus the note numbers), and the file holds that document, word
        // for word in order.
        let added = notes_as_endnotes(&mut doc);
        let bytes = jcj_interchange::export(target.format(), &doc).map_err(|e| e.to_string())?;
        let written = verify::document_words(&doc);
        let mut want: Vec<String> = crate::standard::letters(expected.iter().chain(&added).map(String::as_str));
        let mut have: Vec<String> = crate::standard::letters(written.iter().map(String::as_str));
        want.sort();
        have.sort();
        let moved = Check::of(&want, &have);
        let check = if moved.passed() { Check::of(&written, &read_back(target.format(), &bytes)?) } else { moved };
        return Ok(Exported { bytes, check, left_out });
    }
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
