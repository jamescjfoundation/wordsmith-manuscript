// SPDX-License-Identifier: MIT OR Apache-2.0
//! Words: the one place Manuscript counts and lists them, so every view, target and export agrees.
//!
//! Counting uses the core's word rule ([`jcj_doc::count_words`]): whitespace-separated pieces
//! holding a letter or digit, with each Han ideograph or kana letter a word of its own.

/// How many words `text` has, by the core's rule.
pub fn count(text: &str) -> usize {
    jcj_doc::count_words(text)
}

/// How many characters `text` has, line breaks not counted.
pub fn chars(text: &str) -> usize {
    jcj_doc::count_chars(text)
}

/// How many characters `text` has without any whitespace.
pub fn chars_no_spaces(text: &str) -> usize {
    text.chars().filter(|c| !c.is_whitespace()).count()
}

/// The words of `text` in order, each exactly as written (punctuation attached), by the same
/// rule as [`count`]: `words(t).len() == count(t)` for text without Han ideographs or kana.
pub fn words(text: &str) -> Vec<String> {
    text.split(char::is_whitespace).filter(|w| w.chars().any(char::is_alphanumeric)).map(str::to_owned).collect()
}

/// Every whitespace-separated token of `text`, exactly as written, punctuation-only tokens
/// included: what the word-for-word check compares.
pub fn tokens(text: &str) -> impl Iterator<Item = &str> {
    text.split(char::is_whitespace).filter(|w| !w.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_agree_with_the_word_list() {
        let t = "It was the best of times — it was the worst of times. “Quite,” said he.";
        assert_eq!(count(t), words(t).len());
        assert_eq!(count(t), 15);
        assert_eq!(tokens(t).count(), 16);
    }

    #[test]
    fn characters() {
        assert_eq!(chars("a b\nc"), 4);
        assert_eq!(chars_no_spaces("a b\nc"), 3);
    }
}
