// SPDX-License-Identifier: MIT OR Apache-2.0
//! From an imported document to a binder plan (IMP-1, IMP-3, IMP-4; SPEC 15.2 to 15.4).
//!
//! Headings become parts, chapters and scenes by rules the writer can change; a line such as
//! "Chapter 12" starts a chapter even without a heading style; scene-break lines split a chapter
//! into scenes (or stay as scene breaks); italic, bold, small caps, strikethrough, notes and
//! comments are kept; fonts, sizes and colors are dropped and listed. Every word goes somewhere:
//! the plan's words, read in order, equal the source's ([`check`]).

use std::collections::BTreeMap;

use jcj_doc::para::InlineObject;
use jcj_doc::{CharProps, Document, Paragraph, PartKind};
use serde::{Deserialize, Serialize};
use wordsmith_manuscript_model::doc as mdoc;
use wordsmith_manuscript_model::{Block, Comment, Inline, Inlines, ItemMeta, ItemText, Kind, Marks, Note, Role, Text};

use crate::verify::{self, Check};

/// The rules that shape an import. Kept in the project (`settings.extra["import"]`) once the
/// writer changes them.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Rules {
    /// The roles of heading levels 1, 2, 3 (`None`: decide from the levels the document uses).
    pub heading_roles: Option<Vec<Role>>,
    /// A paragraph like "Chapter 12" or "Part Two" starts a chapter or part.
    pub chapter_lines: bool,
    /// Scene breaks split a chapter into scenes (else they stay as scene-break paragraphs).
    pub split_scenes: bool,
    /// More lines that count as scene breaks, besides the built-in ones.
    pub scene_marks: Vec<String>,
    /// Underlined text becomes italic (the manuscript convention); else underline is dropped.
    pub underline_as_italic: bool,
}

impl Default for Rules {
    fn default() -> Self {
        Rules { heading_roles: None, chapter_lines: true, split_scenes: true, scene_marks: Vec::new(), underline_as_italic: true }
    }
}

/// One item to make.
#[derive(Clone, Debug, PartialEq)]
pub struct Planned {
    /// Its title.
    pub title: String,
    /// Whether the title's words came from the source (a heading), rather than being made up.
    pub title_from_source: bool,
    /// Its role.
    pub role: Role,
    /// Folder or text.
    pub kind: Kind,
    /// How deep, 0 at the top of what is imported.
    pub depth: usize,
    /// Its text.
    pub text: ItemText,
}

/// What an import found and did.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Report {
    /// Items made, by role.
    pub made: BTreeMap<String, usize>,
    /// Formatting that could not be kept (each kind once, with a count).
    pub dropped: BTreeMap<String, usize>,
    /// The word-for-word check of the main text.
    pub check: Option<Check>,
    /// The word-for-word check of the notes.
    pub notes_check: Option<Check>,
}

/// A plan: the items, in order, and the report.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Plan {
    /// The items, depth first.
    pub items: Vec<Planned>,
    /// The report.
    pub report: Report,
}

/// How a paragraph reads.
#[derive(Clone, Debug, PartialEq)]
enum Line {
    /// A heading of level 1 to 9.
    Heading(usize),
    /// "Chapter 12", "Part Two": a chapter or part line.
    Starts(Role),
    /// A scene break.
    Break,
    /// Body text.
    Body,
}

const NUMBER_WORDS: [&str; 30] = [
    "one",
    "two",
    "three",
    "four",
    "five",
    "six",
    "seven",
    "eight",
    "nine",
    "ten",
    "eleven",
    "twelve",
    "thirteen",
    "fourteen",
    "fifteen",
    "sixteen",
    "seventeen",
    "eighteen",
    "nineteen",
    "twenty",
    "thirty",
    "forty",
    "fifty",
    "first",
    "second",
    "third",
    "fourth",
    "fifth",
    "last",
    "the",
];

