// SPDX-License-Identifier: MIT OR Apache-2.0
//! `cargo xtask brand`: the files in `assets/brand/` are the brand kit's, unchanged.
//!
//! The Wordsmith brand kit (the `brand/` folder of the wordsmith repository) owns every logo and
//! icon and the seven palette colors. Its tool writes the kit's files for this app into
//! `assets/brand/`, with a record, `assets/brand/kit.json`: the kit commit the copies were taken
//! at and the list of what the folder holds (project decision 0007, Route F of the project map).
//! In that list a file is *carried* (a kit file, byte for byte, with its SHA-256), *own* (this
//! app writes it) or *derived* (this app makes it from a carried file).
//!
//! This module is Manuscript's own. It holds two checks:
//!
//! - `cargo xtask brand`, in the gate, with no network. Every carried file has the checksum the
//!   record gives; every file in the folder is in the record's list, or is the record; the
//!   theme file's `palette` holds the seven colors the record lists. So a kit file edited here,
//!   a file added by hand and a theme that left the palette all fail before they land.
//! - `cargo xtask brand --kit [<file>]`, outside the gate. It compares the record's list with
//!   this app's entry in the kit's own list, `brand/apps.json` on the wordsmith repository's
//!   `main` (fetched with `curl`; a file named after `--kit` is read in its place). It fails
//!   when the kit has moved since the copies were taken. A workflow runs it every week.
//!
//! The copies are taken again with the kit's tool, never by hand: with the wordsmith repository
//! checked out beside this one, `npm run brand -- --app manuscript` in its `website/wiki`.

use std::path::Path;
use std::process::Command;

use crate::shared::json::{self, Value};
use crate::shared::package::{files_under, sha256_of};
use crate::shared::util::{TaskResult, report};

/// The folder of brand files, from the root.
const FOLDER: &str = "assets/brand";
/// The record the kit's tool leaves in the folder.
const RECORD: &str = "kit.json";
/// This app's name in the kit's list.
const APP: &str = "manuscript";
/// The kit's list of what each app carries, as published.
const KIT_LIST: &str = "https://raw.githubusercontent.com/jamescjfoundation/wordsmith/main/brand/apps.json";
/// How the copies are taken again.
const RETAKE: &str = "with the wordsmith repository checked out beside this one, run `npm run brand -- --app manuscript` in its website/wiki";

/// The names under `key` of the record's list: its `file` members.
fn files_of<'a>(app: &'a Value, key: &str) -> Vec<&'a str> {
    app.get(key).map(Value::items).unwrap_or_default().iter().filter_map(|one| one.str_of("file")).collect()
}

/// The colors a JSON object names, upper case and sorted: the record's `palette`, or a theme
/// file's.
fn colors(palette: Option<&Value>) -> Vec<String> {
    let mut out: Vec<String> = match palette {
        Some(Value::Object(members)) => members.iter().filter_map(|(_, v)| v.as_str()).map(str::to_uppercase).collect(),
        _ => Vec::new(),
    };
    out.sort();
    out
}

