// SPDX-License-Identifier: MIT OR Apache-2.0
//! Find and replace (ED-4, SPEC 5): in the document, the selection, or the whole project, with
//! match case, whole word and regular expressions.
//!
//! Replacing keeps each paragraph's marks: the replacement takes the marks of the first character
//! it replaces. Replace All across the project is one undo step, after a snapshot of every item it
//! changes (INS-5).

use std::ops::Range;

use jcj_doc::Paragraph;
use regex::{Regex, RegexBuilder};
use serde_json::{Value, json};
use wordsmith_manuscript_model::doc as mdoc;
use wordsmith_manuscript_model::{Area, ItemId, Project, Text};

use crate::commands::Spec;
use crate::editor::{Editor, para_index, pos};
use crate::session::{Outcome, Session};

/// What to look for.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Query {
    /// The text or pattern.
    pub text: String,
    /// Match case.
    pub case: bool,
    /// Whole words only.
    pub word: bool,
    /// `text` is a regular expression.
    pub regex: bool,
}

/// The most matches one search reports.
pub const MAX_MATCHES: usize = 100_000;

impl Query {
    /// From command parameters (`query`, `case`, `word`, `regex`).
    pub fn from_params(v: &Value) -> Result<Query, String> {
        let text = v.get("query").and_then(Value::as_str).ok_or("`query` is required")?.to_owned();
        let flag = |k: &str| v.get(k).and_then(Value::as_bool).unwrap_or(false);
        Ok(Query { text, case: flag("case"), word: flag("word"), regex: flag("regex") })
    }

    /// The compiled matcher.
    pub fn matcher(&self) -> Result<Regex, String> {
        if self.text.is_empty() {
            return Err("type something to find".into());
        }
        let mut pattern = if self.regex { self.text.clone() } else { regex::escape(&self.text) };
        if self.word {
            pattern = format!(r"\b(?:{pattern})\b");
        }
        RegexBuilder::new(&pattern).case_insensitive(!self.case).size_limit(1 << 20).build().map_err(|e| format!("not a regular expression: {e}"))
    }
}

/// The matches in one piece of text (non-empty only).
pub fn matches(re: &Regex, text: &str) -> Vec<Range<usize>> {
    re.find_iter(text).filter(|m| !m.is_empty()).map(|m| m.range()).take(MAX_MATCHES).collect()
}

/// The replacement text for one match (`$1` groups for a regular expression).
fn replacement(re: &Regex, q: &Query, found: &str, with: &str) -> String {
    if q.regex {
        let mut out = String::new();
        if let Some(caps) = re.captures(found) {
            caps.expand(with, &mut out);
            return out;
        }
    }
    with.to_owned()
}

/// Replace every match in a paragraph, keeping marks. Returns how many.
pub fn replace_in_paragraph(p: &mut Paragraph, re: &Regex, q: &Query, with: &str, within: Option<Range<usize>>) -> usize {
    let text = p.text.clone();
    let found: Vec<Range<usize>> =
        matches(re, &text).into_iter().filter(|r| within.as_ref().is_none_or(|w| r.start >= w.start && r.end <= w.end)).collect();
    for r in found.iter().rev() {
        let props = p.props_of_char(r.start).clone();
        let rep = replacement(re, q, text.get(r.clone()).unwrap_or(""), with);
        if p.delete(r.start, r.end).is_ok() && !rep.is_empty() {
            let _ = p.insert_text(r.start, &rep, &props);
        }
    }
    found.len()
}

/// Replace in a whole text (body and notes). Returns the new text and the count.
pub fn replace_in_text(t: &Text, re: &Regex, q: &Query, with: &str) -> (Text, usize) {
    let mut doc = mdoc::to_document(t);
    let mut n = 0;
    for b in doc.body.iter_mut() {
        if let jcj_doc::Block::Para(p) = std::sync::Arc::make_mut(b) {
            n += replace_in_paragraph(p, re, q, with, None);
        }
    }
    if n == 0 { (t.clone(), 0) } else { (mdoc::from_document(&doc), n) }
}

/// A match found in the project.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Found {
    /// The item.
    pub item: ItemId,
    /// Where: `text`, `synopsis` or `notes`.
    pub field: &'static str,
    /// The paragraph (in the text's reading order).
    pub para: usize,
    /// The bytes in that paragraph.
    pub range: Range<usize>,
    /// The paragraph's text around the match, for a results list.
    pub context: String,
}

