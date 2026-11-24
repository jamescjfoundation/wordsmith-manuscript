// SPDX-License-Identifier: MIT OR Apache-2.0
//! Reading a document file into a binder plan: DOCX, ODT, RTF, Markdown, HTML and plain text
//! (IMP-1, IMP-3, IMP-4, IMP-5, IMP-7), and OPML outlines (IMP-8, [`crate::opml`]). Files go through the core's `jcj-interchange`; plain text is
//! read here, joining hard-wrapped lines into paragraphs.

use std::path::Path;

use jcj_doc::{Document, Paragraph};
use jcj_interchange::Format;

use crate::mapping::{self, Plan, Rules};

/// The largest file imported.
pub const MAX_FILE: u64 = jcj_interchange::MAX_IMPORT_BYTES as u64;

/// How plain text is split into paragraphs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextRules {
    /// Lines not separated by a blank line are one paragraph (hard-wrapped text).
    pub join_lines: bool,
}

impl Default for TextRules {
    fn default() -> Self {
        TextRules { join_lines: true }
    }
}

/// Read a file's bytes, bounded.
pub fn read_bounded(path: &Path) -> Result<Vec<u8>, String> {
    let meta = std::fs::metadata(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if meta.len() > MAX_FILE {
        return Err(format!("{} is larger than {} MB", path.display(), MAX_FILE >> 20));
    }
    std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))
}

/// A plain text file as a document: paragraphs at blank lines (or every line).
pub fn text_document(text: &str, rules: &TextRules) -> Document {
    let mut doc = jcj_doc::new_document();
    let text = text.strip_prefix('\u{feff}').unwrap_or(text).replace("\r\n", "\n").replace('\r', "\n");
    let mut paras: Vec<String> = Vec::new();
    if rules.join_lines {
        let mut cur: Vec<&str> = Vec::new();
        for line in text.lines() {
            if line.trim().is_empty() {
                if !cur.is_empty() {
                    paras.push(cur.join(" "));
                    cur.clear();
                }
            } else {
                cur.push(line.trim());
            }
        }
        if !cur.is_empty() {
            paras.push(cur.join(" "));
        }
    } else {
        paras = text.lines().filter(|l| !l.trim().is_empty()).map(|l| l.trim().to_owned()).collect();
    }
    if paras.is_empty() {
        paras.push(String::new());
    }
    doc.body =
        paras.into_iter().map(|t| std::sync::Arc::new(jcj_doc::Block::Para(Paragraph::with_text(&t, jcj_doc::CharProps::default())))).collect();
    doc
}

/// The document a file holds.
pub fn document(path: &Path, text_rules: &TextRules) -> Result<Document, String> {
    let bytes = read_bounded(path)?;
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned());
    let format = Format::detect(name.as_deref(), &bytes).ok_or_else(|| format!("{} is not a format that can be imported", path.display()))?;
    if format == Format::Text {
        let text = String::from_utf8(bytes).map_err(|_| format!("{} is not UTF-8 text", path.display()))?;
        return Ok(text_document(&text, text_rules));
    }
    if !format.can_import() {
        return Err(format!("{} files cannot be imported yet", format.label()));
    }
    if format == Format::Html
        && let Some(part) = std::str::from_utf8(&bytes).ok().and_then(page_text)
    {
        return jcj_interchange::import(format, part.as_bytes()).map_err(|e| format!("{}: {e}", path.display()));
    }
    jcj_interchange::import(format, &bytes).map_err(|e| format!("{}: {e}", path.display()))
}

/// The part of a web page that holds its text (IMP-7): the first `<article>` element, else the
/// first `<main>`, with the page's navigation, header and footer around it left out. `None` when
/// the page has neither (all of it is read).
pub fn page_text(html: &str) -> Option<String> {
    let lower = html.to_ascii_lowercase();
    for name in ["article", "main"] {
        let open = format!("<{name}");
        let mut from = 0;
        while let Some(at) = lower[from..].find(&open).map(|i| i + from) {
            let after = &lower[at + open.len()..];
            if !after.starts_with(|c: char| c == '>' || c.is_whitespace()) {
                from = at + open.len();
                continue;
            }
            let start = at + open.len() + after.find('>')? + 1;
            // The matching close tag, counting nested elements of the same name.
            let mut depth = 1usize;
            let mut i = start;
            let close = format!("</{name}");
            while depth > 0 {
                let next_open = lower[i..].find(&open).map(|x| x + i);
                let next_close = lower[i..].find(&close).map(|x| x + i)?;
                match next_open {
                    Some(o) if o < next_close => {
                        depth += 1;
                        i = o + open.len();
                    }
                    _ => {
                        depth -= 1;
                        if depth == 0 {
                            return Some(format!("<html><body>{}</body></html>", &html[start..next_close]));
                        }
                        i = next_close + close.len();
                    }
                }
            }
        }
    }
    None
}

/// Read a file into a plan.
pub fn plan_file(path: &Path, rules: &Rules, text_rules: &TextRules) -> Result<Plan, String> {
    let ext = path.extension().and_then(|e| e.to_str()).map(str::to_lowercase);
    if ext.as_deref() == Some("opml") {
        let bytes = read_bounded(path)?;
        return crate::opml::plan(&bytes).map_err(|e| format!("{}: {e}", path.display()));
    }
    let mut plan = mapping::plan(&document(path, text_rules)?, rules);
    if matches!(ext.as_deref(), Some("html" | "htm" | "xhtml"))
        && std::fs::read(path).ok().and_then(|b| String::from_utf8(b).ok()).is_some_and(|h| page_text(&h).is_some())
    {
        plan.report.dropped.insert("the page around its article (navigation, header, footer)".into(), 1);
    }
    Ok(plan)
}
