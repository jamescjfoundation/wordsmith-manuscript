// SPDX-License-Identifier: MIT OR Apache-2.0
//! Project search (SRC-1, SPEC 11): every word of every text, synopsis, note and bible entry,
//! answered from an index built in the background.
//!
//! core-candidate: open-source-core issue #8 (C7). The index maps each word (lowercased, letters
//! and digits) to the documents holding it; a query narrows the documents by its words, then the
//! exact search ([`crate::find`]) runs on those alone. Patterns and queries without a whole word
//! scan everything. A plain in-memory index answers a 300,000-word project well within the plan's
//! 200 ms, so no search engine library is needed (plan section 14).

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use serde_json::{Value, json};
use wordsmith_manuscript_model::{ItemId, Project};

use crate::commands::Spec;
use crate::find::{Found, Query, find_in_project, matches};
use crate::session::{Outcome, Session};

/// The words of a text as the index keeps them: lowercased runs of letters and digits.
pub fn index_words(text: &str) -> impl Iterator<Item = String> + '_ {
    text.split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()).map(str::to_lowercase)
}

/// An index of a project's words.
#[derive(Clone, Debug, Default)]
pub struct Index {
    /// The project revision it was built from.
    pub revision: u64,
    words: HashMap<String, HashSet<ItemId>>,
}

impl Index {
    /// Build the index of a project.
    pub fn build(p: &Project, revision: u64) -> Index {
        let mut words: HashMap<String, HashSet<ItemId>> = HashMap::new();
        let mut add = |id: ItemId, text: &str| {
            for w in index_words(text) {
                words.entry(w).or_default().insert(id);
            }
        };
        for (id, t) in &p.texts {
            for para in t.body.plain_paragraphs() {
                add(*id, &para);
            }
            add(*id, &t.meta.synopsis);
            add(*id, &t.meta.notes);
        }
        for (id, e) in &p.entries {
            for para in e.body.plain_paragraphs() {
                add(*id, &para);
            }
            for n in e.names() {
                add(*id, n);
            }
        }
        Index { revision, words }
    }

    /// The documents that may match a query: `None` when the index cannot narrow it (a pattern,
    /// or no whole word in it).
    pub fn candidates(&self, q: &Query) -> Option<HashSet<ItemId>> {
        if q.regex {
            return None;
        }
        let words: Vec<String> = index_words(&q.text).collect();
        // Only the words between other characters are whole; the first and last may be parts.
        let whole: Vec<&String> = if q.word { words.iter().collect() } else { words.iter().skip(1).take(words.len().saturating_sub(2)).collect() };
        if whole.is_empty() {
            // A part of a word: documents with a word containing it.
            let part = words.first()?;
            if words.len() > 1 {
                return None;
            }
            let mut out = HashSet::new();
            for (w, ids) in &self.words {
                if w.contains(part.as_str()) {
                    out.extend(ids.iter().copied());
                }
            }
            return Some(out);
        }
        let mut sets = whole.iter().map(|w| self.words.get(w.as_str()).cloned().unwrap_or_default());
        let first = sets.next()?;
        Some(sets.fold(first, |acc, s| acc.intersection(&s).copied().collect()))
    }

    /// How many distinct words it holds.
    pub fn len(&self) -> usize {
        self.words.len()
    }

    /// Whether it holds no words.
    pub fn is_empty(&self) -> bool {
        self.words.is_empty()
    }
}

/// The index, kept up to date in the background.
#[derive(Clone, Default)]
pub struct Searcher {
    index: Arc<Mutex<Option<Index>>>,
    building: Arc<Mutex<Option<u64>>>,
}

