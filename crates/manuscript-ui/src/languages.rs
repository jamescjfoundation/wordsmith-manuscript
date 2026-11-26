// SPDX-License-Identifier: MIT OR Apache-2.0
//! Interface languages (ACC-4): which languages the interface comes in, and Manuscript's own
//! texts in each.
//!
//! The catalogs and the lookups are the core's (`jcj_shell::i18n`): a text is written in
//! English inside the call that looks it up (`tr("Binder")`), and a catalog
//! (`assets/i18n/<language>.toml`, bundled into the app) maps it to the text shown instead. A
//! text a catalog lacks is shown in English, so a partial translation never hides anything.
//!
//! One language holds for the whole window: the shell's own texts (the File page, Preferences,
//! dialogs, the command palette) come from the shell's catalog in the same language. So the
//! interface comes in a language only when Manuscript and the shell both have it.

/// The languages the interface comes in: code, and name in that language.
pub const LANGUAGES: [(&str, &str); 2] = [("en", "English"), ("es", "Español")];

/// Manuscript's own catalogs: code and catalog text. English needs none.
pub const CATALOGS: [(&str, &str); 1] = [("es", include_str!("../../../assets/i18n/es.toml"))];

/// Show the interface in `code` ("" follows the computer). Gives the language now used: one of
/// [`LANGUAGES`], English when the interface does not come in the language asked for.
pub fn set_language(code: &str) -> String {
    let now = jcj_shell::i18n::set_language(code, &CATALOGS);
    if LANGUAGES.iter().any(|l| l.0 == now) { now } else { jcj_shell::i18n::set_language("en", &CATALOGS) }
}
