// SPDX-License-Identifier: MIT OR Apache-2.0
//! The standard manuscript format (EXP-1, EXP-2; SPEC 16.2): Letter or A4 with 1-inch margins,
//! 12-point serif or monospaced type, double spacing, half-inch first-line indents, a header with
//! surname, short title and page number on every page after the title page (numbered from 1 on
//! the first page of text), chapters starting a third of the way down a new page, centered scene
//! breaks, a title page with the rounded word count, and an END line.
//!
//! The layout is a [`PaginationPass`] (`jcj-layout`) over a document built from the compiled
//! manuscript; the header is filled by `jcj-features` through a [`FieldSource`] giving the surname
//! and the short title. DOCX is written from the same document through `jcj-interchange`; PDF
//! through `jcj-output`.

use std::sync::Arc;

use jcj_doc::props::{LineSpacing, TabAlign, TabLeader, TabStop};
use jcj_doc::{Align, CharProps, Document, Paragraph, PartKind, para_block};
use jcj_features::header_footer::{Band, FieldSource, FieldValue, band_line};
use jcj_layout::paginate::{PaginationPass, Paginator};
use serde::{Deserialize, Serialize};
use wordsmith_manuscript_model::{Inline, Project};

use crate::compile::{self, Compiled, Options, ParaKind, Piece};
use crate::verify::{self, Check};

/// Points per inch.
pub const INCH: f32 = 72.0;

/// Paper.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Paper {
    /// US Letter, 8.5 × 11 inches.
    #[default]
    Letter,
    /// A4, 210 × 297 mm.
    A4,
}

impl Paper {
    /// Width and height in points.
    pub fn size(self) -> (f32, f32) {
        match self {
            Paper::Letter => (8.5 * INCH, 11.0 * INCH),
            Paper::A4 => (595.28, 841.89),
        }
    }
}

/// The typeface.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Face {
    /// A serif face (the core's bundled Source Serif 4).
    #[default]
    Serif,
    /// A monospaced face (the core's bundled JetBrains Mono).
    Mono,
}

impl Face {
    /// The face's family name.
    pub fn family(self) -> &'static str {
        match self {
            Face::Serif => "Source Serif 4",
            Face::Mono => "JetBrains Mono",
        }
    }
}

/// The standard manuscript's own settings (with the export [`Options`]).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Standard {
    /// Paper.
    pub paper: Paper,
    /// Typeface.
    pub face: Face,
    /// A title page.
    pub title_page: bool,
    /// Italic shown as underline (older submission guides).
    pub underline_italic: bool,
    /// A fixed creation date for the PDF (`2000-01-01T00:00:00`), for output that must be the
    /// same every time; empty: today.
    pub created: String,
}

impl Default for Standard {
    fn default() -> Self {
        Standard { paper: Paper::Letter, face: Face::Serif, title_page: true, underline_italic: false, created: String::new() }
    }
}

/// The word count as the title page shows it (SPEC 16.2): under 1,000 to the nearest 100, under
/// 25,000 to the nearest 500, else to the nearest 1,000.
pub fn rounded(words: usize) -> usize {
    let step = if words < 1000 {
        100
    } else if words < 25_000 {
        500
    } else {
        1000
    };
    if words == 0 {
        return 0;
    }
    ((words + step / 2) / step * step).max(step)
}

/// `82,000`.
pub fn thousands(n: usize) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// The header's own fields: the surname and the short title.
pub struct Fields {
    /// The surname.
    pub surname: String,
    /// The short title.
    pub short_title: String,
}

impl FieldSource for Fields {
    fn field(&self, name: &str, _doc: &Document) -> Option<FieldValue> {
        match name {
            "surname" => Some(FieldValue::Text(self.surname.clone())),
            "short" => Some(FieldValue::Text(self.short_title.clone())),
            _ => None,
        }
    }

    fn names(&self) -> Vec<String> {
        vec!["surname".into(), "short".into()]
    }
}

fn props(face: Face) -> CharProps {
    CharProps { font: Some(face.family().into()), size: Some(12.0), ..CharProps::default() }
}

fn line(text: &str, face: Face, align: Option<Align>) -> Paragraph {
    let mut p = Paragraph::with_text(text, props(face));
    p.props.align = align;
    p.props.line_spacing = Some(LineSpacing::Multiple(2.0));
    p.props.space_before = Some(0.0);
    p.props.space_after = Some(0.0);
    p
}

/// The pagination pass: chapter and part headings keep with what follows, so are scene breaks.
pub struct ManuscriptPass;

