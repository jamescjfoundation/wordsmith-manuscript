// SPDX-License-Identifier: MIT OR Apache-2.0
//! The word-by-word comparison: each side read back from it is exactly that version.

use proptest::prelude::*;
use wordsmith_manuscript_model::diff::{Change, paragraphs};

fn side(d: &wordsmith_manuscript_model::diff::Diff, keep: Change) -> Vec<String> {
    d.paras
        .iter()
        .filter(|p| p.iter().any(|s| s.change == Change::Same || s.change == keep))
        .map(|p| p.iter().filter(|s| s.change == Change::Same || s.change == keep).map(|s| s.text.as_str()).collect())
        .collect()
}

fn para() -> impl Strategy<Value = String> {
    prop::collection::vec(prop::sample::select(vec!["the", "lamp", "Mara", "rain", "fell", ",", ".", " ", "  ", "don't", "é"]), 1..12)
        .prop_map(|v| v.concat())
}

proptest! {
    #[test]
    fn both_versions_come_back_exactly(old in prop::collection::vec(para(), 0..8), new in prop::collection::vec(para(), 0..8)) {
        let d = paragraphs(&old, &new);
        prop_assert_eq!(side(&d, Change::Removed), old.clone());
        prop_assert_eq!(side(&d, Change::Added), new.clone());
        prop_assert_eq!(d.is_same(), old == new);
    }
}
