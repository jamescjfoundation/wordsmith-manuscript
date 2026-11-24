// SPDX-License-Identifier: MIT OR Apache-2.0
//! A whole project in memory: settings, binder, and the text of every text item.

use std::collections::BTreeMap;

use crate::{Area, Binder, ItemId, ItemText, Kind, Settings, entries::Entry};

/// A project: what `project.toml`, `binder.toml`, `text/` and `bible/` hold.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Project {
    /// Settings and book metadata.
    pub settings: Settings,
    /// The binder.
    pub binder: Binder,
    /// The text of each text item (and folder with text), by id. An item without an entry here
    /// has no text yet.
    pub texts: BTreeMap<ItemId, ItemText>,
    /// Bible entries, by id (the binder's `Entry` items).
    pub entries: BTreeMap<ItemId, Entry>,
    /// Snapshots of items' texts, oldest first, by item.
    pub snapshots: BTreeMap<ItemId, Vec<crate::Snapshot>>,
    /// Stored drafts' manuscripts, by draft id (DRF-1; the list and names are in the settings).
    pub drafts: BTreeMap<String, crate::drafts::DraftBody>,
}

impl Project {
    /// An empty project.
    pub fn new() -> Project {
        Project::default()
    }

    /// The text of an item, if it has one.
    pub fn text(&self, id: ItemId) -> Option<&ItemText> {
        self.texts.get(&id)
    }

    /// The text of an item, created empty if missing.
    pub fn text_mut(&mut self, id: ItemId) -> &mut ItemText {
        self.texts.entry(id).or_default()
    }

    /// Take a snapshot of an item's text now. Two in the same second get `-2`, `-3`.
    pub fn take_snapshot(&mut self, id: ItemId, name: &str, at: std::time::SystemTime) -> Option<String> {
        let text = self.texts.get(&id).map(|t| t.body.clone()).unwrap_or_default();
        let list = self.snapshots.entry(id).or_default();
        let base = crate::snapshot::stamp(at);
        let mut taken = base.clone();
        let mut n = 1;
        while list.iter().any(|s| s.taken == taken) && n < 9999 {
            n += 1;
            taken = format!("{base}-{n}");
        }
        list.push(crate::Snapshot { taken: taken.clone(), name: name.to_owned(), text });
        if list.len() > crate::snapshot::MAX_PER_ITEM {
            list.remove(0);
        }
        Some(taken)
    }

    /// The words of one item.
    pub fn item_words(&self, id: ItemId) -> usize {
        self.texts.get(&id).map_or(0, |t| t.body.word_count())
    }

    /// The included items of the Manuscript, in reading order: an item left out of export leaves
    /// its children out too.
    pub fn included_manuscript(&self) -> Vec<ItemId> {
        let mut out = Vec::new();
        let mut skip_below: Option<usize> = None;
        for row in self.binder.rows_of(Area::Manuscript) {
            if let Some(d) = skip_below {
                if row.depth > d {
                    continue;
                }
                skip_below = None;
            }
            if !row.item.include {
                skip_below = Some(row.depth);
                continue;
            }
            out.push(row.item.id);
        }
        out
    }

    /// The words of the included Manuscript.
    pub fn manuscript_words(&self) -> usize {
        self.included_manuscript().into_iter().map(|id| self.item_words(id)).sum()
    }

    /// The words of an item and everything under it.
    pub fn words_under(&self, id: ItemId) -> usize {
        self.binder.get(id).map_or(0, |i| i.walk().iter().map(|c| self.item_words(c.id)).sum())
    }

    /// Drop texts and entries whose items are no longer in the binder (after emptying the Trash).
    pub fn prune(&mut self) {
        let binder = &self.binder;
        self.texts.retain(|id, _| binder.get(*id).is_some_and(|i| i.kind.has_text()));
        self.entries.retain(|id, _| binder.get(*id).is_some_and(|i| i.kind == Kind::Entry));
        self.snapshots.retain(|id, _| binder.contains(*id));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Item, Place, Text};

    #[test]
    fn excluded_items_leave_their_children_out() {
        let mut p = Project::new();
        let a = Item::with_id(ItemId::from_counter(1), Kind::Folder, "A");
        let mut b = Item::with_id(ItemId::from_counter(2), Kind::Folder, "B");
        b.include = false;
        let b = b.with_children(vec![Item::with_id(ItemId::from_counter(3), Kind::Text, "B1")]);
        let c = Item::with_id(ItemId::from_counter(4), Kind::Text, "C");
        for i in [a, b, c] {
            p.binder.insert(Place::end_of(Area::Manuscript), i).unwrap();
        }
        assert_eq!(p.included_manuscript(), vec![ItemId::from_counter(1), ItemId::from_counter(4)]);
        p.texts.insert(ItemId::from_counter(3), ItemText::with_body(Text::from_paragraphs(&["two words"])));
        p.texts.insert(ItemId::from_counter(4), ItemText::with_body(Text::from_paragraphs(&["three more words"])));
        assert_eq!(p.manuscript_words(), 3);
        assert_eq!(p.words_under(ItemId::from_counter(2)), 2);
    }
}
