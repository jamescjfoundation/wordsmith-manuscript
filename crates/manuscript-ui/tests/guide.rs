// SPDX-License-Identifier: MIT OR Apache-2.0
//! The guide's keys page is made from the commands: it never goes out of step.

use std::path::Path;

#[test]
fn the_keys_page_lists_every_command_with_a_key() {
    let app = wordsmith_manuscript_ui::Manuscript::new();
    let mut rows: Vec<(String, String)> =
        jcj_shell::Commands::all_commands(&app).into_iter().filter(|c| !c.shortcut.is_empty()).map(|c| (c.label, c.shortcut)).collect();
    rows.sort();
    rows.dedup();
    // No two commands share a key, the shell's included.
    let mut keys: Vec<String> = rows
        .iter()
        .map(|r| r.1.clone())
        .chain(jcj_shell::commands::shell_commands().into_iter().map(|c| c.shortcut))
        .filter(|k| !k.is_empty())
        .collect();
    keys.sort();
    let before = keys.len();
    keys.dedup();
    assert_eq!(keys.len(), before, "two commands share a key");
    let mut page = String::from(
        "# Keys\n\nThe default keys (Mod is Ctrl, or Command on a Mac). Every key can be changed on File › Keys, and every command is in the\ncommand palette. This page is made from the commands (`manuscript-ui/tests/guide.rs`).\n\n| Command | Key |\n| --- | --- |\n",
    );
    for (label, key) in rows {
        page.push_str(&format!("| {label} | `{key}` |\n"));
    }
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/guide/keys.md");
    if std::env::var_os("UPDATE_GOLDEN").is_some() {
        std::fs::write(&path, &page).unwrap();
    }
    assert_eq!(std::fs::read_to_string(&path).unwrap_or_default(), page, "rerun with UPDATE_GOLDEN=1 to refresh docs/guide/keys.md");
}
