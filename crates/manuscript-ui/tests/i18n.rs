// SPDX-License-Identifier: MIT OR Apache-2.0
//! Interface languages (ACC-4): every command, ribbon label, group and window title has a
//! translation in each catalog, no catalog keeps a text the code no longer uses, and switching
//! language shows the translation (English where none), in Manuscript's texts and the shell's.

use std::collections::BTreeSet;
use std::path::Path;

use jcj_shell::i18n::{self, Catalog};
use jcj_shell::{App, Cmd, Commands, Shell};
use wordsmith_manuscript_ui::{Manuscript, languages};

fn sources() -> String {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut out = String::new();
    let mut stack = vec![dir];
    while let Some(d) = stack.pop() {
        for e in std::fs::read_dir(&d).unwrap().flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|x| x == "rs") {
                out.push_str(&std::fs::read_to_string(&p).unwrap());
            }
        }
    }
    out
}

#[test]
fn every_language_has_a_catalog_that_reads() {
    for (code, _) in languages::LANGUAGES.iter().filter(|l| l.0 != "en") {
        let text = languages::CATALOGS.iter().find(|c| c.0 == *code).unwrap_or_else(|| panic!("{code}: no catalog")).1;
        assert!(!Catalog::parse(text).unwrap_or_else(|e| panic!("{code}: {e}")).is_empty());
        assert!(i18n::LANGUAGES.iter().any(|l| l.0 == *code), "{code}: the shell's own texts come in it too");
    }
    assert_eq!(languages::CATALOGS.len() + 1, languages::LANGUAGES.len(), "a catalog for every language but English, and no other");
}

#[test]
fn every_text_has_a_translation_and_every_translation_is_used() {
    let src = sources();
    let app = Manuscript::new();
    // What the interface shows: the texts written inside a lookup, and the commands' labels,
    // which are looked up when the list of commands is made.
    let mut wanted: BTreeSet<String> = i18n::texts_in(&src);
    assert!(wanted.len() > 100, "the lookups in the code are found: {}", wanted.len());
    wanted.extend(app.all_commands().into_iter().map(|c| c.label).filter(|l| !l.contains(": ")));
    let singulars = i18n::singulars_in(&src);
    for (code, text) in languages::CATALOGS {
        let catalog = Catalog::parse(text).unwrap_or_else(|e| panic!("{code}: {e}"));
        let (missing, _) = i18n::compare(&catalog, &wanted, &singulars);
        assert!(missing.is_empty(), "{code}: no translation for {missing:?}");
        // A label handed to the shell as a value is looked up there, so a text counts as used
        // when the code holds it anywhere.
        let stale: Vec<&str> = catalog.keys().into_iter().filter(|k| !src.contains(&format!("\"{k}\""))).collect();
        assert!(stale.is_empty(), "{code}: no longer in the code: {stale:?}");
    }
}

#[test]
fn switching_language_switches_the_whole_window() {
    let mut app = Manuscript::new();
    let mut sh = Shell::new("Test", "0", wordsmith_manuscript_ui::brand::tokens());
    app.run(&Cmd::new("view.language.es"), &mut sh).unwrap();
    assert_eq!(i18n::language(), "es");
    assert_eq!(app.view.language, "es");
    let save = app.command("file.save").or_else(|| app.all_commands().into_iter().find(|c| c.shortcut == "Mod+S")).unwrap();
    assert_eq!(save.label, "Guardar");
    // Manuscript's own text, from its catalog.
    assert_eq!(i18n::tr("Binder"), "Índice");
    // The shell's own text, from the shell's catalog: one language holds for the whole window.
    assert_eq!(i18n::tr("Check for Updates"), "Buscar actualizaciones");
    assert_eq!(i18n::tr("A text no catalog has"), "A text no catalog has");
    app.run(&Cmd::new("view.language.en"), &mut sh).unwrap();
    assert_eq!(i18n::language(), "en");
    assert_eq!(app.all_commands().into_iter().find(|c| c.shortcut == "Mod+S").unwrap().label, "Save");
    assert_eq!(i18n::tr("Check for Updates"), "Check for Updates");
    // A language the interface does not come in gives English, not half of each.
    assert_eq!(languages::set_language("xx"), "en");
}
