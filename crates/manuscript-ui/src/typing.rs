// SPDX-License-Identifier: MIT OR Apache-2.0
//! Smart typing (ED-5, SPEC 3.6): curly quotes, dashes and the ellipsis as the writer types, per
//! the project's language. Backspace right after a replacement puts back what was typed.

use crate::editor::{Editor, para_index, pos};

/// Which rules are on, and the language whose quotes are used.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Smart {
    /// Curly quotes and apostrophes.
    pub quotes: bool,
    /// `--` to an em dash, a spaced hyphen to an en dash.
    pub dashes: bool,
    /// `...` to an ellipsis.
    pub ellipsis: bool,
    /// The language (BCP 47) whose quotation marks are used.
    pub lang: String,
}

impl Default for Smart {
    fn default() -> Self {
        Smart { quotes: true, dashes: true, ellipsis: true, lang: "en".into() }
    }
}

/// A replacement made, so Backspace can undo it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Replaced {
    /// The paragraph.
    pub para: usize,
    /// Where the replacement starts (bytes).
    pub start: usize,
    /// What it put there.
    pub inserted: String,
    /// What the writer typed (and the characters it replaced).
    pub typed: String,
}

/// The quotation marks of a language: (open double, close double, open single, close single,
/// space inside guillemets).
pub fn marks(lang: &str) -> (&'static str, &'static str, &'static str, &'static str) {
    let base = lang.split(['-', '_']).next().unwrap_or("").to_ascii_lowercase();
    match base.as_str() {
        "fr" => ("«\u{a0}", "\u{a0}»", "‹\u{a0}", "\u{a0}›"),
        "de" | "cs" | "sk" | "lt" => ("„", "“", "‚", "‘"),
        "pl" | "hu" | "ro" | "nl" => ("„", "”", "‚", "’"),
        "sv" | "fi" => ("”", "”", "’", "’"),
        "es" | "it" | "pt" | "ru" | "uk" => ("«", "»", "“", "”"),
        _ => ("“", "”", "‘", "’"),
    }
}

/// Whether a quote typed after `prev` opens (start of text, after a space or an opening mark).
fn opens(prev: Option<char>) -> bool {
    match prev {
        None => true,
        Some(c) => c.is_whitespace() || "([{\u{2014}\u{2013}\"'“‘«‹„‚/".contains(c),
    }
}

/// The replacement for typing `c` after `before` (the paragraph's text up to the caret): how many
/// characters before the caret it replaces and what it inserts. `None`: type `c` as it is.
pub fn rule(smart: &Smart, before: &str, c: char) -> Option<(usize, String)> {
    let prev = before.chars().last();
    let (od, cd, os, cs) = marks(&smart.lang);
    match c {
        '"' if smart.quotes => Some((0, if opens(prev) { od.to_owned() } else { cd.to_owned() })),
        '\'' if smart.quotes => {
            let in_word = prev.is_some_and(|p| p.is_alphanumeric());
            Some((
                0,
                if in_word {
                    "’".to_owned()
                } else if opens(prev) {
                    os.to_owned()
                } else {
                    cs.to_owned()
                },
            ))
        }
        '-' if smart.dashes && prev == Some('-') => Some((1, "—".to_owned())),
        ' ' if smart.dashes && before.ends_with(" -") && before.chars().rev().nth(2).is_some_and(|w| !w.is_whitespace()) => {
            Some((2, " – ".to_owned()))
        }
        '.' if smart.ellipsis && before.ends_with("..") => Some((2, "…".to_owned())),
        _ => None,
    }
}

/// Type `s` with the smart rules; returns the replacement made, if any.
pub fn type_smart(ed: &mut Editor, s: &str) -> Result<Option<Replaced>, String> {
    let mut chars = s.chars();
    let single = match (chars.next(), chars.next()) {
        (Some(c), None) => Some(c),
        _ => None,
    };
    let caret = ed.session.sel.focus.clone();
    let i = para_index(&caret);
    let replacement = match single {
        Some(c) if ed.session.sel.is_collapsed() => {
            let before = ed.paragraph(i).map(|p| p.text.get(..caret.off.min(p.text.len())).unwrap_or("").to_owned()).unwrap_or_default();
            rule(&ed.smart, &before, c).map(|(n, ins)| {
                let start = before.char_indices().rev().nth(n.saturating_sub(1)).map_or(before.len(), |(b, _)| if n == 0 { before.len() } else { b });
                let replaced = before.get(start..).unwrap_or("").to_owned();
                (start, ins, format!("{replaced}{c}"))
            })
        }
        _ => None,
    };
    match replacement {
        None => {
            ed.guarded(s, false, &|sess, t| jcj_engine::cmd::type_text(sess, t))?;
            Ok(None)
        }
        Some((start, inserted, typed)) => {
            if start < caret.off {
                ed.session.sel.anchor = pos(i, start);
            }
            ed.guarded(&inserted, false, &|sess, t| jcj_engine::cmd::type_text(sess, t))?;
            Ok(Some(Replaced { para: i, start, inserted, typed }))
        }
    }
}

/// Put back what was typed in place of a replacement (Backspace right after it).
pub fn undo_smart(ed: &mut Editor, r: &Replaced) -> Result<(), String> {
    let end = r.start + r.inserted.len();
    let ok = ed.paragraph(r.para).is_some_and(|p| p.text.get(r.start..end) == Some(r.inserted.as_str()));
    if !ok {
        return ed.run("text.backspace", &serde_json::json!({})).map(|_| ());
    }
    ed.session.sel = jcj_engine::Selection { anchor: pos(r.para, r.start), focus: pos(r.para, end) };
    jcj_engine::cmd::paste_text(&mut ed.session, &r.typed).map_err(|e| format!("{e:?}"))?;
    ed.changed();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(lang: &str) -> Smart {
        Smart { lang: lang.into(), ..Smart::default() }
    }

    #[test]
    fn quotes_follow_the_language_and_position() {
        assert_eq!(rule(&s("en"), "", '"'), Some((0, "“".into())));
        assert_eq!(rule(&s("en"), "said", '"'), Some((0, "”".into())));
        assert_eq!(rule(&s("en"), "don", '\''), Some((0, "’".into())));
        assert_eq!(rule(&s("en"), "a ", '\''), Some((0, "‘".into())));
        assert_eq!(rule(&s("de"), "", '"'), Some((0, "„".into())));
        assert_eq!(rule(&s("fr-CA"), "", '"'), Some((0, "«\u{a0}".into())));
    }

    #[test]
    fn dashes_and_ellipsis() {
        assert_eq!(rule(&s("en"), "word-", '-'), Some((1, "—".into())));
        assert_eq!(rule(&s("en"), "pages 3 -", ' '), Some((2, " – ".into())));
        assert_eq!(rule(&s("en"), " -", ' '), None);
        assert_eq!(rule(&s("en"), "wait..", '.'), Some((2, "…".into())));
        let off = Smart { quotes: false, dashes: false, ellipsis: false, lang: "en".into() };
        assert_eq!(rule(&off, "a-", '-'), None);
        assert_eq!(rule(&off, "", '"'), None);
    }
}
