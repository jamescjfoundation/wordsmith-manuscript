// SPDX-License-Identifier: MIT OR Apache-2.0
//! The stitched view (ED-2, plan section 5.5): several items edited as one text.
//!
//! core-candidate: open-source-core#9 (C8).

use jcj_doc::{ExtKey, HasExt, Paragraph};
use wordsmith_manuscript_model::ItemId;

/// The item a paragraph of a stitched view came from.
pub const SOURCE: ExtKey<ItemId> = ExtKey::new("manuscript.source");
/// A divider paragraph between two items, holding the next item's title.
pub const DIVIDER: ExtKey<String> = ExtKey::new("manuscript.divider");

/// Whether a paragraph is a stitched view's divider.
pub fn is_divider(p: &Paragraph) -> bool {
    p.ext().contains(DIVIDER.key())
}

/// A divider's title.
pub fn divider_title(p: &Paragraph) -> String {
    DIVIDER.get_or_none(p).unwrap_or_default()
}

use std::sync::Arc;

use jcj_doc::Document;
use wordsmith_manuscript_model::doc::{self as mdoc};
use wordsmith_manuscript_model::{Project, Text};

use crate::editor::Editor;

/// On a paragraph: it stands for an item whose text is empty.
pub const BLANK: ExtKey<bool> = ExtKey::new("manuscript.blank");

/// The item a paragraph of an editor belongs to: the paragraph's source in a stitched view, else
/// the editor's item.
pub fn item_at(ed: &Editor, para: usize) -> ItemId {
    if !is_stitched(ed) {
        return ed.item;
    }
    (0..=para).rev().find_map(|i| ed.paragraph(i).and_then(|p| SOURCE.get_or_none(p))).unwrap_or(ed.item)
}

/// Whether an editor holds a stitched view.
pub fn is_stitched(ed: &Editor) -> bool {
    ed.paragraphs().iter().any(|p| is_divider(p))
}

fn divider(item: ItemId, title: &str) -> Paragraph {
    let mut p = Paragraph::new();
    let _ = DIVIDER.set(&mut p, &title.to_owned());
    let _ = SOURCE.set(&mut p, &item);
    p
}

/// One document of the items, in the order given: a divider with each item's title, then its
/// paragraphs, each marked with the item it came from.
pub fn build(project: &Project, items: &[ItemId]) -> Document {
    let mut doc = jcj_doc::new_document();
    let mut body = Vec::new();
    for id in items {
        let title = project.binder.get(*id).map(|i| i.title.clone()).unwrap_or_default();
        body.push(Arc::new(jcj_doc::Block::Para(divider(*id, &title))));
        let text = project.text(*id).map(|t| t.body.clone()).unwrap_or_default();
        let sub = mdoc::to_document(&text);
        let blank = text.is_empty();
        for b in sub.body {
            if let jcj_doc::Block::Para(mut p) = (*b).clone() {
                let _ = SOURCE.set(&mut p, id);
                if blank {
                    let _ = BLANK.set(&mut p, &true);
                }
                body.push(Arc::new(jcj_doc::Block::Para(p)));
            }
        }
    }
    if body.is_empty() {
        body.push(Arc::new(jcj_doc::Block::Para(Paragraph::new())));
    }
    doc.body = body;
    doc
}

/// Split a stitched document back into each item's text, in order. A paragraph without a source
/// (made by an edit) belongs to the item above it.
pub fn split(doc: &Document) -> Vec<(ItemId, Text)> {
    let mut out: Vec<(ItemId, Vec<Paragraph>)> = Vec::new();
    for b in &doc.body {
        let Some(p) = b.as_para() else { continue };
        if is_divider(p) {
            if let Some(id) = SOURCE.get_or_none(p) {
                out.push((id, Vec::new()));
            }
            continue;
        }
        if let Some((_, ps)) = out.last_mut() {
            let mut p = p.clone();
            SOURCE.remove(&mut p);
            ps.push(p);
        }
    }
    out.into_iter()
        .map(|(id, ps)| {
            // One empty body paragraph is an empty text (an item whose words were all deleted).
            let blank = ps.len() == 1 && ps.first().is_some_and(|p| p.text.is_empty() && !mdoc::BLOCK.is_set(p));
            if blank || ps.is_empty() {
                return (id, Text::new());
            }
            let mut sub = jcj_doc::new_document();
            sub.body = ps
                .into_iter()
                .map(|mut p| {
                    BLANK.remove(&mut p);
                    Arc::new(jcj_doc::Block::Para(p))
                })
                .collect();
            (id, mdoc::from_document(&sub))
        })
        .collect()
}

/// Write the editor's text back into the project. Returns whether any item changed.
pub fn write_back(ed: &Editor, project: &mut Project) -> bool {
    let mut changed = false;
    let pieces = if is_stitched(ed) { split(&ed.session.doc) } else { vec![(ed.item, ed.text())] };
    for (id, text) in pieces {
        if let Some(e) = project.entries.get_mut(&id) {
            if e.body != text {
                e.body = text;
                changed = true;
            }
            continue;
        }
        if project.binder.get(id).is_none_or(|i| !i.kind.has_text()) {
            continue;
        }
        let now = project.text(id).map(|t| t.body.clone()).unwrap_or_default();
        if now != text {
            project.text_mut(id).body = text;
            changed = true;
        }
    }
    changed
}

/// The dividers of a document, in order (to check an edit kept them).
pub fn dividers(doc: &Document) -> Vec<ItemId> {
    doc.body.iter().filter_map(|b| b.as_para()).filter(|p| is_divider(p)).filter_map(|p| SOURCE.get_or_none(p)).collect()
}

/// Whether an item belongs in a stitched view: text documents always, folders only when they
/// hold text of their own.
pub fn shows(project: &Project, item: &wordsmith_manuscript_model::Item) -> bool {
    match item.kind {
        wordsmith_manuscript_model::Kind::Text => true,
        wordsmith_manuscript_model::Kind::Folder => project.text(item.id).is_some_and(|t| !t.body.is_empty()),
        _ => false,
    }
}

/// The items a stitched view of `root` shows: the item and everything under it with text, in
/// binder order (an item left out of export included: the writer chose it).
pub fn items_under(project: &Project, root: ItemId) -> Vec<ItemId> {
    project.binder.get(root).map_or_else(Vec::new, |i| i.walk().into_iter().filter(|c| shows(project, c)).map(|c| c.id).collect())
}

/// Open a stitched view of `items` in a new editor.
pub fn open(project: &Project, items: &[ItemId], registry: Arc<jcj_engine::Registry>) -> Option<Editor> {
    let first = *items.first()?;
    let doc = build(project, items);
    let mut ed = Editor::on_document(first, doc, registry);
    // The caret starts in the first item's text, after its divider.
    ed.set_caret(1, 0, false);
    Some(ed)
}
