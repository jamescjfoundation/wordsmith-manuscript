// SPDX-License-Identifier: MIT OR Apache-2.0
//! Entries: typed records with fields from a template, a body, links and names.
//!
//! core-candidate: open-source-core issue #14 (C13). This module is the general layer of the
//! story bible: it names no manuscript concept. Entry types and their templates are data. It
//! moves to the core when a second app uses it (plan section 5.6).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{ItemId, Text};

/// One entry: a record of some type with fields, a body and names.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Entry {
    /// The type's key (`character`, `place`, ... or any type a template defines).
    pub kind: String,
    /// The entry's name.
    pub name: String,
    /// Other names the entry goes by.
    pub aliases: Vec<String>,
    /// Field values by field name, in the template's order when shown.
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
    /// A list of short values.
    List,
    /// A picture from the project's media.
    Image,
    /// A link to another entry.
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
    /// The fields.
    pub fields: Vec<TemplateField>,
}

impl Template {
    /// A template of one-line text fields.
    pub fn simple(kind: &str, title: &str, fields: &[&str]) -> Template {
        Template {
            kind: kind.into(),
            title: title.into(),
            fields: fields.iter().map(|f| TemplateField { name: (*f).into(), kind: FieldType::Text }).collect(),
        }
    }
}