/// Whether a short line names a chapter or part ("Chapter 12", "CHAPTER XII.", "Part Two").
fn starts(text: &str) -> Option<Role> {
    let t = text.trim();
    if t.is_empty() || t.chars().count() > 60 {
        return None;
    }
    let mut words = t.split_whitespace();
    let first = words.next()?.to_lowercase();
    let second = words.next()?.trim_end_matches(['.', ':', ']', ',']).to_lowercase();
    let role = match first.as_str() {
        "chapter" => Role::Chapter,
        "part" | "book" => Role::Part,
        _ => return None,
    };
    let numberish = second.chars().all(|c| c.is_ascii_digit()) && !second.is_empty()
        || second.chars().all(|c| "ivxlcdm".contains(c)) && !second.is_empty()
        || NUMBER_WORDS.iter().any(|n| second.starts_with(n));
    numberish.then_some(role)
}

/// Whether a paragraph is a scene break: only symbols (`***`, `* * *`, `#`, `~`, a run of one
/// symbol), a short centered line of symbols, or one of the extra marks.
pub fn is_break(text: &str, centered: bool, extra: &[String]) -> bool {
    let t = text.trim();
    if t.is_empty() {
        return false;
    }
    if extra.iter().any(|m| m.trim() == t) {
        return true;
    }
    if t.chars().any(char::is_alphanumeric) || t.chars().count() > 12 {
        return false;
    }
    let marks: Vec<char> = t.chars().filter(|c| !c.is_whitespace()).collect();
    let one_symbol = marks.windows(2).all(|w| w[0] == w[1]);
    one_symbol || (centered && marks.len() <= 5)
}

pub(crate) fn heading_level(doc: &Document, p: &Paragraph) -> Option<usize> {
    if let Some(l) = p.props.outline_level.filter(|l| *l < 9) {
        return Some(usize::from(l) + 1);
    }
    let mut style = p.props.style.clone();
    for _ in 0..8 {
        let id = style?;
        let s = doc.styles.styles.iter().find(|s| s.id == id)?;
        if let Some(l) = s.para.outline_level.filter(|l| *l < 9) {
            return Some(usize::from(l) + 1);
        }
        let name = s.name.to_lowercase();
        if let Some(n) = name.strip_prefix("heading ").and_then(|n| n.trim().parse::<usize>().ok()) {
            return Some(n);
        }
        style = s.based_on.clone();
    }
    None
}

fn line(doc: &Document, p: &Paragraph, rules: &Rules) -> Line {
    let text = p.text.replace(jcj_doc::para::OBJ, "");
    if let Some(l) = heading_level(doc, p)
        && !text.trim().is_empty()
    {
        return Line::Heading(l);
    }
    let centered = matches!(p.props.align, Some(jcj_doc::Align::Center));
    if is_break(&text, centered, &rules.scene_marks) {
        return Line::Break;
    }
    if rules.chapter_lines
        && let Some(r) = starts(&text)
    {
        return Line::Starts(r);
    }
    Line::Body
}

/// The roles of heading levels 1, 2, 3 for a document that uses `levels` (SPEC 15.2).
fn roles_for(levels: &[usize], rules: &Rules) -> BTreeMap<usize, Role> {
    let order: Vec<Role> = match &rules.heading_roles {
        Some(r) => r.clone(),
        None => match levels.len() {
            0 => Vec::new(),
            1 => vec![Role::Chapter],
            2 => vec![Role::Part, Role::Chapter],
            _ => vec![Role::Part, Role::Chapter, Role::Scene],
        },
    };
    levels.iter().copied().zip(order).collect()
}

/// The text of a paragraph's character properties, as kept marks; what was dropped is counted.
fn marks(p: &CharProps, rules: &Rules, main_lang: Option<&str>, dropped: &mut BTreeMap<String, usize>) -> Marks {
    let mut m = mdoc::marks_of(p);
    if m.lang.as_deref() == main_lang {
        m.lang = None;
    }
    m.link = None;
    m.comment = None;
    let underlined = p.underline.is_some_and(|u| u != jcj_doc::props::Underline::None);
    if underlined {
        if rules.underline_as_italic {
            m.italic = true;
            *dropped.entry("underline (kept as italic)".into()).or_default() += 1;
        } else {
            *dropped.entry("underline".into()).or_default() += 1;
        }
    }
    if p.font.is_some() || p.size.is_some() {
        *dropped.entry("fonts and sizes".into()).or_default() += 1;
    }
    if p.color.is_some() || p.highlight.is_some() || p.shading.is_some() {
        *dropped.entry("colors".into()).or_default() += 1;
    }
    m
}