fn context(text: &str, r: &Range<usize>) -> String {
    let mut a = r.start.saturating_sub(40);
    while !text.is_char_boundary(a) {
        a -= 1;
    }
    let mut b = (r.end + 40).min(text.len());
    while !text.is_char_boundary(b) {
        b += 1;
    }
    let s = text.get(a..b).unwrap_or("").replace('\n', " ");
    format!("{}{s}{}", if a > 0 { "…" } else { "" }, if b < text.len() { "…" } else { "" })
}

/// Every match in the project's text items, synopses and notes, in binder order (Trash last).
pub fn find_in_project(p: &Project, re: &Regex) -> Vec<Found> {
    let mut out = Vec::new();
    for area in Area::ALL {
        for row in p.binder.rows_of(area) {
            let id = row.item.id;
            if let Some(t) = p.texts.get(&id) {
                for (i, para) in t.body.plain_paragraphs().iter().enumerate() {
                    for r in matches(re, para) {
                        out.push(Found { item: id, field: "text", para: i, context: context(para, &r), range: r });
                    }
                }
                for (field, s) in [("synopsis", &t.meta.synopsis), ("notes", &t.meta.notes)] {
                    for r in matches(re, s) {
                        out.push(Found { item: id, field, para: 0, context: context(s, &r), range: r });
                    }
                }
            }
            if let Some(e) = p.entries.get(&id) {
                for (i, para) in e.body.plain_paragraphs().iter().enumerate() {
                    for r in matches(re, para) {
                        out.push(Found { item: id, field: "text", para: i, context: context(para, &r), range: r });
                    }
                }
            }
            if out.len() >= MAX_MATCHES {
                return out;
            }
        }
    }
    out
}

/// The find commands.
pub fn specs() -> Vec<Spec> {
    let q = r#"{"query": string, "case"?: bool, "word"?: bool, "regex"?: bool"#;
    let _ = q;
    vec![
        Spec {
            id: "find.project",
            label: "Find in Project",
            key: "Mod+Shift+F",
            params: r#"{"query": string, "case"?: bool, "word"?: bool, "regex"?: bool}"#,
            about: "Every match in the project's texts, synopses and notes.",
            mutates: false,
        },
        Spec {
            id: "find.replaceProject",
            label: "Replace All in Project",
            key: "",
            params: r#"{"query": string, "with": string, "case"?: bool, "word"?: bool, "regex"?: bool}"#,
            about: "Replace every match in the project's texts (one undo step, a snapshot of each changed item first).",
            mutates: true,
        },
    ]
}

/// Run a find command on the project, or `None` if `id` is not one.
pub fn run(s: &mut Session, id: &str, v: &Value) -> Option<Outcome> {
    Some(match id {
        "find.project" => (|| {
            let q = Query::from_params(v)?;
            let re = q.matcher()?;
            let found = find_in_project(&s.project, &re);
            Ok(json!({
                "count": found.len(),
                "matches": found.iter().take(1000).map(|f| json!({
                    "item": f.item.as_text(),
                    "title": s.project.binder.get(f.item).map(|i| i.title.clone()).unwrap_or_default(),
                    "field": f.field,
                    "paragraph": f.para,
                    "start": f.range.start,
                    "end": f.range.end,
                    "context": f.context,
                })).collect::<Vec<_>>(),
            }))
        })(),
        "find.replaceProject" => (|| {
            let q = Query::from_params(v)?;
            let with = v.get("with").and_then(Value::as_str).ok_or("`with` is required")?.to_owned();
            let re = q.matcher()?;
            let mut changes: Vec<(ItemId, Text, usize)> = Vec::new();
            for (id, t) in &s.project.texts {
                if s.project.binder.area_of(*id) == Some(Area::Trash) {
                    continue;
                }
                let (nt, n) = replace_in_text(&t.body, &re, &q, &with);
                if n > 0 {
                    changes.push((*id, nt, n));
                }
            }
            let ids: Vec<ItemId> = changes.iter().map(|c| c.0).collect();
            let total: usize = changes.iter().map(|c| c.2).sum();
            if total == 0 {
                return Ok(json!({ "replaced": 0, "items": 0 }));
            }
            // A snapshot of every item before it changes (INS-5).
            let label = format!("Before replacing “{}”", q.text);
            for id in &ids {
                crate::snapshots::take(s, *id, &label);
            }
            s.change("Replace All", &ids, &[], |p| {
                for (id, t, _) in changes {
                    p.text_mut(id).body = t;
                }
                Ok(())
            })?;
            Ok(json!({ "replaced": total, "items": ids.len() }))
        })(),
        _ => return None,
    })
}

