// SPDX-License-Identifier: MIT OR Apache-2.0
//! One word rule. "The writer's words are sacred" is proven by listing words, and the rule that
//! lists them is written in two places: `manuscript-model`'s `words::words` (the book view and
//! Wordsmith Book list words with it; the model may not depend on the crate that holds the
//! core's) and the core's `verify::words` (every import and export check here). They must never
//! disagree, so this compares them on a whole novel and on the hard cases.

use wordsmith_manuscript_io::verify;
use wordsmith_manuscript_model::words;

#[test]
fn the_model_and_the_core_list_the_same_words() {
    let novel = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/novels/pride-and-prejudice.txt");
    let novel = std::fs::read_to_string(novel).unwrap();
    let hard = "“Hello,” she said — * * * again.\tTabs\u{a0}and no-break\u{2009}spaces; l’été, 1867; ¿qué? 東京 … _it_ 3.5%\r\nlast";
    for text in [novel.as_str(), hard, "", "   ", "—"] {
        let (model, core) = (words::words(text), verify::words(text));
        assert_eq!(model.len(), core.len());
        assert!(model == core, "the two word rules differ");
    }
    assert!(words::words(&novel).len() > 100_000);
}
