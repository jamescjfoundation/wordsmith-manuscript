// SPDX-License-Identifier: MIT OR Apache-2.0
//! Spelling (ED-11, SPEC 6): the core's spelling check, with the project word list (the
//! writer's own words and every bible entry's name and aliases) accepted.

use std::collections::{HashMap, HashSet};
use std::ops::Range;

use serde_json::{Value, json};
use wordsmith_manuscript_model::Project;

use crate::commands::Spec;
use crate::session::{Outcome, Session};

/// The words a project accepts besides the dictionary.
pub fn project_words(p: &Project) -> HashSet<String> {
    let mut out: HashSet<String> = p.settings.words.iter().cloned().collect();
    for e in p.entries.values() {
        for name in e.names() {
            for w in name.split(|c: char| c.is_whitespace() || c == '-') {
                let w = w.trim_matches(|c: char| !c.is_alphanumeric() && c != '\'' && c != '’');
                if !w.is_empty() {
                    out.insert(w.to_owned());
                    out.insert(format!("{w}'s"));
                    out.insert(format!("{w}’s"));
                }
            }
        }
    }
    out
}

/// The misspelled words in a paragraph of language `lang`, not counting the project's words.
pub fn marks(text: &str, lang: &str, words: &HashSet<String>) -> Vec<Range<usize>> {
    let opts = jcj_proof::SpellingOptions::default();
    jcj_proof::check(text, Some(lang), &opts, false)
        .into_iter()
        .filter(|i| i.kind == jcj_proof::IssueKind::Spelling)
        .map(|i| i.range)
        .filter(|r| text.get(r.clone()).is_some_and(|w| !words.contains(w)))
        .collect()
}

/// A grammar or style hint (ED-9): what it says, where, and the replacements offered.
#[derive(Clone, Debug, PartialEq)]
pub struct Hint {
    /// The bytes of the paragraph it is about.
    pub range: Range<usize>,
    /// What it says.
    pub message: String,
    /// Replacements, best first.
    pub suggestions: Vec<String>,
}

/// The grammar hints of a paragraph of language `lang` (the core's offline grammar check).
pub fn hints(text: &str, lang: &str) -> Vec<Hint> {
    let opts = jcj_proof::SpellingOptions::default();
    jcj_proof::check(text, Some(lang), &opts, true)
        .into_iter()
        .filter(|i| i.kind == jcj_proof::IssueKind::Grammar)
        .map(|i| Hint { range: i.range, message: i.message, suggestions: i.suggestions })
        .collect()
}

/// Spelling marks and grammar hints kept per paragraph while its text stays the same.
#[derive(Default)]
pub struct SpellCache {
    marks: HashMap<usize, (u64, u64, Vec<Range<usize>>)>,
    hinted: HashMap<usize, (u64, Vec<Hint>)>,
    /// Whether grammar hints are shown (off by default).
    pub grammar: bool,
    /// Hints the writer chose to ignore this session (message and text).
    pub ignored: HashSet<(String, String)>,
    /// The project's words, and a number that changes with them.
    pub words: HashSet<String>,
    words_gen: u64,
    /// The language checked.
    pub lang: String,
    /// Whether spelling is shown.
    pub on: bool,
}

impl SpellCache {
    /// A cache for a project.
    pub fn new(p: &Project) -> SpellCache {
        SpellCache {
            marks: HashMap::new(),
            hinted: HashMap::new(),
            grammar: false,
            ignored: HashSet::new(),
            words: project_words(p),
            words_gen: 1,
            lang: p.settings.book.language.clone(),
            on: true,
        }
    }

    /// Take a new word list (the project's changed).
    pub fn set_words(&mut self, words: HashSet<String>) {
        if words != self.words {
            self.words = words;
            self.words_gen += 1;
        }
    }

    /// The marks of paragraph `i` at revision `rev`.
    pub fn of(&mut self, i: usize, rev: u64, text: &str) -> Vec<Range<usize>> {
        if !self.on {
            return Vec::new();
        }
        let generation = self.words_gen.wrapping_add(jcj_proof::generation());
        if let Some((r, g, m)) = self.marks.get(&i)
            && *r == rev
            && *g == generation
        {
            return m.clone();
        }
        let m = marks(text, &self.lang, &self.words);
        self.marks.insert(i, (rev, generation, m.clone()));
        m
    }
}