impl PaginationPass for ManuscriptPass {
    fn prepare(&mut self, doc: &mut Document) {
        for b in doc.body.iter_mut() {
            if let jcj_doc::Block::Para(p) = Arc::make_mut(b) {
                let is_heading = p.props.outline_level.is_some();
                let is_break = p.props.align == Some(Align::Center) && !p.text.chars().any(char::is_alphanumeric);
                if is_heading || is_break {
                    p.props.keep_next = Some(true);
                }
                // Widows and orphans are left to the layout's own control.
                p.props.widow_control = Some(true);
            }
        }
    }
}

/// The standard manuscript document for a compiled manuscript.
pub fn document(p: &Project, c: &Compiled, o: &Options, s: &Standard) -> Document {
    let mut doc = jcj_doc::new_document();
    doc.body.clear();
    let (w, h) = s.paper.size();
    let face = s.face;
    let book = &p.settings.book;
    doc.core.title.clone_from(&book.title);
    doc.core.creator = book.byline().to_owned();
    if !s.created.is_empty() {
        doc.core.created.clone_from(&s.created);
        doc.core.modified.clone_from(&s.created);
    }
    let mut section = doc.last_section.clone();
    section.page_w = w;
    section.page_h = h;
    section.margin_top = INCH;
    section.margin_bottom = INCH;
    section.margin_left = INCH;
    section.margin_right = INCH;
    section.header = 0.5 * INCH;
    section.footer = 0.5 * INCH;
    let third = (h / 3.0 - INCH).max(0.0);
    let mut body: Vec<Paragraph> = Vec::new();
    // The title page.
    if s.title_page {
        let words = c.content_words();
        let count = format!("about {} words", thousands(rounded(words)));
        let mut first = line(&format!("{}\t{count}", if book.author.is_empty() { book.byline() } else { &book.author }), face, None);
        first.props.line_spacing = Some(LineSpacing::Multiple(1.0));
        first.props.tabs = Some(vec![TabStop { pos: w - 2.0 * INCH, align: TabAlign::Right, leader: TabLeader::None }]);
        body.push(first);
        for c in &book.contact {
            let mut l = line(c, face, None);
            l.props.line_spacing = Some(LineSpacing::Multiple(1.0));
            body.push(l);
        }
        let mut t = line(&book.title, face, Some(Align::Center));
        t.props.space_before = Some(third.max(2.0 * INCH));
        body.push(t);
        if !book.subtitle.is_empty() {
            body.push(line(&book.subtitle, face, Some(Align::Center)));
        }
        body.push(line("by", face, Some(Align::Center)));
        let mut by = line(book.byline(), face, Some(Align::Center));
        // The title page is a section of its own: no header, and the text's pages count from 1.
        let mut title_section = section.clone();
        title_section.headers = Default::default();
        title_section.footers = Default::default();
        title_section.title_page = false;
        by.section = Some(Box::new(title_section));
        body.push(by);
    }
    let mut chapter_page = false;
    for piece in &c.pieces {
        match piece {
            Piece::Heading { level, text, .. } => {
                let mut p = line(text, face, Some(Align::Center));
                p.props.outline_level = Some(level.saturating_sub(1));
                p.props.style = Some(format!("Heading{level}"));
                if *level <= 2 {
                    p.props.page_break_before = Some(!body.is_empty() && !(chapter_page && *level == 2));
                    p.props.space_before = Some(third);
                    chapter_page = *level == 1;
                }
                body.push(p);
            }
            Piece::Para { content, kind, .. } => {
                chapter_page = false;
                let mut p = Paragraph::new();
                p.props.line_spacing = Some(LineSpacing::Multiple(2.0));
                p.props.space_before = Some(0.0);
                p.props.space_after = Some(0.0);
                match kind {
                    ParaKind::Body { .. } => p.props.indent_first = Some(0.5 * INCH),
                    ParaKind::Quote | ParaKind::Verse | ParaKind::Aside => {
                        p.props.indent_left = Some(INCH);
                        p.props.indent_right = Some(0.5 * INCH);
                    }
                    ParaKind::Heading(_) => p.props.align = Some(Align::Center),
                }
                for i in content {
                    if let Inline::Span { text, marks } = i {
                        let mut cp = props(face);
                        let m = compile::export_marks(marks);
                        cp.bold = m.bold.then_some(true);
                        cp.small_caps = m.small_caps.then_some(true);
                        cp.strike = m.strike.then_some(true);
                        if m.italic {
                            if s.underline_italic {
                                cp.underline = Some(jcj_doc::props::Underline::Single);
                            } else {
                                cp.italic = Some(true);
                            }
                        }
                        cp.lang.clone_from(&m.lang);
                        let end = p.text.len();
                        let _ = p.insert_text(end, text, &cp);
                    }
                }
                p.normalize();
                body.push(p);
            }
            Piece::SceneBreak => body.push(line(&o.scene_break, face, Some(Align::Center))),
            Piece::End => body.push(line("END", face, Some(Align::Center))),
            // Notes print at the end of the item that refers to them, as numbered paragraphs.
            Piece::Notes(ns) => {
                for n in ns {
                    let text = n.paras.iter().map(|p| wordsmith_manuscript_model::text::plain(p)).collect::<Vec<_>>().join(" ");
                    let mut p = line(&text, face, None);
                    p.props.indent_left = Some(0.5 * INCH);
                    body.push(p);
                }
            }
            Piece::Image { .. } | Piece::Raw { .. } => {}
        }
    }
    if body.is_empty() {
        body.push(line("", face, None));
    }
    doc.body = body.into_iter().map(|p| Arc::new(jcj_doc::Block::Para(p))).collect();
    // The text's section: the header on every page, numbered from 1.
    let fields = Fields { surname: book.header_surname(), short_title: book.header_title().to_uppercase() };
    let band = Band { right: "{surname} / {short} / {page}".into(), ..Band::default() };
    let header_line = band_line(&doc, &band, None, &fields);
    let header = doc.add_part(PartKind::Header, vec![para_block(header_line)]);
    section.headers.default = Some(header);
    section.title_page = false;
    if s.title_page {
        section.page_num_start = Some(1);
        section.page_num_set = true;
    }
    doc.last_section = section;
    doc
}