/// The language most of a document's text is marked with (it is not repeated as a mark).
fn main_language(doc: &Document) -> Option<String> {
    let mut count: BTreeMap<String, usize> = BTreeMap::new();
    for b in &doc.body {
        if let Some(p) = b.as_para() {
            for (r, props) in p.run_ranges() {
                if let Some(l) = &props.lang {
                    *count.entry(l.clone()).or_default() += r.len();
                }
            }
        }
    }
    let default = doc.styles.default_chr.lang.clone();
    count.into_iter().max_by_key(|(_, n)| *n).map(|(l, _)| l).or(default)
}

/// Reads one paragraph into inlines, collecting its notes and comments.
struct Reader<'a> {
    doc: &'a Document,
    rules: &'a Rules,
    main_lang: Option<String>,
    dropped: BTreeMap<String, usize>,
    open_comments: Vec<u32>,
}

/// What a paragraph refers to: notes (label, part id) and comments (id).
#[derive(Default)]
struct Refs {
    notes: Vec<(String, u32)>,
    comments: Vec<u32>,
}

impl Reader<'_> {
    fn inlines(&mut self, p: &Paragraph, refs: &mut Refs, next_note: &mut usize) -> Inlines {
        let mut out: Inlines = Vec::new();
        let objects: BTreeMap<usize, &InlineObject> = p.objects_with_offsets().collect();
        for (range, props) in p.run_ranges() {
            let Some(text) = p.text.get(range.clone()) else { continue };
            let base = marks(props, self.rules, self.main_lang.as_deref(), &mut self.dropped);
            let mut buf = String::new();
            let flush = |buf: &mut String, out: &mut Inlines, comment: Option<u32>| {
                if !buf.is_empty() {
                    let mut m = base.clone();
                    m.comment = comment.map(|c| format!("c{c}"));
                    out.push(Inline::Span { text: std::mem::take(buf), marks: m });
                }
            };
            for (i, c) in text.char_indices() {
                if c != jcj_doc::para::OBJ {
                    buf.push(c);
                    continue;
                }
                flush(&mut buf, &mut out, self.open_comments.last().copied());
                match objects.get(&(range.start + i)) {
                    Some(InlineObject::NoteRef { id, .. }) => {
                        *next_note += 1;
                        let label = next_note.to_string();
                        refs.notes.push((label.clone(), *id));
                        out.push(Inline::NoteRef { label, marks: Marks::default() });
                    }
                    Some(InlineObject::CommentStart { id, .. }) => {
                        self.open_comments.push(*id);
                        refs.comments.push(*id);
                    }
                    Some(InlineObject::CommentEnd { id, .. }) => self.open_comments.retain(|c| c != id),
                    Some(other) => {
                        let kind = format!("{other:?}");
                        let name = kind.split(|c: char| !c.is_alphanumeric()).next().unwrap_or("object").to_owned();
                        if !name.starts_with("Bookmark") && !name.starts_with("Field") {
                            *self.dropped.entry(format!("embedded {name}")).or_default() += 1;
                        }
                    }
                    None => {}
                }
            }
            flush(&mut buf, &mut out, self.open_comments.last().copied());
        }
        wordsmith_manuscript_model::text::normalize_inlines(&mut out);
        out
    }

    /// A part's paragraphs (a note's or a comment's text).
    fn part_paras(&mut self, id: u32) -> Vec<Inlines> {
        let Some(part) = self.doc.parts.get(&id) else { return Vec::new() };
        let mut out = Vec::new();
        let mut dummy = Refs::default();
        let mut n = 0;
        for b in &part.blocks {
            if let Some(p) = b.as_para() {
                let c = self.inlines(p, &mut dummy, &mut n);
                if !c.is_empty() {
                    out.push(c);
                }
            }
        }
        out
    }
}

