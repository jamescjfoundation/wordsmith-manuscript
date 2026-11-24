// SPDX-License-Identifier: MIT OR Apache-2.0
//! Comments (ED-7): notes to oneself anchored to a stretch of text. The text carries the
//! comment's id as a mark; what the comment says, who signed it and when are in the item's
//! metadata. Removing a comment never touches the words it was anchored to.

use serde_json::{Value, json};
use wordsmith_manuscript_model::doc as mdoc;
use wordsmith_manuscript_model::{Comment, Inline, ItemId, Project, Text};

use crate::commands::{Spec, id_param, opt_str, str_param};
use crate::session::{Outcome, Session};

/// The comment commands.
pub fn specs() -> Vec<Spec> {
    vec![
        Spec {
            id: "comment.add",
            label: "Add Comment to Words",
            key: "",
            params: r#"{"id": id, "quote": string, "text": string, "occurrence"?: n, "by"?: string}"#,
            about: "Anchor a comment to words of an item's text (the first occurrence of the quote, or the nth). Returns the comment's id.",
            mutates: true,
        },
        Spec {
            id: "comment.set",
            label: "Edit Comment",
            key: "",
            params: r#"{"id": id, "comment": string, "text": string, "by"?: string}"#,
            about: "What a comment says (created if the text carries its id but the item has no entry for it).",
            mutates: true,
        },
        Spec {
            id: "comment.remove",
            label: "Remove Comment",
            key: "",
            params: r#"{"id": id, "comment": string}"#,
            about: "Remove a comment; the words it was anchored to stay as they are.",
            mutates: true,
        },
        Spec {
            id: "comment.list",
            label: "List Comments",
            key: "",
            params: r#"{"id": id}"#,
            about: "An item's comments in text order: id, text, signature, date, the words anchored to, and whether they are still in the text.",
            mutates: false,
        },
    ]
}

/// The comment ids a text carries, in order of first appearance, with the words each covers.
pub fn anchored(t: &Text) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    for c in t.inline_runs() {
        for i in c {
            if let Inline::Span { text, marks } = i
                && let Some(id) = &marks.comment
            {
                match out.iter_mut().find(|(k, _)| k == id) {
                    Some((_, words)) => words.push_str(text),
                    None => out.push((id.clone(), text.clone())),
                }
            }
        }
    }
    out
}

/// A fresh comment id for an item (`c1`, `c2`, ...): one neither its text nor its metadata uses.
pub fn fresh_id(p: &Project, id: ItemId) -> String {
    let t = p.text(id);
    let used: Vec<String> =
        t.map(|t| anchored(&t.body).into_iter().map(|(k, _)| k).chain(t.meta.comments.keys().cloned()).collect()).unwrap_or_default();
    (1u32..).map(|n| format!("c{n}")).find(|k| !used.contains(k)).unwrap_or_else(|| "c0".into())
}

/// Put the comment mark on the `occurrence`th (from 0) stretch of a text that reads `quote`
/// (within one paragraph). Whether it was found.
pub fn anchor(t: &mut Text, quote: &str, occurrence: usize, comment: &str) -> bool {
    if quote.is_empty() {
        return false;
    }
    let mut doc = mdoc::to_document(t);
    let mut seen = 0usize;
    for b in doc.body.iter_mut() {
        let jcj_doc::Block::Para(p) = std::sync::Arc::make_mut(b) else { continue };
        let mut start = 0;
        while let Some(at) = p.text[start..].find(quote) {
            let from = start + at;
            if seen == occurrence {
                let to = from + quote.len();
                let c = comment.to_owned();
                if p.format(from, to, &|cp: &mut jcj_doc::CharProps| {
                    let _ = mdoc::COMMENT.set(cp, &c);
                })
                .is_err()
                {
                    return false;
                }
                *t = mdoc::from_document(&doc);
                return true;
            }
            seen += 1;
            start = from + quote.len().max(1);
        }
    }
    false
}

/// Take a comment's mark off a text, leaving its words. Whether it was there.
pub fn unanchor(t: &mut Text, comment: &str) -> bool {
    let mut found = false;
    for c in t.inline_runs_mut() {
        for i in c.iter_mut() {
            let marks = match i {
                Inline::Span { marks, .. } | Inline::NoteRef { marks, .. } => marks,
            };
            if marks.comment.as_deref() == Some(comment) {
                marks.comment = None;
                found = true;
            }
        }
    }
    if found {
        t.normalize();
    }
    found
}

