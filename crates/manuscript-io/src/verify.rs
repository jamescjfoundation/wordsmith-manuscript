// SPDX-License-Identifier: MIT OR Apache-2.0
//! The word-for-word check (plan section 8, rule 3 of section 4): the words of a source and of
//! what was made from it, in reading order, compared exactly. The first difference is reported
//! with the words around it.
//!
//! core-candidate: open-source-core issue #10 (C9). Nothing here is specific to manuscripts: a
//! "word" is a whitespace-separated piece holding a letter or digit, exactly as written
//! (punctuation attached), so a changed quote mark, a dropped comma or a merged word all show.

use jcj_doc::{Document, Paragraph};

/// The words of a piece of text, exactly as written.
pub fn words(text: &str) -> Vec<String> {
    text.split(char::is_whitespace).filter(|w| w.chars().any(char::is_alphanumeric)).map(str::to_owned).collect()
}

/// A paragraph's words (inline objects left out).
pub fn paragraph_words(p: &Paragraph) -> Vec<String> {
    words(&p.text.replace(jcj_doc::para::OBJ, ""))
}

/// The words of a document's main text in reading order (table cells row by row).
pub fn document_words(doc: &Document) -> Vec<String> {
    let mut out = Vec::new();
    for b in &doc.body {
        collect(b, &mut out);
    }
    out
}

fn collect(b: &jcj_doc::Block, out: &mut Vec<String>) {
    match b {
        jcj_doc::Block::Para(p) => out.extend(paragraph_words(p)),
        jcj_doc::Block::Table(t) => {
            for row in &t.rows {
                for cell in &row.cells {
                    for cb in &cell.blocks {
                        collect(cb, out);
                    }
                }
            }
        }
    }
}

/// Where two word lists first differ.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Difference {
    /// The index of the first word that differs.
    pub at: usize,
    /// The source's word there (none: the source ended).
    pub expected: Option<String>,
    /// The result's word there (none: the result ended).
    pub found: Option<String>,
    /// The words just before, which both share.
    pub before: Vec<String>,
    /// The source's next few words.
    pub expected_after: Vec<String>,
    /// The result's next few words.
    pub found_after: Vec<String>,
}

impl std::fmt::Display for Difference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let show = |w: &Option<String>| w.clone().unwrap_or_else(|| "(end)".into());
        write!(
            f,
            "word {} differs: expected “{}”, found “{}” after “…{}”. Source goes on “{}”; result goes on “{}”.",
            self.at + 1,
            show(&self.expected),
            show(&self.found),
            self.before.join(" "),
            self.expected_after.join(" "),
            self.found_after.join(" ")
        )
    }
}

/// How many words of context a difference shows on each side.
pub const CONTEXT: usize = 6;

/// Compare two word lists. `Ok` holds the number of words; `Err` the first difference.
#[allow(clippy::result_large_err, reason = "a difference is read once, when the check fails; boxing it would change a public signature for no gain")]
pub fn compare(expected: &[String], found: &[String]) -> Result<usize, Difference> {
    let at = expected.iter().zip(found).position(|(a, b)| a != b).unwrap_or(expected.len().min(found.len()));
    if at == expected.len() && at == found.len() {
        return Ok(at);
    }
    let slice = |v: &[String], a: usize, b: usize| v.get(a.min(v.len())..b.min(v.len())).unwrap_or_default().to_vec();
    Err(Difference {
        at,
        expected: expected.get(at).cloned(),
        found: found.get(at).cloned(),
        before: slice(expected, at.saturating_sub(CONTEXT), at),
        expected_after: slice(expected, at + 1, at + 1 + CONTEXT),
        found_after: slice(found, at + 1, at + 1 + CONTEXT),
    })
}

/// The outcome of a check, for reports.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Check {
    /// Every word matched: how many.
    Same(usize),
    /// The first difference.
    Differs(Difference),
}

impl Check {
    /// Check two word lists.
    pub fn of(expected: &[String], found: &[String]) -> Check {
        match compare(expected, found) {
            Ok(n) => Check::Same(n),
            Err(d) => Check::Differs(d),
        }
    }

    /// Whether every word matched.
    pub fn passed(&self) -> bool {
        matches!(self, Check::Same(_))
    }
}

impl std::fmt::Display for Check {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Check::Same(n) => write!(f, "every word matches ({n} words)"),
            Check::Differs(d) => write!(f, "{d}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn w(s: &str) -> Vec<String> {
        words(s)
    }

    #[test]
    fn words_keep_punctuation_and_skip_symbols() {
        assert_eq!(w("“Hello,” she said — * * * again."), ["“Hello,”", "she", "said", "again."]);
    }

    #[test]
    fn the_first_difference_comes_with_context() {
        assert_eq!(compare(&w("a b c"), &w("a b c")), Ok(3));
        let d = compare(&w("one two three four"), &w("one two tree four")).unwrap_err();
        assert_eq!(d.at, 2);
        assert_eq!(d.expected.as_deref(), Some("three"));
        assert_eq!(d.found.as_deref(), Some("tree"));
        assert_eq!(d.before, ["one", "two"]);
        assert!(d.to_string().contains("word 3"));
        let d = compare(&w("a b"), &w("a b c")).unwrap_err();
        assert_eq!((d.expected, d.found.as_deref()), (None, Some("c")));
        let d = compare(&w("a b c"), &w("a b")).unwrap_err();
        assert_eq!(d.found, None);
        // A changed quote mark is a difference.
        assert!(compare(&w("\"Hi\""), &w("“Hi”")).is_err());
    }
}
