// SPDX-License-Identifier: MIT OR Apache-2.0
//! Comparing two versions of a text word by word (INS-5; drafts compare scenes with it too).
//!
//! Paragraphs are matched first (the longest common run of equal paragraphs); between two matched
//! paragraphs, the old and new ones left over are paired in order and compared word by word, and
//! the rest are whole paragraphs added or removed. Words, the spaces between them and each
//! punctuation mark are the units, so a changed comma shows as a changed comma.
//!
//! The comparison is the core's (`jcj_doc::wordiff`), under the names it has always had here.
//! [`texts`] is this app's own: the entry point over its text type.

pub use jcj_doc::wordiff::{Change, Diff, Segment, paragraph, paragraphs, tokens};

use crate::Text;

/// Compare two texts (their paragraphs, then their notes).
pub fn texts(old: &Text, new: &Text) -> Diff {
    paragraphs(&old.plain_paragraphs(), &new.plain_paragraphs())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn side(segs: &[Segment], keep: Change) -> String {
        segs.iter().filter(|s| s.change == Change::Same || s.change == keep).map(|s| s.text.as_str()).collect()
    }

    #[test]
    fn tokens_are_words_spaces_and_marks() {
        assert_eq!(tokens("Don't stop,  now!"), ["Don't", " ", "stop", ",", "  ", "now", "!"]);
    }

    #[test]
    fn a_changed_word_shows_alone() {
        let segs = paragraph("The old lamp was lit.", "The brass lamp was lit!");
        assert_eq!(side(&segs, Change::Removed), "The old lamp was lit.");
        assert_eq!(side(&segs, Change::Added), "The brass lamp was lit!");
        assert!(segs.contains(&Segment { change: Change::Removed, text: "old".into() }));
        assert!(segs.contains(&Segment { change: Change::Added, text: "brass".into() }));
    }

    #[test]
    fn paragraphs_added_removed_and_changed() {
        let old: Vec<String> = ["One.", "Two words here.", "Three."].iter().map(|s| (*s).to_owned()).collect();
        let new: Vec<String> = ["Zero.", "One.", "Two new words here.", "Four."].iter().map(|s| (*s).to_owned()).collect();
        let d = paragraphs(&old, &new);
        let olds: Vec<String> = d.paras.iter().map(|p| side(p, Change::Removed)).filter(|s| !s.is_empty()).collect();
        let news: Vec<String> = d.paras.iter().map(|p| side(p, Change::Added)).filter(|s| !s.is_empty()).collect();
        assert_eq!(olds, old);
        assert_eq!(news, new);
        assert_eq!((d.added, d.removed), (3, 1));
        assert!(!d.is_same());
        assert!(paragraphs(&old, &old).is_same());
    }

    #[test]
    fn texts_are_compared_by_their_paragraphs() {
        let old = Text::from_paragraphs(&["The old lamp was lit.", "Nobody came."]);
        let new = Text::from_paragraphs(&["The brass lamp was lit.", "Nobody came."]);
        let d = texts(&old, &new);
        assert_eq!((d.added, d.removed, d.paras.len()), (1, 1, 2));
        assert!(texts(&old, &old).is_same());
    }
}