fn list(p: &Project, id: ItemId) -> Result<Value, String> {
    let t = p.text(id).ok_or_else(|| format!("item {id} has no text"))?;
    let marks = anchored(&t.body);
    let mut rows = Vec::new();
    for (k, words) in &marks {
        let c = t.meta.comments.get(k).cloned().unwrap_or_default();
        rows.push(json!({ "comment": k, "text": c.text, "by": c.by, "date": c.date, "words": words, "anchored": true }));
    }
    for (k, c) in &t.meta.comments {
        if !marks.iter().any(|(m, _)| m == k) {
            rows.push(json!({ "comment": k, "text": c.text, "by": c.by, "date": c.date, "words": "", "anchored": false }));
        }
    }
    Ok(json!({ "comments": rows }))
}

fn today() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}

/// Run a comment command, or `None` if `id` is not one.
pub fn run(s: &mut Session, id: &str, v: &Value) -> Option<Outcome> {
    Some(match id {
        "comment.list" => id_param(v, "id").and_then(|i| list(&s.project, i)),
        "comment.add" => (|| {
            let i = id_param(v, "id")?;
            if !s.project.binder.get(i).is_some_and(|x| x.kind.has_text()) {
                return Err(format!("item {i} has no text to comment on"));
            }
            let quote = str_param(v, "quote")?.to_owned();
            let text = str_param(v, "text")?.to_owned();
            let occurrence = v.get("occurrence").and_then(Value::as_u64).unwrap_or(0) as usize;
            let by = opt_str(v, "by").unwrap_or("").to_owned();
            let key = fresh_id(&s.project, i);
            let k = key.clone();
            s.change("Add Comment", &[i], &[], |p| {
                let t = p.text_mut(i);
                if !anchor(&mut t.body, &quote, occurrence, &k) {
                    return Err(format!("the text does not hold “{quote}” there"));
                }
                t.meta.comments.insert(k, Comment { text, by, date: today() });
                Ok(())
            })?;
            Ok(json!({ "comment": key }))
        })(),
        "comment.set" => (|| {
            let i = id_param(v, "id")?;
            let key = str_param(v, "comment")?.to_owned();
            let text = str_param(v, "text")?.to_owned();
            let by = opt_str(v, "by").map(str::to_owned);
            let t = s.project.text(i).ok_or_else(|| format!("item {i} has no text"))?;
            if !t.meta.comments.contains_key(&key) && !anchored(&t.body).iter().any(|(k, _)| *k == key) {
                return Err(format!("item {i} has no comment {key}"));
            }
            s.change("Edit Comment", &[i], &[], |p| {
                let c = p.text_mut(i).meta.comments.entry(key).or_insert_with(|| Comment { date: today(), ..Comment::default() });
                c.text = text;
                if let Some(b) = by {
                    c.by = b;
                }
                Ok(())
            })?;
            Ok(json!({}))
        })(),
        "comment.remove" => (|| {
            let i = id_param(v, "id")?;
            let key = str_param(v, "comment")?.to_owned();
            let t = s.project.text(i).ok_or_else(|| format!("item {i} has no text"))?;
            if !t.meta.comments.contains_key(&key) && !anchored(&t.body).iter().any(|(k, _)| *k == key) {
                return Err(format!("item {i} has no comment {key}"));
            }
            s.change("Remove Comment", &[i], &[], |p| {
                let t = p.text_mut(i);
                unanchor(&mut t.body, &key);
                t.meta.comments.remove(&key);
                Ok(())
            })?;
            Ok(json!({}))
        })(),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anchoring_and_removing_keeps_every_word() {
        let mut t = Text::from_paragraphs(&["The lamp was lit.", "The lamp went out."]);
        let before = t.plain_paragraphs();
        assert!(anchor(&mut t, "lamp", 1, "c1"));
        assert_eq!(anchored(&t), vec![("c1".to_owned(), "lamp".to_owned())]);
        assert_eq!(t.plain_paragraphs(), before);
        assert!(!anchor(&mut t, "lamp", 2, "c2"));
        assert!(unanchor(&mut t, "c1"));
        assert_eq!(t, Text::from_paragraphs(&["The lamp was lit.", "The lamp went out."]));
    }
}