/// A node being built.
#[derive(Default)]
struct Node {
    title: String,
    title_from_source: bool,
    role: Role,
    /// Text segments split at scene breaks.
    segments: Vec<Vec<Block>>,
    notes: Vec<(String, u32)>,
    comments: Vec<u32>,
    children: Vec<Node>,
}

impl Node {
    fn new(title: String, from_source: bool, role: Role) -> Node {
        Node { title, title_from_source: from_source, role, segments: vec![Vec::new()], ..Node::default() }
    }

    fn has_text(&self) -> bool {
        self.segments.iter().any(|s| !s.is_empty())
    }
}

/// Make a plan from an imported document.
pub fn plan(doc: &Document, rules: &Rules) -> Plan {
    let mut reader = Reader { doc, rules, main_lang: main_language(doc), dropped: BTreeMap::new(), open_comments: Vec::new() };
    let paras: Vec<&Paragraph> = doc.body.iter().filter_map(|b| b.as_para()).collect();
    if doc.body.iter().any(|b| b.as_table().is_some()) {
        *reader.dropped.entry("tables (their text kept as paragraphs)".into()).or_default() += 1;
    }
    let kinds: Vec<Line> = all_paragraphs(doc).iter().map(|p| line(doc, p, rules)).collect();
    let mut levels: Vec<usize> = kinds.iter().filter_map(|k| if let Line::Heading(l) = k { Some(*l) } else { None }).collect();
    levels.sort_unstable();
    levels.dedup();
    let roles = roles_for(&levels, rules);
    let _ = paras;
    // The tree: front matter, then parts, chapters and scenes.
    let mut root = Node::new(String::new(), false, Role::None);
    let mut front = Node::new("Front Matter".into(), false, Role::FrontMatter);
    // Path of open nodes by role rank: part(0), chapter(1), scene(2).
    let mut part: Option<Node> = None;
    let mut chapter: Option<Node> = None;
    let mut scene: Option<Node> = None;
    let mut next_note = 0usize;
    fn close_scene(scene: &mut Option<Node>, chapter: &mut Option<Node>, part: &mut Option<Node>, root: &mut Node) {
        if let Some(s) = scene.take() {
            match (chapter.as_mut(), part.as_mut()) {
                (Some(c), _) => c.children.push(s),
                (None, Some(p)) => p.children.push(s),
                (None, None) => root.children.push(s),
            }
        }
    }
    fn close_chapter(scene: &mut Option<Node>, chapter: &mut Option<Node>, part: &mut Option<Node>, root: &mut Node) {
        close_scene(scene, chapter, part, root);
        if let Some(c) = chapter.take() {
            match part.as_mut() {
                Some(p) => p.children.push(c),
                None => root.children.push(c),
            }
        }
    }
    fn close_part(scene: &mut Option<Node>, chapter: &mut Option<Node>, part: &mut Option<Node>, root: &mut Node) {
        close_chapter(scene, chapter, part, root);
        if let Some(p) = part.take() {
            root.children.push(p);
        }
    }
    for (p, kind) in all_paragraphs(doc).into_iter().zip(kinds) {
        let role = match &kind {
            Line::Heading(l) => roles.get(l).copied(),
            Line::Starts(r) => Some(*r),
            _ => None,
        };
        let title = p.text.replace(jcj_doc::para::OBJ, "").trim().to_owned();
        match role {
            Some(Role::Part) => {
                close_part(&mut scene, &mut chapter, &mut part, &mut root);
                part = Some(Node::new(title, true, Role::Part));
                continue;
            }
            Some(Role::Chapter) => {
                close_chapter(&mut scene, &mut chapter, &mut part, &mut root);
                chapter = Some(Node::new(title, true, Role::Chapter));
                continue;
            }
            Some(Role::Scene) => {
                close_scene(&mut scene, &mut chapter, &mut part, &mut root);
                scene = Some(Node::new(title, true, Role::Scene));
                continue;
            }
            _ => {}
        }
        let target: &mut Node = if let Some(s) = scene.as_mut() {
            s
        } else if let Some(c) = chapter.as_mut() {
            c
        } else if let Some(pt) = part.as_mut() {
            pt
        } else {
            &mut front
        };
        if kind == Line::Break {
            // Text before any part or chapter is split at breaks too: if no structure turns up,
            // the pieces become items of their own (IMP-5).
            if rules.split_scenes && (target.role == Role::Chapter || target.role == Role::FrontMatter) {
                target.segments.push(Vec::new());
            } else if let Some(seg) = target.segments.last_mut() {
                seg.push(Block::SceneBreak);
            }
            continue;
        }
        let mut refs = Refs::default();
        let content = reader.inlines(p, &mut refs, &mut next_note);
        let block = match kind {
            // A heading below the roles stays a heading in the text.
            Line::Heading(l) => Block::Heading { level: l.clamp(1, 3) as u8, content },
            _ => Block::Para(content),
        };
        if let Some(seg) = target.segments.last_mut() {
            seg.push(block);
        }
        target.notes.extend(refs.notes);
        target.comments.extend(refs.comments);
    }
    close_part(&mut scene, &mut chapter, &mut part, &mut root);
    let mut items = Vec::new();
    let structured = !root.children.is_empty();
    if !structured && front.has_text() {
        // No parts or chapters: one text, or one item per piece between separator lines.
        let n = front.segments.iter().filter(|s| !s.is_empty()).count();
        let Node { segments, notes, comments, .. } = front;
        let mut k = 0;
        for seg in segments.into_iter().filter(|s| !s.is_empty()) {
            k += 1;
            let text = item_text(&mut reader, seg, &notes, &comments);
            let (title, role) = if n == 1 { ("Imported Text".to_owned(), Role::None) } else { (format!("Chapter {k}"), Role::Chapter) };
            items.push(Planned { title, title_from_source: false, role, kind: Kind::Text, depth: 0, text });
        }
    } else if front.has_text() {
        let mut front = front;
        // Front matter keeps its breaks as breaks.
        let segs = std::mem::take(&mut front.segments);
        let mut joined: Vec<Block> = Vec::new();
        for (i, seg) in segs.into_iter().enumerate() {
            if i > 0 {
                joined.push(Block::SceneBreak);
            }
            joined.extend(seg);
        }
        front.segments = vec![joined];
        flatten(&mut reader, front, 0, &mut items);
    }
    for child in root.children {
        flatten(&mut reader, child, 0, &mut items);
    }
    let mut report = Report { dropped: reader.dropped, ..Report::default() };
    for it in &items {
        *report.made.entry(it.role.title().to_owned()).or_default() += 1;
    }
    report.check = Some(Check::of(&source_words(doc, rules), &plan_words(&items)));
    report.notes_check = Some(Check::of(&source_note_words(doc), &plan_note_words(&items)));
    Plan { items, report }
}