impl Searcher {
    /// Start building the index for this revision in the background, unless it is built or being
    /// built.
    pub fn refresh(&self, p: &Project, revision: u64) {
        let current = self.index.lock().ok().and_then(|g| g.as_ref().map(|i| i.revision));
        let busy = self.building.lock().ok().and_then(|g| *g);
        if current == Some(revision) || busy == Some(revision) {
            return;
        }
        if let Ok(mut b) = self.building.lock() {
            *b = Some(revision);
        }
        let project = p.clone();
        let slot = self.index.clone();
        let building = self.building.clone();
        std::thread::spawn(move || {
            let idx = Index::build(&project, revision);
            if let Ok(mut s) = slot.lock() {
                *s = Some(idx);
            }
            if let Ok(mut b) = building.lock()
                && *b == Some(revision)
            {
                *b = None;
            }
        });
    }

    /// Build the index now (tests, the command line).
    pub fn build_now(&self, p: &Project, revision: u64) {
        if let Ok(mut s) = self.index.lock() {
            *s = Some(Index::build(p, revision));
        }
    }

    /// The index, if one for `revision` is ready.
    pub fn ready(&self, revision: u64) -> Option<Index> {
        self.index.lock().ok().and_then(|g| g.clone()).filter(|i| i.revision == revision)
    }

    /// Search the project: through the index when it is ready for this revision, else by scanning.
    pub fn search(&self, p: &Project, revision: u64, q: &Query) -> Result<Vec<Found>, String> {
        let re = q.matcher()?;
        let candidates = self.index.lock().ok().and_then(|g| g.as_ref().filter(|i| i.revision == revision).and_then(|i| i.candidates(q)));
        let Some(ids) = candidates else { return Ok(find_in_project(p, &re)) };
        // The exact search on the candidates only, in binder order.
        let mut out = Vec::new();
        for row in p.binder.rows() {
            if !ids.contains(&row.item.id) {
                continue;
            }
            let id = row.item.id;
            if let Some(t) = p.texts.get(&id) {
                for (i, para) in t.body.plain_paragraphs().iter().enumerate() {
                    for r in matches(&re, para) {
                        out.push(Found { item: id, field: "text", para: i, context: para.chars().take(80).collect(), range: r });
                    }
                }
                for (field, s) in [("synopsis", &t.meta.synopsis), ("notes", &t.meta.notes)] {
                    for r in matches(&re, s) {
                        out.push(Found { item: id, field, para: 0, context: s.chars().take(80).collect(), range: r });
                    }
                }
            }
            if let Some(e) = p.entries.get(&id) {
                for (i, para) in e.body.plain_paragraphs().iter().enumerate() {
                    for r in matches(&re, para) {
                        out.push(Found { item: id, field: "text", para: i, context: para.chars().take(80).collect(), range: r });
                    }
                }
            }
        }
        Ok(out)
    }
}

/// The search commands.
pub fn specs() -> Vec<Spec> {
    vec![Spec {
        id: "search.project",
        label: "Search the Project",
        key: "Mod+Shift+P",
        params: r#"{"query": string, "case"?: bool, "word"?: bool, "regex"?: bool}"#,
        about: "The items whose text, synopsis, notes or bible entry hold the query, with each match.",
        mutates: false,
    }]
}

/// Run a search command, or `None` if `id` is not one.
pub fn run(s: &mut Session, id: &str, v: &Value) -> Option<Outcome> {
    Some(match id {
        "search.project" => (|| {
            let q = Query::from_params(v)?;
            let searcher = Searcher::default();
            searcher.build_now(&s.project, s.revision());
            let found = searcher.search(&s.project, s.revision(), &q)?;
            let mut items: Vec<ItemId> = found.iter().map(|f| f.item).collect();
            items.dedup();
            Ok(json!({
                "items": items.iter().map(|i| json!({ "id": i.as_text(), "title": s.project.binder.get(*i).map(|x| x.title.clone()).unwrap_or_default(), "matches": found.iter().filter(|f| f.item == *i).count() })).collect::<Vec<_>>(),
                "count": found.len(),
            }))
        })(),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_words_are_lowercase_letters_and_digits() {
        let w: Vec<String> = index_words("The keeper's lamp, No. 7!").collect();
        assert_eq!(w, ["the", "keeper", "s", "lamp", "no", "7"]);
    }
}
