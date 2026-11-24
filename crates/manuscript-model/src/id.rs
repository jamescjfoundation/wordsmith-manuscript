// SPDX-License-Identifier: MIT OR Apache-2.0
//! Stable identifiers for binder items and bible entries.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// The id of a binder item or bible entry: a UUID version 7 (time-ordered), written in lowercase.
/// Files are named by id, so renaming or moving an item never touches its files.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ItemId(uuid::Uuid);

impl ItemId {
    /// A new id, ordered after every id made before it in this process.
    pub fn new() -> ItemId {
        ItemId(uuid::Uuid::now_v7())
    }

    /// An id from 16 bytes (tests and deterministic templates).
    pub const fn from_bytes(bytes: [u8; 16]) -> ItemId {
        ItemId(uuid::Uuid::from_bytes(bytes))
    }

    /// An id made from a counter: the same `n` always gives the same id. For templates, tests and
    /// fixtures that must produce identical bytes on every run.
    pub fn from_counter(n: u64) -> ItemId {
        let mut b = [0u8; 16];
        b[8..].copy_from_slice(&n.to_be_bytes());
        // Version 7 and the RFC variant, so the id is a valid version 7 UUID.
        b[6] = 0x70;
        b[8] = (b[8] & 0x3f) | 0x80;
        ItemId(uuid::Uuid::from_bytes(b))
    }

    /// The id as lowercase hyphenated text, the form used in file names and links.
    pub fn as_text(&self) -> String {
        self.0.hyphenated().to_string()
    }
}

impl Default for ItemId {
    fn default() -> Self {
        ItemId::new()
    }
}

impl fmt::Display for ItemId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.hyphenated())
    }
}

/// Why text is not an item id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BadId(pub String);

impl fmt::Display for BadId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "`{}` is not an item id (a lowercase UUID)", self.0.chars().take(60).collect::<String>())
    }
}

impl std::error::Error for BadId {}

impl FromStr for ItemId {
    type Err = BadId;

    /// Only the canonical form is accepted: 36 characters, lowercase, hyphenated. A file name
    /// or link that is not in that form is not an id.
    fn from_str(s: &str) -> Result<ItemId, BadId> {
        if s.len() != 36 || s.bytes().any(|b| b.is_ascii_uppercase()) {
            return Err(BadId(s.to_owned()));
        }
        uuid::Uuid::try_parse(s).map(ItemId).map_err(|_| BadId(s.to_owned()))
    }
}

impl Serialize for ItemId {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.as_text())
    }
}

impl<'de> Deserialize<'de> for ItemId {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<ItemId, D::Error> {
        let s = String::deserialize(d)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_lowercase_and_parse_back() {
        let id = ItemId::new();
        let text = id.as_text();
        assert_eq!(text, text.to_lowercase());
        assert_eq!(text.parse::<ItemId>().unwrap(), id);
    }

    #[test]
    fn new_ids_are_ordered() {
        let a = ItemId::new();
        let b = ItemId::new();
        assert!(a < b);
    }

    #[test]
    fn non_canonical_text_is_refused() {
        assert!("not-an-id".parse::<ItemId>().is_err());
        let up = ItemId::new().as_text().to_uppercase();
        assert!(up.parse::<ItemId>().is_err());
        assert!("0190a5c2b7d07c3e8d2f1a2b3c4d5e6f".parse::<ItemId>().is_err());
    }

    #[test]
    fn counter_ids_are_stable_and_valid() {
        assert_eq!(ItemId::from_counter(7), ItemId::from_counter(7));
        let text = ItemId::from_counter(7).as_text();
        assert_eq!(text.parse::<ItemId>().unwrap(), ItemId::from_counter(7));
        assert_eq!(&text[14..15], "7");
    }
}
