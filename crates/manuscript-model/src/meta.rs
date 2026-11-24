// SPDX-License-Identifier: MIT OR Apache-2.0
//! Per-item metadata kept with the item's text (the front matter of `text/<id>.md`).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::ItemId;

/// A comment anchored to a stretch of text (the text carries the comment's id as a mark).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Comment {
    /// What the comment says (dialect text).
    pub text: String,
    /// Who wrote it, as the writer chose to sign it (may be empty).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub by: String,
    /// When, as `YYYY-MM-DD` or an RFC 3339 time (may be empty).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub date: String,
}

/// Metadata of one text item. Label, status, role and include-in-export live on the binder item
/// (`binder.toml`), so the tree can be shown without reading every text file.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemMeta {
    /// A short summary: the card's text and the outline's second line.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub synopsis: String,
    /// The writer's notes on the item (dialect text).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub notes: String,
    /// Keywords, from the project's keyword list.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub keywords: Vec<String>,
    /// Bible entries linked to the item by hand.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub entries: Vec<ItemId>,
    /// The item's word target.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<u64>,
    /// Custom field values by field name (point of view, scene date, location).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub fields: BTreeMap<String, String>,
    /// Comments by id.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub comments: BTreeMap<String, Comment>,
}

impl ItemMeta {
    /// Whether nothing is set (the text file then has no front matter).
    pub fn is_empty(&self) -> bool {
        *self == ItemMeta::default()
    }
}

/// The text of one item with its metadata.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ItemText {
    /// The metadata.
    pub meta: ItemMeta,
    /// The text.
    pub body: crate::Text,
}

impl ItemText {
    /// An item text with only a body.
    pub fn with_body(body: crate::Text) -> ItemText {
        ItemText { meta: ItemMeta::default(), body }
    }

    /// Whether there is neither metadata nor text.
    pub fn is_empty(&self) -> bool {
        self.meta.is_empty() && self.body.is_empty()
    }
}