/// Find and replace in an open editor: the document or the selection.
impl Editor {
    /// The matches in the editor's text paragraphs (not dividers), within the selection when
    /// `selection` is set.
    pub fn find(&self, q: &Query, selection: bool) -> Result<Vec<(usize, Range<usize>)>, String> {
        let re = q.matcher()?;
        let (a, b) = self.session.sel.ordered();
        let mut out = Vec::new();
        for (i, p) in self.paragraphs().iter().enumerate() {
            if crate::stitch::is_divider(p) {
                continue;
            }
            if selection && (i < para_index(&a) || i > para_index(&b)) {
                continue;
            }
            for r in matches(&re, &p.text) {
                if selection {
                    let from = if i == para_index(&a) { a.off } else { 0 };
                    let to = if i == para_index(&b) { b.off } else { p.text.len() };
                    if r.start < from || r.end > to {
                        continue;
                    }
                }
                out.push((i, r));
            }
        }
        Ok(out)
    }

    /// Select the next match after the caret (or before it, going back), wrapping around.
    pub fn find_next(&mut self, q: &Query, back: bool) -> Result<bool, String> {
        let all = self.find(q, false)?;
        let (a, b) = self.session.sel.ordered();
        let here = if back { (para_index(&a), a.off) } else { (para_index(&b), b.off) };
        let pick = if back {
            all.iter().rev().find(|(i, r)| (*i, r.end) <= here && (*i, r.start) < here).or_else(|| all.last())
        } else {
            all.iter().find(|(i, r)| (*i, r.start) >= here && (*i, r.end) > here).or_else(|| all.first())
        };
        let Some((i, r)) = pick.cloned() else { return Ok(false) };
        self.session.sel = jcj_engine::Selection { anchor: pos(i, r.start), focus: pos(i, r.end) };
        self.reveal = true;
        Ok(true)
    }

    /// Replace every match in the document (or the selection), as one undo step.
    pub fn replace_all(&mut self, q: &Query, with: &str, selection: bool) -> Result<usize, String> {
        if self.read_only {
            return Err("this text is being edited in a stitched view".into());
        }
        let re = q.matcher()?;
        let found = self.find(q, selection)?;
        if found.is_empty() {
            return Ok(0);
        }
        let (a, b) = self.session.sel.ordered();
        self.session.checkpoint("Replace All");
        let mut paras: Vec<usize> = found.iter().map(|(i, _)| *i).collect();
        paras.dedup();
        let mut n = 0;
        for i in paras {
            let within = if selection {
                let len = self.paragraph(i).map_or(0, |p| p.text.len());
                Some((if i == para_index(&a) { a.off } else { 0 })..(if i == para_index(&b) { b.off } else { len }))
            } else {
                None
            };
            if let Some(jcj_doc::Block::Para(p)) = self.session.doc.body.get_mut(i).map(std::sync::Arc::make_mut) {
                n += replace_in_paragraph(p, &re, q, with, within);
            }
        }
        self.session.sel = jcj_engine::Selection::caret(pos(para_index(&a), a.off.min(self.paragraph(para_index(&a)).map_or(0, |p| p.text.len()))));
        self.changed();
        Ok(n)
    }

    /// Replace the selected match and find the next.
    pub fn replace_one(&mut self, q: &Query, with: &str) -> Result<bool, String> {
        let re = q.matcher()?;
        let sel = self.selected_text();
        let whole = re.find(&sel).is_some_and(|m| m.start() == 0 && m.end() == sel.len());
        if !self.session.sel.is_collapsed() && whole {
            let rep = replacement(&re, q, &sel, with);
            self.guarded(&rep, true, &|sess, t| jcj_engine::cmd::paste_text(sess, t))?;
        }
        self.find_next(q, false)
    }
}
