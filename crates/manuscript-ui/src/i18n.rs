// SPDX-License-Identifier: MIT OR Apache-2.0
//! Interface languages (ACC-4): the interface's words through a message catalog.
//!
//! A catalog maps the English text, exactly as the code writes it, to the text shown instead
//! (`assets/i18n/<language>.toml`, bundled into the app). A text a catalog lacks is shown in
//! English, so a partial translation never hides anything. The language is a preference; by
//! default it follows the computer's.
//!
//! core-candidate: open-source-core#32. The core has catalogs now (`jcj-i18n`, core issue #18,
//! release 0.1.13) and the shell's own texts go through them. This module stays until two
//! things are settled: the lookups reachable through `jcj-shell` (#32), and the choice, which
//! writers will see, that the shell's texts follow the language too. `docs/core-candidates.md`.

use std::collections::HashMap;
use std::sync::{Arc, OnceLock, RwLock};

/// The languages the interface comes in: code, name in that language, catalog.
pub const LANGUAGES: [(&str, &str, &str); 2] = [("en", "English", ""), ("es", "Español", include_str!("../../../assets/i18n/es.toml"))];

type Catalog = Arc<HashMap<String, String>>;

fn current() -> &'static RwLock<(String, Catalog)> {
    static CURRENT: OnceLock<RwLock<(String, Catalog)>> = OnceLock::new();
    CURRENT.get_or_init(|| RwLock::new(("en".into(), Arc::new(HashMap::new()))))
}

/// A catalog's entries (an empty map for English or a damaged catalog).
pub fn catalog(code: &str) -> HashMap<String, String> {
    let Some((_, _, text)) = LANGUAGES.iter().find(|l| l.0 == code) else { return HashMap::new() };
    toml::from_str::<HashMap<String, String>>(text).unwrap_or_default()
}

/// The computer's language, if the interface comes in it (from `LC_ALL`, `LC_MESSAGES` or
/// `LANG`: `es_ES.UTF-8` is `es`); else English.
pub fn system_language() -> String {
    for var in ["LC_ALL", "LC_MESSAGES", "LANG"] {
        if let Ok(v) = std::env::var(var) {
            let code: String = v.chars().take_while(|c| c.is_ascii_alphabetic()).collect::<String>().to_lowercase();
            if !code.is_empty() {
                return if LANGUAGES.iter().any(|l| l.0 == code) { code } else { "en".into() };
            }
        }
    }
    "en".into()
}

/// Show the interface in `code` ("" follows the computer). Returns the language now used.
pub fn set_language(code: &str) -> String {
    let code = if code.is_empty() { system_language() } else { code.to_owned() };
    let code = if LANGUAGES.iter().any(|l| l.0 == code) { code } else { "en".into() };
    let map = Arc::new(catalog(&code));
    if let Ok(mut c) = current().write() {
        *c = (code.clone(), map);
    }
    code
}

/// The language the interface is shown in.
pub fn language() -> String {
    current().read().map(|c| c.0.clone()).unwrap_or_else(|_| "en".into())
}

/// The interface text for `english`, in the current language.
pub fn tr(english: &str) -> String {
    current().read().ok().and_then(|c| c.1.get(english).cloned()).unwrap_or_else(|| english.to_owned())
}