impl SpellCache {
    /// The grammar hints of paragraph `i` at revision `rev` (none while hints are off).
    pub fn hints(&mut self, i: usize, rev: u64, text: &str) -> Vec<Hint> {
        if !self.grammar {
            return Vec::new();
        }
        let found = match self.hinted.get(&i) {
            Some((r, h)) if *r == rev => h.clone(),
            _ => {
                let h = hints(text, &self.lang);
                self.hinted.insert(i, (rev, h.clone()));
                h
            }
        };
        found.into_iter().filter(|h| !self.ignored.contains(&(h.message.clone(), text.get(h.range.clone()).unwrap_or("").to_owned()))).collect()
    }
}

/// The spelling commands.
pub fn specs() -> Vec<Spec> {
    vec![
        Spec {
            id: "spelling.addWord",
            label: "Add to Project Words",
            key: "",
            params: r#"{"word": string}"#,
            about: "Accept a word in this project's spelling check.",
            mutates: true,
        },
        Spec {
            id: "grammar.check",
            label: "Check Grammar",
            key: "",
            params: r#"{"id": id}"#,
            about: "Grammar and style hints for an item (offline): paragraph, place, the words, what is wrong and replacements.",
            mutates: false,
        },
        Spec {
            id: "spelling.check",
            label: "Check Spelling",
            key: "",
            params: r#"{"id": id}"#,
            about: "The misspelled words of an item, with suggestions.",
            mutates: false,
        },
    ]
}

/// Run a spelling command, or `None` if `id` is not one.
pub fn run(s: &mut Session, id: &str, v: &Value) -> Option<Outcome> {
    Some(match id {
        "spelling.addWord" => (|| {
            let w = v.get("word").and_then(Value::as_str).map(str::trim).filter(|w| !w.is_empty()).ok_or("`word` is required")?.to_owned();
            if s.project.settings.words.contains(&w) {
                return Ok(json!({ "added": false }));
            }
            s.change("Add to Project Words", &[], &[], |p| {
                p.settings.words.push(w);
                p.settings.words.sort();
                Ok(())
            })?;
            Ok(json!({ "added": true }))
        })(),
        "grammar.check" => (|| {
            let item = crate::commands::id_param(v, "id")?;
            let lang = s.project.settings.book.language.clone();
            let text = s.project.text(item).map(|t| t.body.plain_paragraphs()).unwrap_or_default();
            let mut out = Vec::new();
            for (i, para) in text.iter().enumerate() {
                for h in hints(para, &lang) {
                    out.push(json!({
                        "paragraph": i, "start": h.range.start, "end": h.range.end,
                        "words": para.get(h.range.clone()).unwrap_or(""), "message": h.message, "suggestions": h.suggestions,
                    }));
                }
            }
            Ok(json!({ "hints": out }))
        })(),
        "spelling.check" => (|| {
            let item = crate::commands::id_param(v, "id")?;
            let words = project_words(&s.project);
            let lang = s.project.settings.book.language.clone();
            let text = s.project.text(item).map(|t| t.body.plain_paragraphs()).unwrap_or_default();
            let mut out = Vec::new();
            for (i, p) in text.iter().enumerate() {
                for r in marks(p, &lang, &words) {
                    let w = p.get(r.clone()).unwrap_or("").to_owned();
                    out.push(json!({ "paragraph": i, "start": r.start, "end": r.end, "word": w, "suggestions": jcj_proof::suggestions(&w, 5, Some(&lang)) }));
                }
            }
            Ok(Value::Array(out))
        })(),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use wordsmith_manuscript_model::Entry;

    #[test]
    fn bible_names_and_project_words_are_accepted() {
        let mut p = Project::new();
        let mut e = Entry::new("character", "Zarethiel Quonmarsh");
        e.aliases = vec!["Zeth".into()];
        p.entries.insert(wordsmith_manuscript_model::ItemId::from_counter(1), e);
        p.settings.words.push("flimbuster".into());
        let words = project_words(&p);
        let text = "Zarethiel met Zeth and a flimbuster near the qwrtzx.";
        let found: Vec<&str> = marks(text, "en", &words).into_iter().map(|r| &text[r]).collect();
        assert_eq!(found, ["qwrtzx"]);
        let none = HashSet::new();
        assert!(marks(text, "en", &none).len() >= 3);
    }
}