/// What is wrong with the folder, given the record's text, the files in the folder (paths from
/// it), the checksum of a file in it and the text of a file in it.
fn problems(
    record: &str,
    present: &[String],
    checksum: impl Fn(&str) -> Result<String, String>,
    text: impl Fn(&str) -> Option<String>,
) -> Vec<String> {
    let parsed = match json::parse(record) {
        Ok(parsed) => parsed,
        Err(e) => return vec![format!("{FOLDER}/{RECORD} cannot be read: {e}")],
    };
    let Some(app) = parsed.get("app") else {
        return vec![format!("{FOLDER}/{RECORD} has no `app` list. It is written by the kit's tool: {RETAKE}")];
    };
    let mut out = Vec::new();
    let carried = app.get("carried").map(Value::items).unwrap_or_default();
    if carried.is_empty() {
        out.push(format!("{FOLDER}/{RECORD} lists no carried file"));
    }
    for one in carried {
        let (Some(file), Some(wanted)) = (one.str_of("file"), one.str_of("sha256")) else {
            out.push(format!("{FOLDER}/{RECORD}: a carried file has no `file` or no `sha256`"));
            continue;
        };
        let from = one.str_of("from").unwrap_or("the kit");
        if !present.iter().any(|p| p == file) {
            out.push(format!("{FOLDER}/{file} is missing (the kit's {from})"));
            continue;
        }
        match checksum(file) {
            Ok(found) if found == wanted => {}
            Ok(_) => out.push(format!("{FOLDER}/{file} is not the kit's {from} as the record has it. A kit file is never changed here")),
            Err(e) => out.push(e),
        }
    }
    // The app's own files and the ones it derives are named in the list and must be there.
    for (key, kind) in [("own", "this app's own"), ("derived", "made here from a kit file")] {
        for file in files_of(app, key) {
            if !present.iter().any(|p| p == file) {
                out.push(format!("{FOLDER}/{file} is missing ({kind}, by the record's list)"));
            }
        }
    }
    let known: Vec<&str> = ["carried", "own", "derived"].iter().flat_map(|key| files_of(app, key)).chain([RECORD]).collect();
    for file in present {
        if !known.contains(&file.as_str()) {
            out.push(format!("{FOLDER}/{file} is not in the record's list. A brand file comes from the kit; a file of this app's own is named in the kit's list first"));
        }
    }
    // The theme is this app's own, but for the seven palette colors.
    let wanted = colors(app.get("palette"));
    match app.str_of("theme") {
        None => out.push(format!("{FOLDER}/{RECORD} names no theme file")),
        Some(theme) => match text(theme).map(|t| json::parse(&t)) {
            None => out.push(format!("{FOLDER}/{theme}, the theme file, cannot be read")),
            Some(Err(e)) => out.push(format!("{FOLDER}/{theme}: {e}")),
            Some(Ok(tokens)) => {
                let found = colors(tokens.get("palette"));
                if wanted.len() != 7 {
                    out.push(format!("{FOLDER}/{RECORD}: the palette has {} colors, not seven", wanted.len()));
                } else if found != wanted {
                    out.push(format!("{FOLDER}/{theme}: its palette is {}; the kit's seven colors are {}", found.join(" "), wanted.join(" ")));
                }
            }
        },
    }
    out
}

/// What differs between the record's list and the kit's own list (`brand/apps.json`), as
/// sentences. Empty: the copies here are what the kit gives now.
fn moved(record: &str, kit_list: &str) -> Result<Vec<String>, String> {
    let record = json::parse(record).map_err(|e| format!("{FOLDER}/{RECORD}: {e}"))?;
    let kit = json::parse(kit_list).map_err(|e| format!("the kit's list: {e}"))?;
    let ours = record.get("app").ok_or_else(|| format!("{FOLDER}/{RECORD} has no `app` list"))?;
    let theirs = kit.get("apps").and_then(|apps| apps.get(APP)).ok_or_else(|| format!("the kit's list has no entry for {APP}"))?;
    if ours == theirs {
        return Ok(Vec::new());
    }
    let sums = |app: &Value| -> Vec<(String, String)> {
        let carried = app.get("carried").map(Value::items).unwrap_or_default();
        carried.iter().filter_map(|one| Some((one.str_of("file")?.to_owned(), one.str_of("sha256")?.to_owned()))).collect()
    };
    let (here, there) = (sums(ours), sums(theirs));
    let mut out = Vec::new();
    for (file, sum) in &there {
        match here.iter().find(|(name, _)| name == file) {
            None => out.push(format!("{file}: the kit now gives this file, and it is not taken yet")),
            Some((_, taken)) if taken != sum => out.push(format!("{file}: changed in the kit since it was taken")),
            Some(_) => {}
        }
    }
    for (file, _) in &here {
        if !there.iter().any(|(name, _)| name == file) {
            out.push(format!("{file}: no longer in the kit's list for this app"));
        }
    }
    if colors(ours.get("palette")) != colors(theirs.get("palette")) {
        out.push("the palette: the kit's seven colors changed".to_owned());
    }
    if out.is_empty() {
        out.push(
            "the list differs in something other than a carried file or the palette (the app's own or derived files, the theme's name)".to_owned(),
        );
    }
    Ok(out)
}

