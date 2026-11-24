// SPDX-License-Identifier: MIT OR Apache-2.0
//! Reading a document file into a binder plan: DOCX, ODT, RTF, Markdown, HTML and plain text
//! (IMP-1, IMP-3, IMP-4, IMP-5). Files go through the core's `jcj-interchange`; plain text is
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
    jcj_interchange::import(format, &bytes).map_err(|e| format!("{}: {e}", path.display()))
}

/// Read a file into a plan.
pub fn plan_file(path: &Path, rules: &Rules, text_rules: &TextRules) -> Result<Plan, String> {
    Ok(mapping::plan(&document(path, text_rules)?, rules))
}
