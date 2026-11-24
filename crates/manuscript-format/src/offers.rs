// SPDX-License-Identifier: MIT OR Apache-2.0
//! Fixes offered back by Wordsmith Book (plan section 9.2 item 4, `docs/book-handoff.md`).
//!
//! Book never writes the project. A light fix made there (a typo, a missing comma) is written by
//! Book into an offers file **beside** the project folder ([`offers_path`]: `My Novel.wsm` →
//! `My Novel.wsm-offers.json`), as a change against an item's stable id: these exact words, in
//! one paragraph, become those. Manuscript shows the offers in its "Offered fixes" inbox; the
//! writer accepts or rejects each, and the decisions are kept in the project
//! (`[extra] offers`), where Book reads them through [`decisions`].
//!
//! The file is versioned ([`VERSION`]); Manuscript refuses a newer one. Reading is bounded.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use wordsmith_manuscript_model::{ItemId, Project};

/// The offers file's version.
pub const VERSION: u32 = 1;
/// The largest offers file read.
pub const MAX_FILE: u64 = 8 << 20;
/// The most offers read from one file.
pub const MAX_OFFERS: usize = 10_000;
/// The settings key the decisions are kept under.
pub const KEY: &str = "offers";

/// One fix offered back.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Offer {
    /// The offer's id, unique within the file (Book's choice: a UUID is fine).
    pub id: String,
    /// The item whose text it changes.
    pub item: ItemId,
    /// The exact words to change, within one paragraph of the item's text (plain text: what a
    /// reader sees, without marks); they must occur exactly once in the item.
    pub find: String,
    /// What they become (plain text; it takes the formatting of the words it replaces).
    pub replace: String,
    /// Why, for the writer ("typo", "the name is spelt Marah elsewhere").
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub note: String,
    /// When it was made, RFC 3339 or `YYYY-MM-DD`.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub made: String,
}

/// An offers file.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OffersFile {
    /// [`VERSION`].
    pub version: u32,
    /// The offers, oldest first.
    pub offers: Vec<Offer>,
}

/// What the writer decided about an offer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Decision {
    /// Taken into the text.
    Accepted,
    /// Turned down.
    Rejected,
}

/// Where Book writes the offers for the project folder at `root`: a file beside the folder.
pub fn offers_path(root: &Path) -> PathBuf {
    let name = root.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "project".into());
    root.with_file_name(format!("{name}-offers.json"))
}

/// Read an offers file's bytes, checking its version, size and ids.
pub fn read(bytes: &[u8]) -> Result<OffersFile, String> {
    if bytes.len() as u64 > MAX_FILE {
        return Err(format!("the offers file is larger than {} MB", MAX_FILE >> 20));
    }
    let file: OffersFile = serde_json::from_slice(bytes).map_err(|e| format!("the offers file is damaged: {e}"))?;
    if file.version > VERSION {
        return Err(format!("the offers file is version {}, newer than this app reads ({VERSION})", file.version));
    }
    if file.offers.len() > MAX_OFFERS {
        return Err(format!("the offers file holds more than {MAX_OFFERS} offers"));
    }
    let mut seen = std::collections::BTreeSet::new();
    for o in &file.offers {
        if o.id.trim().is_empty() || !seen.insert(o.id.as_str()) {
            return Err(format!("offer ids must be present and unique (`{}`)", o.id));
        }
        if o.find.is_empty() || o.find.contains('\n') || o.replace.contains('\n') {
            return Err(format!("offer `{}` must change words within one paragraph", o.id));
        }
    }
    Ok(file)
}

/// Write an offers file (for Book).
pub fn write(file: &OffersFile) -> Result<Vec<u8>, String> {
    serde_json::to_vec_pretty(file).map_err(|e| e.to_string())
}

/// The writer's decisions so far, by offer id.
pub fn decisions(p: &Project) -> BTreeMap<String, Decision> {
    p.settings.extra.get(KEY).and_then(|v| serde_json::from_value(v.clone()).ok()).unwrap_or_default()
}

/// Record a decision in the project's settings.
pub fn decide(p: &mut Project, id: &str, d: Decision) -> Result<(), String> {
    let mut all = decisions(p);
    all.insert(id.to_owned(), d);
    p.settings.extra.insert(KEY.into(), serde_json::to_value(all).map_err(|e| e.to_string())?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_file_sits_beside_the_project_and_reads_back() {
        assert_eq!(offers_path(Path::new("/w/My Novel.wsm")), Path::new("/w/My Novel.wsm-offers.json"));
        let f = OffersFile {
            version: 1,
            offers: vec![Offer {
                id: "a".into(),
                item: ItemId::from_counter(1),
                find: "teh".into(),
                replace: "the".into(),
                note: "typo".into(),
                made: String::new(),
            }],
        };
        assert_eq!(read(&write(&f).unwrap()).unwrap(), f);
        let newer = String::from_utf8(write(&f).unwrap()).unwrap().replace("\"version\": 1", "\"version\": 2");
        assert!(read(newer.as_bytes()).is_err());
        let twice = OffersFile { version: 1, offers: vec![f.offers[0].clone(), f.offers[0].clone()] };
        assert!(read(&write(&twice).unwrap()).is_err());
        assert!(read(b"{ not json").is_err());
    }
}