/// The kit's list as published, fetched with `curl`.
fn published_list() -> Result<String, String> {
    let out = Command::new("curl")
        .args(["--fail", "--silent", "--show-error", "--location", "--max-time", "60", KIT_LIST])
        .output()
        .map_err(|e| format!("could not run curl to read {KIT_LIST}: {e}"))?;
    if !out.status.success() {
        return Err(format!("could not read {KIT_LIST}: {}", String::from_utf8_lossy(&out.stderr).trim()));
    }
    String::from_utf8(out.stdout).map_err(|_| format!("{KIT_LIST} is not text"))
}

/// `cargo xtask brand [--kit [<file>]]`.
pub fn task(root: &Path, args: &[String]) -> TaskResult {
    let folder = root.join(FOLDER);
    let record =
        std::fs::read_to_string(folder.join(RECORD)).map_err(|e| format!("{FOLDER}/{RECORD} cannot be read ({e}). To write it: {RETAKE}"))?;
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    match args.as_slice() {
        [] => {
            let present = files_under(&folder)?;
            let found = problems(&record, &present, |file| sha256_of(&folder.join(file)), |file| std::fs::read_to_string(folder.join(file)).ok());
            let failed = !found.is_empty();
            report(
                found,
                &format!("{} files in {FOLDER} are the brand kit's or this app's own, and the theme holds the kit's palette", present.len()),
            )
            .map_err(|e| if failed { format!("{e}\nTo take the kit's files again: {RETAKE}") } else { e })
        }
        ["--kit"] => kit(&record, &published_list()?),
        ["--kit", file] => kit(&record, &std::fs::read_to_string(file).map_err(|e| format!("cannot read {file}: {e}"))?),
        _ => Err("usage: cargo xtask brand [--kit [<the kit's brand/apps.json>]]".to_owned()),
    }
}

fn kit(record: &str, kit_list: &str) -> TaskResult {
    let found = moved(record, kit_list)?;
    let failed = !found.is_empty();
    report(found, &format!("the brand kit's list for {APP} is the one {FOLDER}/{RECORD} records"))
        .map_err(|e| if failed { format!("{e}\nThe brand kit has moved since the copies here were taken. To take them again: {RETAKE}") } else { e })
}

#[cfg(test)]
mod tests {
    use super::*;