/// A standard manuscript file and its check.
pub struct Written {
    /// The bytes.
    pub bytes: Vec<u8>,
    /// The word-for-word check against the manuscript (read back from the bytes).
    pub check: Check,
    /// How many pages (PDF).
    pub pages: usize,
}

impl Compiled {
    /// The words of the manuscript's text alone (no titles made up by the export): what the
    /// title page's count counts.
    pub fn content_words(&self) -> usize {
        self.pieces
            .iter()
            .map(|p| match p {
                Piece::Para { content, .. } => wordsmith_manuscript_model::words::count(&wordsmith_manuscript_model::text::plain(content)),
                Piece::Notes(ns) => ns
                    .iter()
                    .flat_map(|n| n.paras.iter())
                    .map(|c| wordsmith_manuscript_model::words::count(&wordsmith_manuscript_model::text::plain(c)))
                    .sum(),
                _ => 0,
            })
            .sum()
    }
}

/// The words a standard manuscript holds besides the compiled text: the title page.
fn title_words(p: &Project, c: &Compiled, s: &Standard) -> Vec<String> {
    if !s.title_page {
        return Vec::new();
    }
    let book = &p.settings.book;
    let mut out = verify::words(if book.author.is_empty() { book.byline() } else { &book.author });
    out.extend(verify::words(&format!("about {} words", thousands(rounded(c.content_words())))));
    for line in &book.contact {
        out.extend(verify::words(line));
    }
    out.extend(verify::words(&book.title));
    out.extend(verify::words(&book.subtitle));
    out.push("by".into());
    out.extend(verify::words(book.byline()));
    out
}

/// What a standard manuscript must hold, word for word: the title page, then the text with each
/// item's notes after it.
pub fn expected_words(p: &Project, c: &Compiled, s: &Standard) -> Vec<String> {
    let mut out = title_words(p, c, s);
    for piece in &c.pieces {
        match piece {
            Piece::Heading { text, .. } => out.extend(verify::words(text)),
            Piece::Para { content, .. } => out.extend(verify::words(&wordsmith_manuscript_model::text::plain(content))),
            Piece::Notes(ns) => out.extend(
                ns.iter()
                    .map(|n| n.paras.iter().map(|x| wordsmith_manuscript_model::text::plain(x)).collect::<Vec<_>>().join(" "))
                    .flat_map(|t| verify::words(&t)),
            ),
            Piece::End => out.push("END".into()),
            _ => {}
        }
    }
    out
}

