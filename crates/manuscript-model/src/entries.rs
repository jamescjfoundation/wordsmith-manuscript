// SPDX-License-Identifier: MIT OR Apache-2.0
//! Entries: typed records with fields from a template, a body, links, backlinks and names.
//!
//! The general layer of the story bible (decision 0004) is the core's `jcj-entries` (promoted
//! from here, core issue #14, C13): entry types and templates, names, lookup by name, whole-word
//! mentions and backlinks. This module gives it Manuscript's ids and text, and adds what reads
//! Manuscript's text: the `wsm:` links in it.

use crate::{Inline, ItemId, Text};

pub use jcj_entries::{FieldType, Template, TemplateField, find_mentions, mentions};

/// One entry: a record of some type with fields, a body (Manuscript's text) and names.
pub type Entry = jcj_entries::Entry<ItemId, Text>;

/// Who refers to an entry.
pub type Backlinks = jcj_entries::Backlinks<ItemId>;

/// Every name and alias with its entry, longest first (so a longer name wins over a shorter one
/// inside it).
pub fn name_list<'a>(entries: impl IntoIterator<Item = (&'a ItemId, &'a Entry)>) -> Vec<(String, ItemId)> {
    jcj_entries::name_list(entries)
}

/// The entry a name refers to: an exact name or alias first, then one that differs only in case.
pub fn find_by_name<'a>(entries: impl IntoIterator<Item = (&'a ItemId, &'a Entry)> + Clone, name: &str) -> Option<ItemId> {
    jcj_entries::find_by_name(entries, name)
}

/// The scheme of a link to an item or entry.
pub const SCHEME: &str = "wsm:";

/// The ids a text links to (`wsm:` links), each once, in order of first appearance.
pub fn links_in(text: &Text) -> Vec<ItemId> {
    let mut out = Vec::new();
    let mut visit = |content: &[Inline]| {
        for i in content {
            if let Some(target) = i.marks().link.as_deref().and_then(|l| l.strip_prefix(SCHEME)).and_then(|l| l.parse::<ItemId>().ok())
                && !out.contains(&target)
            {
                out.push(target);
            }
        }
    };
    for b in &text.blocks {
        match b {
            crate::Block::Para(c) | crate::Block::Heading { content: c, .. } => visit(c),
            crate::Block::Quote(ps) | crate::Block::Fenced { paras: ps, .. } => ps.iter().for_each(|p| visit(p)),
            _ => {}
        }
    }
    for n in &text.notes {
        n.paras.iter().for_each(|p| visit(p));
    }
    out
}

/// The backlinks of `entry` among `documents` (each a document's id and text).
pub fn backlinks<'a>(target: ItemId, entry: &Entry, documents: impl IntoIterator<Item = (ItemId, &'a Text)>) -> Backlinks {
    let docs: Vec<(ItemId, Vec<ItemId>, Vec<String>)> = documents.into_iter().map(|(id, t)| (id, links_in(t), t.plain_paragraphs())).collect();
    jcj_entries::backlinks(target, &entry.names(), docs.iter().map(|(id, links, paragraphs)| jcj_entries::Document { id: *id, links, paragraphs }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Marks;

    fn id(n: u64) -> ItemId {
        ItemId::from_counter(n)
    }

    #[test]
    fn names_and_lookup() {
        let mut a = Entry::new("person", "Anna Field");
        a.aliases = vec!["Annie".into(), " ".into()];
        let b = Entry::new("place", "Field");
        let set = [(id(1), a), (id(2), b)];
        let refs: Vec<(&ItemId, &Entry)> = set.iter().map(|(i, e)| (i, e)).collect();
        let list = name_list(refs.clone());
        assert_eq!(list.first().map(|x| x.0.as_str()), Some("Anna Field"));
        assert_eq!(list.len(), 3);
        assert_eq!(find_by_name(refs.clone(), "annie"), Some(id(1)));
        assert_eq!(find_by_name(refs.clone(), "Field"), Some(id(2)));
        assert_eq!(find_by_name(refs, "Nobody"), None);
    }

    #[test]
    fn whole_word_mentions_respect_capitals() {
        assert!(mentions("Then Anna left.", "Anna"));
        assert!(!mentions("Then Annabel left.", "Anna"));
        assert!(!mentions("then anna left", "Anna"));
        assert!(mentions("The Old Mill stood.", "old mill"), "a name without a capital matches any case");
        assert!(mentions("By the old mill.", "old mill"));
    }

    #[test]
    fn backlinks_split_links_from_mentions() {
        let e = Entry::new("person", "Mara");
        let linked = Text {
            blocks: vec![crate::Block::Para(vec![Inline::marked("her", Marks { link: Some(format!("wsm:{}", id(9))), ..Marks::default() })])],
            notes: vec![],
        };
        let mentioned = Text::from_paragraphs(&["Mara climbed."]);
        let neither = Text::from_paragraphs(&["Nobody here."]);
        let docs = vec![(id(1), &linked), (id(2), &mentioned), (id(3), &neither)];
        let b = backlinks(id(9), &e, docs);
        assert_eq!(b, Backlinks { linked: vec![id(1)], mentioned: vec![id(2)] });
        assert_eq!(links_in(&linked), vec![id(9)]);
    }

    #[test]
    fn template_order_then_extra_fields() {
        let t = Template::new("person", "Person", "People", &[("Age", FieldType::Text), ("History", FieldType::LongText)]);
        let mut e = Entry::new("person", "X");
        e.fields.insert("History".into(), "Long.".into());
        e.fields.insert("Eyes".into(), "Grey".into());
        let names: Vec<String> = t.ordered(&e).into_iter().map(|(n, _)| n).collect();
        assert_eq!(names, ["Age", "History", "Eyes"]);
    }
}
