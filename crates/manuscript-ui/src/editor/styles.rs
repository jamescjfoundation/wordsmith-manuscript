// SPDX-License-Identifier: MIT OR Apache-2.0
//! Manuscript's styles in the editor (ED-1): the Write tab's paragraph styles and character
//! styles. Only styles the text dialect can hold exist here.

use serde_json::{Value, json};
use wordsmith_manuscript_model::doc::{self as mdoc, BlockTag};

use super::{Editor, para_index, pos};
use crate::commands::Spec;

const fn spec(id: &'static str, label: &'static str, key: &'static str, params: &'static str, about: &'static str) -> Spec {
    Spec { id, label, key, params, about, mutates: true }
}

/// The editor's style commands.
pub fn specs() -> Vec<Spec> {
    vec![
        spec("write.body", "Body", "Mod+Alt+0", "{}", "Make the selected paragraphs body text."),
        spec("write.quote", "Block Quote", "Mod+Alt+Q", "{}", "Make the selected paragraphs a block quote."),
        spec("write.verse", "Verse", "Mod+Alt+V", "{}", "Make the selected paragraphs verse (line breaks kept)."),
        spec("write.aside", "Aside", "Mod+Alt+A", r#"{"kind"?: string}"#, "Make the selected paragraphs an aside (a letter, a note, a message)."),
        spec("write.heading", "Heading", "", r#"{"level": 1|2|3}"#, "Make the paragraph a heading (for front and back matter pages)."),
        spec("write.sceneBreak", "Scene Break", "Mod+Shift+Enter", "{}", "Insert a scene break at the caret."),
        spec("write.italic", "Italic", "Mod+I", "{}", "Italic on or off."),
        spec("write.bold", "Bold", "Mod+B", "{}", "Bold on or off."),
        spec("write.smallCaps", "Small Caps", "Mod+Alt+K", "{}", "Small capitals on or off."),
        spec("write.strike", "Strikethrough", "", "{}", "Strikethrough on or off."),
        spec("write.language", "Language", "", r#"{"lang": string}"#, "Mark the selection as another language (empty to clear)."),
        spec("write.plain", "Plain Text", "Mod+Space", "{}", "Remove the character styles from the selection."),
        spec("write.lineBreak", "Line Break", "", "{}", "A line break inside the paragraph."),
        spec(
            "write.link",
            "Link",
            "Mod+K",
            r#"{"target": "wsm:<id>" | url}"#,
            "Link the selection to a binder item, a bible entry or a web address.",
        ),
        spec("write.unlink", "Remove Link", "Mod+Shift+K", "{}", "Remove links from the selection."),
    ]
}

/// The group a new block of `kind` starting at paragraph `i` joins: the paragraph before's when
/// it is the same kind, else a new one.
fn group_for(ed: &Editor, i: usize, same: impl Fn(&BlockTag) -> Option<u32>) -> u32 {
    if i > 0
        && let Some(g) = ed.paragraph(i - 1).map(mdoc::tag_of).as_ref().and_then(&same)
    {
        return g;
    }
    let max = ed
        .paragraphs()
        .iter()
        .map(|p| match mdoc::tag_of(p) {
            BlockTag::Quote { group }
            | BlockTag::Verse { group }
            | BlockTag::Aside { group, .. }
            | BlockTag::EmptyFence { group, .. }
            | BlockTag::Note { group, .. } => group,
            _ => 0,
        })
        .max()
        .unwrap_or(0);
    max + 1
}

/// Set paragraph `i`'s block kind (as its own undo step).
pub fn set_block(ed: &mut Editor, i: usize, tag: &BlockTag) {
    ed.session.checkpoint("Paragraph Style");
    if let Some(p) = ed.paragraph_mut(i) {
        mdoc::set_tag(p, tag);
    }
    ed.changed();
}

fn apply_block(ed: &mut Editor, make: &dyn Fn(u32) -> BlockTag, same: &dyn Fn(&BlockTag) -> Option<u32>) -> Result<(), String> {
    if ed.read_only {
        return Err("this text is being edited in a stitched view".into());
    }
    let range = ed.selected_paragraphs();
    let group = group_for(ed, *range.start(), same);
    ed.session.checkpoint("Paragraph Style");
    for i in range {
        let Some(p) = ed.paragraph(i) else { continue };
        if crate::stitch::is_divider(p) || !mdoc::is_text(&mdoc::tag_of(p)) {
            continue;
        }
        let tag = make(group);
        if let Some(p) = ed.paragraph_mut(i) {
            mdoc::set_tag(p, &tag);
        }
    }
    ed.changed();
    Ok(())
}

fn toggle(ed: &mut Editor, engine: &str) -> Result<(), String> {
    ed.run(engine, &json!({})).map(|_| ())
}

/// Insert a scene break at the caret: the paragraph is split there and the break goes between.
pub fn scene_break(ed: &mut Editor) -> Result<(), String> {
    if ed.read_only {
        return Err("this text is being edited in a stitched view".into());
    }
    let i = para_index(&ed.session.sel.focus);
    let empty = ed.paragraph(i).is_some_and(|p| p.text.is_empty());
    if !empty {
        let at_end = ed.paragraph(i).is_some_and(|p| ed.session.sel.focus.off >= p.text.len());
        ed.run("text.newParagraph", &json!({}))?;
        if at_end {
            // The caret is on the new, empty paragraph: it becomes the break.
        } else {
            // Split in the middle: a new paragraph for the break before the second half.
            let j = para_index(&ed.session.sel.focus);
            ed.set_caret(j, 0, false);
            ed.run("text.newParagraph", &json!({}))?;
            let k = para_index(&ed.session.sel.focus);
            ed.set_caret(k - 1, 0, false);
        }
    }
    let i = para_index(&ed.session.sel.focus);
    set_block(ed, i, &BlockTag::SceneBreak);
    // The caret moves to the paragraph after the break, made if needed.
    if ed.paragraph(i + 1).is_none() {
        ed.set_caret(i, 0, false);
        ed.run("text.newParagraph", &json!({}))?;
        set_block(ed, i + 1, &BlockTag::Body);
    }
    ed.set_caret(i + 1, 0, false);
    Ok(())
}

/// Set or clear the language of the selection.
fn language(ed: &mut Editor, lang: &str) -> Result<(), String> {
    if ed.session.sel.is_collapsed() {
        return Err("select the words in another language first".into());
    }
    let lang = lang.trim().to_owned();
    if !lang.is_empty() && !lang.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return Err(format!("`{lang}` is not a language tag (such as fr or de-CH)"));
    }
    let (a, b) = ed.session.sel.ordered();
    ed.session.checkpoint("Language");
    for i in para_index(&a)..=para_index(&b) {
        let Some(len) = ed.paragraph(i).map(|p| p.text.len()) else { continue };
        let from = if i == para_index(&a) { a.off.min(len) } else { 0 };
        let to = if i == para_index(&b) { b.off.min(len) } else { len };
        let value = (!lang.is_empty()).then(|| lang.clone());
        if let Some(p) = ed.paragraph_mut(i) {
            let _ = p.format(from, to, &|c: &mut jcj_doc::CharProps| c.lang.clone_from(&value));
        }
    }
    ed.changed();
    Ok(())
}

/// Link the selection to a target, or remove links (`None`).
fn link(ed: &mut Editor, target: Option<&str>) -> Result<(), String> {
    if ed.session.sel.is_collapsed() {
        return Err("select the words to link first".into());
    }
    if let Some(t) = target
        && (t.trim().is_empty() || t.contains(char::is_whitespace))
    {
        return Err("a link needs a target: an item (wsm:<id>) or a web address".into());
    }
    let (a, b) = ed.session.sel.ordered();
    ed.session.checkpoint(if target.is_some() { "Link" } else { "Remove Link" });
    for i in para_index(&a)..=para_index(&b) {
        let Some(len) = ed.paragraph(i).map(|p| p.text.len()) else { continue };
        let from = if i == para_index(&a) { a.off.min(len) } else { 0 };
        let to = if i == para_index(&b) { b.off.min(len) } else { len };
        let t = target.map(str::to_owned);
        if let Some(p) = ed.paragraph_mut(i) {
            let _ = p.format(from, to, &|c: &mut jcj_doc::CharProps| match &t {
                Some(t) => {
                    let _ = mdoc::LINK.set(c, t);
                }
                None => mdoc::LINK.remove(c),
            });
        }
    }
    ed.changed();
    Ok(())
}

/// Anchor a comment (its id) to the selection: one undo step.
pub fn mark_comment(ed: &mut Editor, key: &str) -> Result<(), String> {
    if ed.session.sel.is_collapsed() {
        return Err("select the words to comment on first".into());
    }
    let (a, b) = ed.session.sel.ordered();
    ed.session.checkpoint("Add Comment");
    for i in para_index(&a)..=para_index(&b) {
        let Some(len) = ed.paragraph(i).map(|p| p.text.len()) else { continue };
        let from = if i == para_index(&a) { a.off.min(len) } else { 0 };
        let to = if i == para_index(&b) { b.off.min(len) } else { len };
        let k = key.to_owned();
        if let Some(p) = ed.paragraph_mut(i) {
            let _ = p.format(from, to, &|c: &mut jcj_doc::CharProps| {
                let _ = mdoc::COMMENT.set(c, &k);
            });
        }
    }
    ed.changed();
    Ok(())
}

/// Remove italic, bold, small caps, strikethrough and language from the selection.
fn plain(ed: &mut Editor) -> Result<(), String> {
    let (a, b) = ed.session.sel.ordered();
    ed.session.checkpoint("Plain Text");
    for i in para_index(&a)..=para_index(&b) {
        let Some(len) = ed.paragraph(i).map(|p| p.text.len()) else { continue };
        let from = if i == para_index(&a) { a.off.min(len) } else { 0 };
        let to = if i == para_index(&b) { b.off.min(len) } else { len };
        if let Some(p) = ed.paragraph_mut(i) {
            let _ = p.format(from, to, &|c: &mut jcj_doc::CharProps| {
                c.italic = None;
                c.bold = None;
                c.small_caps = None;
                c.strike = None;
                c.lang = None;
            });
        }
    }
    ed.changed();
    Ok(())
}

/// Run a style command, or `None` if `id` is not one.
pub fn run(ed: &mut Editor, id: &str, v: &Value) -> Option<Result<(), String>> {
    Some(match id {
        "write.body" => apply_block(ed, &|_| BlockTag::Body, &|_| None),
        "write.quote" => {
            apply_block(ed, &|group| BlockTag::Quote { group }, &|t| if let BlockTag::Quote { group } = t { Some(*group) } else { None })
        }
        "write.verse" => {
            apply_block(ed, &|group| BlockTag::Verse { group }, &|t| if let BlockTag::Verse { group } = t { Some(*group) } else { None })
        }
        "write.aside" => {
            let kind = v.get("kind").and_then(Value::as_str).map(str::trim).filter(|k| !k.is_empty()).map(str::to_owned);
            let k2 = kind.clone();
            apply_block(ed, &move |group| BlockTag::Aside { group, aside: kind.clone() }, &move |t| match t {
                BlockTag::Aside { group, aside } if *aside == k2 => Some(*group),
                _ => None,
            })
        }
        "write.heading" => {
            let level = v.get("level").and_then(Value::as_u64).unwrap_or(1).clamp(1, 3) as u8;
            apply_block(ed, &move |_| BlockTag::Heading { level }, &|_| None)
        }
        "write.sceneBreak" => scene_break(ed),
        "write.italic" => toggle(ed, "format.italic"),
        "write.bold" => toggle(ed, "format.bold"),
        "write.smallCaps" => toggle(ed, "format.smallCaps"),
        "write.strike" => toggle(ed, "format.strikethrough"),
        "write.language" => language(ed, v.get("lang").and_then(Value::as_str).unwrap_or("")),
        "write.plain" => plain(ed),
        "write.lineBreak" => ed.run("text.lineBreak", &json!({})).map(|_| ()),
        "write.link" => link(ed, Some(v.get("target").and_then(Value::as_str).unwrap_or(""))),
        "write.unlink" => link(ed, None),
        _ => return None,
    })
}

/// The block kind at the caret, for the ribbon's state.
pub fn current_block(ed: &Editor) -> BlockTag {
    ed.paragraph(para_index(&ed.session.sel.focus)).map(mdoc::tag_of).unwrap_or(BlockTag::Body)
}

/// Whether a position exists (tests).
pub fn valid(ed: &Editor, i: usize, off: usize) -> bool {
    let _ = pos(i, off);
    ed.paragraph(i).is_some_and(|p| off <= p.text.len())
}
