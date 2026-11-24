// SPDX-License-Identifier: MIT OR Apache-2.0
//! Entries: typed records with fields from a template, a body, links, backlinks and names.
//!
//! core-candidate: open-source-core issue #14 (C13). This module is the general layer of the
//! story bible (decision 0004): it names no manuscript concept. Entry types and their templates
//! are data. Entries are stored as Markdown files with front matter (`manuscript-format`). It
//! moves to the core when a second app uses it (plan section 5.6).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{Inline, ItemId, Text};

/// One entry: a record of some type with fields, a body and names.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Entry {
    /// The type's key (any type a template defines).
    pub kind: String,
    /// The entry's name.
    pub name: String,
    /// Other names the entry goes by.
    pub aliases: Vec<String>,
    /// Field values by field name.
    pub fields: BTreeMap<String, String>,
    /// Typed links to other entries: (relation, entry).
    pub relations: Vec<(String, ItemId)>,
    /// The free body.
    pub body: Text,
}

impl Entry {
    /// A new entry of a type, with a name.
    pub fn new(kind: &str, name: &str) -> Entry {
        Entry { kind: kind.into(), name: name.into(), ..Entry::default() }
    }

    /// The name and the aliases, without empties.
    pub fn names(&self) -> Vec<&str> {
        std::iter::once(self.name.as_str()).chain(self.aliases.iter().map(String::as_str)).filter(|n| !n.trim().is_empty()).collect()
    }
}

/// The kind of a template field.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FieldType {
    /// One line of text.
    #[default]
    Text,
    /// Several lines of text.
    LongText,
    /// A list of short values, one per line.
    List,
    /// A picture from the project's media (its path).
    Image,
    /// A link to another entry (its id).
    Link,
}

/// One field of a template.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TemplateField {
    /// The field's name.
    pub name: String,
    /// Its type.
    #[serde(default)]
    pub kind: FieldType,
}

/// A template: the fields an entry type has, in order.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Template {
    /// The type's key.
    pub kind: String,
    /// The type's name as shown (singular).
    pub title: String,
    /// The type's name for a group of entries (plural), as a folder is named.
    #[serde(default)]
    pub plural: String,
    /// The fields.
    pub fields: Vec<TemplateField>,
}

impl Template {
    /// A template of fields of the given types.
    pub fn new(kind: &str, title: &str, plural: &str, fields: &[(&str, FieldType)]) -> Template {
        Template {
            kind: kind.into(),
            title: title.into(),
            plural: plural.into(),
            fields: fields.iter().map(|(n, k)| TemplateField { name: (*n).into(), kind: *k }).collect(),
        }
    }

    /// The fields of an entry in this template's order, then any others the entry has.
    pub fn ordered<'a>(&self, e: &'a Entry) -> Vec<(String, &'a str)> {
        let mut out: Vec<(String, &str)> = self.fields.iter().map(|f| (f.name.clone(), e.fields.get(&f.name).map_or("", String::as_str))).collect();
        for (k, v) in &e.fields {
            if !self.fields.iter().any(|f| &f.name == k) {
                out.push((k.clone(), v.as_str()));
            }
        }
        out
    }
}

/// Every name and alias with its entry, longest first (so a longer name wins over a shorter one
/// inside it).
pub fn name_list<'a>(entries: impl IntoIterator<Item = (&'a ItemId, &'a Entry)>) -> Vec<(String, ItemId)> {
    let mut out: Vec<(String, ItemId)> = entries.into_iter().flat_map(|(id, e)| e.names().into_iter().map(|n| (n.trim().to_owned(), *id))).collect();
    out.sort_by(|a, b| b.0.chars().count().cmp(&a.0.chars().count()).then_with(|| a.0.cmp(&b.0)));
    out
}

/// The entry a name refers to: an exact name or alias first, then one that differs only in case.
pub fn find_by_name<'a>(entries: impl IntoIterator<Item = (&'a ItemId, &'a Entry)> + Clone, name: &str) -> Option<ItemId> {
    let name = name.trim();
    entries
        .clone()
        .into_iter()
        .find(|(_, e)| e.names().iter().any(|n| n.trim() == name))
        .or_else(|| entries.into_iter().find(|(_, e)| e.names().iter().any(|n| n.trim().to_lowercase() == name.to_lowercase())))
        .map(|(id, _)| *id)
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

/// Whether `name` appears in `text` as a whole word (names starting with a capital letter match
/// only with the same case; others match in any case).
pub fn mentions(text: &str, name: &str) -> bool {
    let name = name.trim();
    if name.is_empty() {
        return false;
    }
    let case = name.chars().next().is_some_and(char::is_uppercase);
    let (hay, needle) = if case { (text.to_owned(), name.to_owned()) } else { (text.to_lowercase(), name.to_lowercase()) };
    let word = |c: Option<char>| c.is_some_and(|c| c.is_alphanumeric() || c == '_');
    let mut from = 0;
    while let Some(at) = hay.get(from..).and_then(|h| h.find(&needle)) {
        let start = from + at;
        let end = start + needle.len();
        if !word(hay[..start].chars().next_back()) && !word(hay[end..].chars().next()) {
            return true;
        }
        from = end;
    }
    false
}

/// Who refers to an entry.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Backlinks {
    /// Documents that link to it, in the order given.
    pub linked: Vec<ItemId>,
    /// Documents that mention a name or alias of it without linking.
    pub mentioned: Vec<ItemId>,
}

/// The backlinks of `entry` among `documents` (each a document's id and text).
pub fn backlinks<'a>(target: ItemId, entry: &Entry, documents: impl IntoIterator<Item = (ItemId, &'a Text)>) -> Backlinks {
    let names = entry.names();
    let mut out = Backlinks::default();
    for (id, text) in documents {
        if id == target {
            continue;
        }
        if links_in(text).contains(&target) {
            out.linked.push(id);
        } else if text.plain_paragraphs().iter().any(|p| names.iter().any(|n| mentions(p, n))) {
            out.mentioned.push(id);
        }
    }
    out
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
