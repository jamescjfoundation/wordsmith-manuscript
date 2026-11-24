// SPDX-License-Identifier: MIT OR Apache-2.0
//! Interface languages (ACC-4): every command, ribbon label, group and window title has a
//! translation in each catalog, no catalog keeps a text the code no longer uses, and switching
//! language shows the translation (English where none).

use std::path::Path;

use jcj_shell::{App, Cmd, Commands, Shell};
use wordsmith_manuscript_ui::{Manuscript, i18n};

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

/// The quoted texts that follow `pattern` in the sources (`tr("…")` calls and the like).
fn quoted_after(src: &str, pattern: &str) -> Vec<String> {
    src.match_indices(pattern).filter_map(|(at, _)| src[at + pattern.len()..].split('"').next().map(str::to_owned)).collect()
}

#[test]
fn every_text_has_a_translation_and_every_translation_is_used() {
    let src = sources();
    let app = Manuscript::new();
    let mut wanted: Vec<String> = app.all_commands().into_iter().map(|c| c.label).filter(|l| !l.contains(": ")).collect();
    wanted.extend(quoted_after(&src, "i18n::tr(\""));
    wanted.push("Interface Language".into());
    wanted.sort();
    wanted.dedup();
    for (code, _, _) in i18n::LANGUAGES.iter().filter(|l| l.0 != "en") {
        let cat = i18n::catalog(code);
        assert!(!cat.is_empty(), "{code}: the catalog reads");
        let missing: Vec<&String> = wanted.iter().filter(|w| !cat.contains_key(*w)).collect();
        assert!(missing.is_empty(), "{code}: no translation for {missing:?}");
        let stale: Vec<&String> = cat.keys().filter(|k| !src.contains(&format!("\"{k}\""))).collect();
        assert!(stale.is_empty(), "{code}: no longer in the code: {stale:?}");
    }
}

#[test]
fn switching_language_switches_the_interface() {
    let mut app = Manuscript::new();
    let mut sh = Shell::new("Test", "0", wordsmith_manuscript_ui::brand::tokens());
    app.run(&Cmd::new("view.language.es"), &mut sh).unwrap();
    assert_eq!(i18n::language(), "es");
    assert_eq!(app.view.language, "es");
    let save = app.command("file.save").or_else(|| app.all_commands().into_iter().find(|c| c.shortcut == "Mod+S")).unwrap();
    assert_eq!(save.label, "Guardar");
    assert_eq!(i18n::tr("A text no catalog has"), "A text no catalog has");
    app.run(&Cmd::new("view.language.en"), &mut sh).unwrap();
    assert_eq!(app.all_commands().into_iter().find(|c| c.shortcut == "Mod+S").unwrap().label, "Save");
}