    const RECORD_TEXT: &str = r##"{
      "kit": { "repository": "example/kit", "commit": "abc" },
      "app": {
        "carried": [
          { "file": "icon.png", "from": "brand/app/icon.png", "sha256": "aa" },
          { "file": "mark.svg", "from": "brand/app/mark.svg", "sha256": "bb" }
        ],
        "own": [ { "file": "tokens.json", "what": "the theme" } ],
        "derived": [],
        "theme": "tokens.json",
        "palette": { "ink": "#111111", "paper": "#FFFFFF", "app-color": "#D24B3E", "app-color-text": "#A8322A", "margin-blue": "#3B4FA8", "slate": "#5A6080", "night": "#12162B" }
      }
    }"##;
    const THEME: &str = r##"{ "palette": { "ink": "#111111", "paper": "#ffffff", "card-red": "#D24B3E", "card-red-text": "#A8322A", "margin-blue": "#3B4FA8", "slate": "#5A6080", "night": "#12162B" } }"##;

    fn check(present: &[&str], sums: &[(&str, &str)], theme: Option<&str>) -> Vec<String> {
        let present: Vec<String> = present.iter().map(|s| (*s).to_owned()).collect();
        problems(
            RECORD_TEXT,
            &present,
            |file| sums.iter().find(|(name, _)| *name == file).map(|(_, sum)| (*sum).to_owned()).ok_or_else(|| format!("{file}: no checksum")),
            |file| (file == "tokens.json").then_some(theme).flatten().map(str::to_owned),
        )
    }

    const ALL: &[&str] = &["icon.png", "kit.json", "mark.svg", "tokens.json"];
    const SUMS: &[(&str, &str)] = &[("icon.png", "aa"), ("mark.svg", "bb")];

    #[test]
    fn a_folder_that_is_the_kits_passes() {
        // The theme may name the app's color as it likes and write a color in either case: the
        // seven colors are compared, not their names.
        assert_eq!(check(ALL, SUMS, Some(THEME)), Vec::<String>::new());
    }

    #[test]
    fn a_changed_missing_or_added_file_fails() {
        let changed = check(ALL, &[("icon.png", "aa"), ("mark.svg", "cc")], Some(THEME));
        assert_eq!(changed.len(), 1, "{changed:?}");
        assert!(changed[0].contains("assets/brand/mark.svg is not the kit's brand/app/mark.svg"), "{changed:?}");
        let missing = check(&["icon.png", "kit.json", "tokens.json"], SUMS, Some(THEME));
        assert!(missing.iter().any(|p| p.contains("assets/brand/mark.svg is missing")), "{missing:?}");
        let added = check(&["icon.png", "kit.json", "mark.svg", "sketch.png", "tokens.json"], SUMS, Some(THEME));
        assert_eq!(added.len(), 1, "{added:?}");
        assert!(added[0].contains("assets/brand/sketch.png is not in the record's list"), "{added:?}");
        // A file the list calls the app's own must be there too.
        let no_theme = check(&["icon.png", "kit.json", "mark.svg"], SUMS, None);
        assert!(no_theme.iter().any(|p| p.contains("tokens.json is missing")), "{no_theme:?}");
    }

    #[test]
    fn a_theme_that_left_the_palette_fails() {
        let other = THEME.replace("#3B4FA8", "#3B4FA9");
        let found = check(ALL, SUMS, Some(&other));
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].contains("tokens.json: its palette is") && found[0].contains("#3B4FA9"), "{found:?}");
        assert!(check(ALL, SUMS, Some("not json")).iter().any(|p| p.contains("tokens.json")));
    }

    #[test]
    fn a_record_that_cannot_be_used_says_so() {
        let none: [String; 0] = [];
        assert!(problems("{", &none, |_| Ok(String::new()), |_| None)[0].contains("cannot be read"));
        assert!(problems("{}", &none, |_| Ok(String::new()), |_| None)[0].contains("has no `app` list"));
    }

    #[test]
    fn the_kits_list_is_compared_with_the_record() {
        let list = |entry: &str| format!(r#"{{ "note": "n", "apps": {{ "manuscript": {entry}, "other": {{}} }} }}"#);
        let ours = RECORD_TEXT.split_once(r#""app": "#).map(|(_, rest)| rest.trim_end().trim_end_matches('}').trim_end()).unwrap();
        assert_eq!(moved(RECORD_TEXT, &list(ours)), Ok(Vec::new()));
        let changed = moved(RECORD_TEXT, &list(&ours.replace(r#""sha256": "bb""#, r#""sha256": "dd""#))).unwrap();
        assert_eq!(changed, ["mark.svg: changed in the kit since it was taken"]);
        let more = ours.replace(r#"{ "file": "icon.png""#, r#"{ "file": "new.svg", "from": "x", "sha256": "ee" }, { "file": "icon.png""#);
        assert_eq!(moved(RECORD_TEXT, &list(&more)).unwrap(), ["new.svg: the kit now gives this file, and it is not taken yet"]);
        let recolored = moved(RECORD_TEXT, &list(&ours.replace("#12162B", "#12162C"))).unwrap();
        assert_eq!(recolored, ["the palette: the kit's seven colors changed"]);
        assert!(moved(RECORD_TEXT, r#"{ "apps": {} }"#).unwrap_err().contains("no entry for manuscript"));
    }

    /// The record in the repository is one the check accepts, with the files beside it.
    #[test]
    fn the_folder_in_this_repository_passes() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        assert_eq!(task(&root, &[]), Ok(()));
    }
}