/// Every paragraph of the main text, table cells included, in reading order.
fn all_paragraphs(doc: &Document) -> Vec<&Paragraph> {
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

fn item_text(reader: &mut Reader<'_>, blocks: Vec<Block>, notes: &[(String, u32)], comments: &[u32]) -> ItemText {
    let mut text = Text { blocks, notes: Vec::new() };
    // Only the notes this text refers to.
    let referenced: Vec<String> = text
        .blocks
        .iter()
        .flat_map(|b| match b {
            Block::Para(c) | Block::Heading { content: c, .. } => c.clone(),
            _ => Vec::new(),
        })
        .filter_map(|i| if let Inline::NoteRef { label, .. } = i { Some(label) } else { None })
        .collect();
    for (label, part) in notes {
        if referenced.contains(label) {
            text.notes.push(Note { label: label.clone(), paras: reader.part_paras(*part) });
        }
    }
    text.normalize();
    let mut meta = ItemMeta::default();
    for c in comments {
        if let Some(com) = reader.doc.comments.get(c) {
            let body = reader.part_paras(com.part).iter().map(|p| wordsmith_manuscript_model::text::plain(p)).collect::<Vec<_>>().join("\n\n");
            meta.comments.insert(format!("c{c}"), Comment { text: body, by: com.author.clone(), date: com.date.clone() });
        }
    }
    ItemText { meta, body: text }
}

fn flatten(reader: &mut Reader<'_>, node: Node, depth: usize, out: &mut Vec<Planned>) {
    let split = node.role == Role::Chapter && node.segments.len() > 1 && reader.rules.split_scenes;
    let has_children = !node.children.is_empty();
    let kind = if split || has_children { Kind::Folder } else { Kind::Text };
    let Node { title, title_from_source, role, segments, notes, comments, children } = node;
    if split {
        out.push(Planned { title, title_from_source, role, kind: Kind::Folder, depth, text: ItemText::default() });
        let n = segments.len();
        for (i, seg) in segments.into_iter().enumerate() {
            let text = item_text(reader, seg, &notes, &comments);
            if text.body.is_empty() && n > 1 && (i == 0 || i == n - 1) {
                continue;
            }
            out.push(Planned {
                title: format!("Scene {}", i + 1),
                title_from_source: false,
                role: Role::Scene,
                kind: Kind::Text,
                depth: depth + 1,
                text,
            });
        }
    } else {
        let blocks: Vec<Block> = segments.into_iter().flatten().collect();
        let text = item_text(reader, blocks, &notes, &comments);
        let title = if title.is_empty() { role.title().to_owned() } else { title };
        out.push(Planned { title, title_from_source, role, kind, depth, text });
    }
    for c in children {
        flatten(reader, c, depth + 1, out);
    }
}

/// The source's words, in reading order, as the plan will hold them: headings that become
/// titles count as words; scene-break lines are not words.
pub fn source_words(doc: &Document, rules: &Rules) -> Vec<String> {
    all_paragraphs(doc).into_iter().filter(|p| line(doc, p, rules) != Line::Break).flat_map(verify::paragraph_words).collect()
}

/// A plan's words in reading order: each item's title when it came from the source, then its
/// text (notes left out).
pub fn plan_words(items: &[Planned]) -> Vec<String> {
    let mut out = Vec::new();
    for it in items {
        if it.title_from_source {
            out.extend(verify::words(&it.title));
        }
        let body = Text { blocks: it.text.body.blocks.clone(), notes: Vec::new() };
        out.extend(body.plain_paragraphs().iter().flat_map(|p| verify::words(p)));
    }
    out
}

/// The words of the document's notes, in the order they are referred to.
pub fn source_note_words(doc: &Document) -> Vec<String> {
    let mut out = Vec::new();
    for p in all_paragraphs(doc) {
        for (_, o) in p.objects_with_offsets() {
            if let InlineObject::NoteRef { id, .. } = o
                && let Some(part) = doc.parts.get(id)
                && matches!(part.kind, PartKind::Footnote | PartKind::Endnote)
            {
                for b in &part.blocks {
                    if let Some(np) = b.as_para() {
                        out.extend(verify::paragraph_words(np));
                    }
                }
            }
        }
    }
    out
}

/// A plan's note words, item by item.
pub fn plan_note_words(items: &[Planned]) -> Vec<String> {
    items
        .iter()
        .flat_map(|it| {
            it.text.body.notes.iter().flat_map(|n| n.paras.iter().flat_map(|p| verify::words(&wordsmith_manuscript_model::text::plain(p))))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chapter_lines_and_breaks() {
        assert_eq!(starts("Chapter 12"), Some(Role::Chapter));
        assert_eq!(starts("CHAPTER XII."), Some(Role::Chapter));
        assert_eq!(starts("Part Two"), Some(Role::Part));
        assert_eq!(starts("Chapter and verse were quoted at length by the vicar."), None);
        assert_eq!(starts("Chapters"), None);
        for b in ["***", "* * *", "#", "~", "§§§", "———"] {
            assert!(is_break(b, false, &[]), "{b}");
        }
        assert!(!is_break("*!*", false, &[]));
        assert!(is_break("*!*", true, &[]));
        assert!(!is_break("Chapter", false, &[]));
        assert!(is_break("[break]", false, &["[break]".into()]));
    }
}
