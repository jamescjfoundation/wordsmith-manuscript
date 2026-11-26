// SPDX-License-Identifier: MIT OR Apache-2.0
//! Comparing two versions of a text word by word (INS-5; drafts compare scenes with it too).
//!
//! Paragraphs are matched first (the longest common run of equal paragraphs); between two matched
//! paragraphs, the old and new ones left over are paired in order and compared word by word, and
//! the rest are whole paragraphs added or removed. Words, the spaces between them and each
//! punctuation mark are the units, so a changed comma shows as a changed comma.
//!
//! core-candidate: open-source-core#17. The core's comparison (`jcj-features` compare) marks whole paragraphs; this
//! word-level refinement would serve any foundation app's compare.

use crate::Text;

/// What a stretch of a comparison is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Change {
    /// In both versions.
    Same,
    /// Only in the newer version.
    Added,
    /// Only in the older version.
    Removed,
}

/// A stretch of one paragraph of a comparison.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Segment {
    /// What it is.
    pub change: Change,
    /// Its text.
    pub text: String,
}

/// A comparison: the paragraphs of both versions interleaved, each as segments.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Diff {
    /// The paragraphs in reading order.
    pub paras: Vec<Vec<Segment>>,
    /// Words only in the newer version.
    pub added: usize,
    /// Words only in the older version.
    pub removed: usize,
}

impl Diff {
    /// Whether the versions hold the same words in the same paragraphs.
    pub fn is_same(&self) -> bool {
        self.paras.iter().flatten().all(|s| s.change == Change::Same)
    }
}

/// The units a paragraph is compared in: runs of letters and digits (with inner apostrophes),
/// runs of spaces, and single other characters.
pub fn tokens(s: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0;
    let mut kind = 0u8; // 1 word, 2 space
    for (i, c) in s.char_indices() {
        let k = if c.is_alphanumeric() || ((c == '\'' || c == '’') && kind == 1) {
            1
        } else if c.is_whitespace() {
            2
        } else {
            3
        };
        if i > start && (k != kind || k == 3) {
            out.push(&s[start..i]);
            start = i;
        }
        kind = k;
    }
    if start < s.len() {
        out.push(&s[start..]);
    }
    out
}

/// The longest common subsequence of two lists, as pairs of indexes.
fn lcs<T: PartialEq>(a: &[T], b: &[T]) -> Vec<(usize, usize)> {
    // Equal ends are taken off first: most edits touch a small part of a text.
    let mut pre = 0;
    while pre < a.len() && pre < b.len() && a[pre] == b[pre] {
        pre += 1;
    }
    let mut suf = 0;
    while suf < a.len() - pre && suf < b.len() - pre && a[a.len() - 1 - suf] == b[b.len() - 1 - suf] {
        suf += 1;
    }
    let (ma, mb) = (&a[pre..a.len() - suf], &b[pre..b.len() - suf]);
    let mut out: Vec<(usize, usize)> = (0..pre).map(|i| (i, i)).collect();
    let (n, m) = (ma.len(), mb.len());
    if n > 0 && m > 0 && n.saturating_mul(m) <= 16_000_000 {
        let w = m + 1;
        let mut t = vec![0u32; (n + 1) * w];
        for i in (0..n).rev() {
            for j in (0..m).rev() {
                t[i * w + j] = if ma[i] == mb[j] { t[(i + 1) * w + j + 1] + 1 } else { t[(i + 1) * w + j].max(t[i * w + j + 1]) };
            }
        }
        let (mut i, mut j) = (0, 0);
        while i < n && j < m {
            if ma[i] == mb[j] {
                out.push((pre + i, pre + j));
                i += 1;
                j += 1;
            } else if t[(i + 1) * w + j] >= t[i * w + j + 1] {
                i += 1;
            } else {
                j += 1;
            }
        }
    }
    out.extend((0..suf).map(|k| (a.len() - suf + k, b.len() - suf + k)));
    out
}

fn push(segs: &mut Vec<Segment>, change: Change, text: &str) {
    if text.is_empty() {
        return;
    }
    match segs.last_mut() {
        Some(s) if s.change == change => s.text.push_str(text),
        _ => segs.push(Segment { change, text: text.to_owned() }),
    }
}

fn count(text: &str) -> usize {
    crate::words::words(text).len()
}

/// Compare two paragraphs word by word.
pub fn paragraph(old: &str, new: &str) -> Vec<Segment> {
    let (a, b) = (tokens(old), tokens(new));
    let common = lcs(&a, &b);
    let mut segs = Vec::new();
    let (mut i, mut j) = (0, 0);
    for (ci, cj) in common.into_iter().chain(std::iter::once((a.len(), b.len()))) {
        for t in &a[i..ci] {
            push(&mut segs, Change::Removed, t);
        }
        for t in &b[j..cj] {
            push(&mut segs, Change::Added, t);
        }
        if ci < a.len() {
            push(&mut segs, Change::Same, a[ci]);
        }
        i = ci + 1;
        j = cj + 1;
    }
    segs
}

/// Compare two lists of paragraphs.
pub fn paragraphs(old: &[String], new: &[String]) -> Diff {
    let common = lcs(old, new);
    let mut d = Diff::default();
    let (mut i, mut j) = (0, 0);
    for (ci, cj) in common.into_iter().chain(std::iter::once((old.len(), new.len()))) {
        let (gone, came) = (&old[i..ci], &new[j..cj]);
        for k in 0..gone.len().max(came.len()) {
            let segs = match (gone.get(k), came.get(k)) {
                (Some(o), Some(n)) => paragraph(o, n),
                (Some(o), None) => vec![Segment { change: Change::Removed, text: o.clone() }],
                (None, Some(n)) => vec![Segment { change: Change::Added, text: n.clone() }],
                (None, None) => Vec::new(),
            };
            d.paras.push(segs);
        }
        if ci < old.len() {
            d.paras.push(vec![Segment { change: Change::Same, text: old[ci].clone() }]);
        }
        i = ci + 1;
        j = cj + 1;
    }
    for s in d.paras.iter().flatten() {
        match s.change {
            Change::Added => d.added += count(&s.text),
            Change::Removed => d.removed += count(&s.text),
            Change::Same => {}
        }
    }
    d
}

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
}