/// The standard manuscript as DOCX.
pub fn docx(p: &Project, o: &Options, s: &Standard) -> Result<Written, String> {
    let c = compile::compile(p, o);
    let doc = document(p, &c, o, s);
    let bytes = jcj_interchange::export(jcj_interchange::Format::Docx, &doc).map_err(|e| e.to_string())?;
    let back = jcj_interchange::import(jcj_interchange::Format::Docx, &bytes).map_err(|e| e.to_string())?;
    let check = Check::of(&expected_words(p, &c, s), &verify::document_words(&back));
    Ok(Written { bytes, check, pages: 0 })
}

/// The standard manuscript laid out: the paginated document and its layout.
pub fn paginate(p: &Project, o: &Options, s: &Standard) -> (Compiled, jcj_layout::paginate::Paginated) {
    let c = compile::compile(p, o);
    let doc = document(p, &c, o, s);
    let pag = Paginator::new().paginate(&doc, &mut ManuscriptPass);
    (c, pag)
}

/// One page of a laid-out manuscript drawn into pixels at `scale` pixels per point: width,
/// height and straight RGBA (for the page view).
pub fn page_pixels(pag: &jcj_layout::paginate::Paginated, page: usize, scale: f32) -> Option<(u32, u32, Vec<u8>)> {
    let img = jcj_output::image::render_page(&pag.doc, &pag.layout, page, scale, &jcj_output::image::RenderSettings::default())?;
    Some((img.width, img.height, img.straight_rgba()))
}

/// How many pages a laid-out manuscript has.
pub fn page_count(pag: &jcj_layout::paginate::Paginated) -> usize {
    pag.layout.pages.len()
}

/// The standard manuscript as PDF.
pub fn pdf(p: &Project, o: &Options, s: &Standard) -> Result<Written, String> {
    let (c, pag) = paginate(p, o, s);
    let settings = jcj_output::PdfSettings {
        title: Some(p.settings.book.title.clone()),
        author: Some(p.settings.book.byline().to_owned()),
        created: (!s.created.is_empty()).then(|| s.created.clone()),
        ..jcj_output::PdfSettings::default()
    };
    let bytes = jcj_output::export_pdf(&pag.doc, &pag.layout, &settings).map_err(|e| e.to_string())?;
    let pages = pdf_pages(&bytes)?;
    // The PDF's letters and digits in order, each page's header line left out (the title page
    // has none).
    let mut found: Vec<&str> = Vec::new();
    for (n, pg) in pages.iter().enumerate() {
        let skip = if n == 0 && s.title_page { 0 } else { 1 };
        found.extend(pg.iter().skip(skip).map(String::as_str));
    }
    let expected = expected_words(p, &c, s);
    let check = Check::of(&letters(expected.iter().map(String::as_str)), &letters(found));
    Ok(Written { pages: pages.len(), bytes, check })
}

/// A PDF's text, page by page, line by line, top to bottom (glyphs grouped by where they land,
/// in drawing order). The core's PDFs place glyphs one by one, so spaces between words may be
/// missing: compare letters with [`letters`], not words.
pub fn pdf_pages(bytes: &[u8]) -> Result<Vec<Vec<String>>, String> {
    let pages = jcj_output::read::read_pages(bytes, &jcj_output::read::ReadLimits::default())
        .map_err(|e| format!("the PDF could not be read back: {e:?}"))?;
    Ok(pages
        .iter()
        .map(|pg| {
            let mut lines: Vec<(f64, String)> = Vec::new();
            for g in pg.glyphs.iter().filter(|g| g.mode == jcj_output::read::GlyphMode::Fill) {
                let t = g.text.as_deref().unwrap_or("");
                match lines.iter_mut().find(|(y, _)| (*y - g.y).abs() < 1.0) {
                    Some((_, l)) => l.push_str(t),
                    None => lines.push((g.y, t.to_owned())),
                }
            }
            lines.sort_by(|a, b| b.0.total_cmp(&a.0));
            lines.into_iter().map(|(_, l)| l).collect()
        })
        .collect())
}

/// The letters and digits of some words, one per entry: the PDF check compares these in order.
pub fn letters<'a>(words: impl IntoIterator<Item = &'a str>) -> Vec<String> {
    words.into_iter().flat_map(|w| w.chars().filter(|c| c.is_alphanumeric())).map(String::from).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn word_counts_round_by_size() {
        assert_eq!(rounded(449), 400);
        assert_eq!(rounded(450), 500);
        assert_eq!(rounded(12_240), 12_000);
        assert_eq!(rounded(12_260), 12_500);
        assert_eq!(rounded(81_640), 82_000);
        assert_eq!(rounded(3), 100);
        assert_eq!(thousands(184_000), "184,000");
    }
}
