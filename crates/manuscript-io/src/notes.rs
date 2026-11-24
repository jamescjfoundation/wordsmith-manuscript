// SPDX-License-Identifier: MIT OR Apache-2.0
//! Importing a folder of Markdown notes as bible entries (IMP-6, SPEC 15.7).
//!
//! Each `.md` file becomes an entry named after the file; its front matter (`---` YAML-style
//! `key: value` lines, or `+++` TOML) becomes fields, aliases and the type; `[[Name]]` and
//! `[[Name|text]]` become links to the entry of that name, keeping the display text. A link with
//! no matching entry keeps its text and is reported. Subfolders become bible folders. Reading is
//! bounded: file count, depth and sizes.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use wordsmith_manuscript_format::dialect;
use wordsmith_manuscript_model::entries::{self, SCHEME};
use wordsmith_manuscript_model::{Block, Entry, Inline, ItemId};

/// The most note files read.
pub const MAX_NOTES: usize = 20_000;
/// The deepest folder read.
pub const MAX_DEPTH: usize = 16;
/// The largest note read.
pub const MAX_NOTE: u64 = 8 * 1024 * 1024;

/// The link scheme used while names are not resolved yet.
const PENDING: &str = "wsm-name:";

/// One imported note.
#[derive(Clone, Debug, PartialEq)]
pub struct Note {
    /// Its path inside the folder (`Characters/Anna.md`).
    pub path: String,
    /// The folders it sits in, outermost first.
    pub folders: Vec<String>,
    /// The id it will have.
    pub id: ItemId,
    /// The entry.
    pub entry: Entry,
}

/// What an import found.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct NotesReport {
    /// Notes read.
    pub notes: usize,
    /// Links resolved to an entry.
    pub resolved: usize,
    /// Links with no matching entry: (note, name).
    pub unresolved: Vec<(String, String)>,
    /// Files left out, and why.
    pub skipped: Vec<(String, String)>,
    /// Notes whose words did not all arrive (always empty when the import is sound).
    pub word_mismatches: Vec<String>,
}

/// A folder of notes, read.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct NotesFolder {
    /// The notes, in path order.
    pub notes: Vec<Note>,
    /// What happened.
    pub report: NotesReport,
}

/// Read every `.md` note under `root`. Links stay pending until [`resolve`].
pub fn read_folder(root: &Path, types: &[(&str, &str)], next_id: &mut dyn FnMut() -> ItemId) -> Result<NotesFolder, String> {
    let mut files: Vec<(PathBuf, Vec<String>)> = Vec::new();
    let mut skipped = Vec::new();
    let mut stack: Vec<(PathBuf, Vec<String>)> = vec![(root.to_path_buf(), Vec::new())];
    while let Some((dir, folders)) = stack.pop() {
        if folders.len() > MAX_DEPTH {
            skipped.push((folders.join("/"), "too deep".to_owned()));
            continue;
        }
        let entries = std::fs::read_dir(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        for e in entries.flatten() {
            let Ok(kind) = e.file_type() else { continue };
            let name = e.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') || kind.is_symlink() {
                continue;
            }
            if kind.is_dir() {
                let mut f = folders.clone();
                f.push(name);
                stack.push((e.path(), f));
            } else if name.to_lowercase().ends_with(".md") {
                files.push((e.path(), folders.clone()));
                if files.len() > MAX_NOTES {
                    return Err(format!("the folder holds more than {MAX_NOTES} notes"));
                }
            }
        }
    }
    files.sort();
    let mut notes = Vec::new();
    for (path, folders) in files {
        let rel = path.strip_prefix(root).map(|p| p.to_string_lossy().replace('\\', "/")).unwrap_or_default();
        let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        if size > MAX_NOTE {
            skipped.push((rel, "larger than the limit".into()));
            continue;
        }
        let text = match std::fs::read(&path).map(String::from_utf8) {
            Ok(Ok(t)) => t,
            _ => {
                skipped.push((rel, "not UTF-8 text".into()));
                continue;
            }
        };
        let name = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        let entry = note_entry(&name, &text, &folders, types);
        notes.push(Note { path: rel, folders, id: next_id(), entry });
    }
    // Links stay pending until [`resolve`] is given the project's other entries too.
    Ok(NotesFolder { report: NotesReport { notes: notes.len(), skipped, ..NotesReport::default() }, notes })
}

/// Parse a front matter's simple `key: value` lines (and `- item` lists under a key).
fn front_matter(text: &str) -> (BTreeMap<String, Vec<String>>, &str) {
    let mut out: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let (fence, rest) = if let Some(r) = text.strip_prefix("---\n").or_else(|| text.strip_prefix("---\r\n")) {
        ("---", r)
    } else if let Some(r) = text.strip_prefix("+++\n").or_else(|| text.strip_prefix("+++\r\n")) {
        ("+++", r)
    } else {
        return (out, text);
    };
    let mut offset = 0;
    let mut last_key: Option<String> = None;
    for line in rest.split_inclusive('\n') {
        offset += line.len();
        let l = line.trim_end_matches(['\r', '\n']);
        if l.trim() == fence {
            let body = &rest[offset..];
            return (out, body.strip_prefix('\n').or_else(|| body.strip_prefix("\r\n")).unwrap_or(body));
        }
        let t = l.trim();
        if let Some(item) = t.strip_prefix("- ") {
            if let Some(k) = &last_key {
                out.entry(k.clone()).or_default().push(unquote(item));
            }
            continue;
        }
        let sep = if fence == "+++" { '=' } else { ':' };
        if let Some((k, v)) = t.split_once(sep) {
            let k = k.trim().trim_matches('"').to_owned();
            let v = v.trim();
            last_key = Some(k.clone());
            if v.is_empty() {
                out.entry(k).or_default();
            } else if let Some(list) = v.strip_prefix('[').and_then(|x| x.strip_suffix(']')) {
                out.entry(k).or_default().extend(list.split(',').map(unquote).filter(|x| !x.is_empty()));
            } else {
                out.entry(k).or_default().push(unquote(v));
            }
        }
    }
    // Not closed: no front matter after all.
    (BTreeMap::new(), text)
}

fn unquote(s: &str) -> String {
    s.trim().trim_matches(|c| c == '"' || c == '\'').trim().to_owned()
}

/// Encode a name for a pending link destination.
fn encode(name: &str) -> String {
    let mut out = String::new();
    for c in name.chars() {
        match c {
            ' ' => out.push_str("%20"),
            '(' => out.push_str("%28"),
            ')' => out.push_str("%29"),
            '%' => out.push_str("%25"),
            '\\' => out.push_str("%5C"),
            c => out.push(c),
        }
    }
    out
}

fn decode(s: &str) -> String {
    s.replace("%20", " ").replace("%28", "(").replace("%29", ")").replace("%5C", "\\").replace("%25", "%")
}

/// Escape text for the dialect's link label.
fn label(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        if "\\*[]~_<`".contains(c) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// Turn a note's Markdown into the dialect: soft-wrapped lines joined into paragraphs, and
/// `[[links]]` into pending links.
pub fn to_dialect(body: &str) -> String {
    let mut out = String::new();
    let mut para: Vec<String> = Vec::new();
    let flush = |para: &mut Vec<String>, out: &mut String| {
        if !para.is_empty() {
            out.push_str(&para.join(" "));
            out.push_str("\n\n");
            para.clear();
        }
    };
    let starts_block = |l: &str| {
        let t = l.trim_start();
        t.starts_with('#')
            || t.starts_with('>')
            || t.starts_with("- ")
            || t.starts_with("* ")
            || t.starts_with("+ ")
            || t.starts_with("```")
            || t.chars().next().is_some_and(|c| c.is_ascii_digit()) && t.contains(". ")
    };
    for line in body.lines() {
        if line.trim().is_empty() {
            flush(&mut para, &mut out);
            continue;
        }
        let l = links(line.trim());
        if starts_block(line) {
            flush(&mut para, &mut out);
            // A list item or heading stays a paragraph of its own; list markers are kept as text.
            let t = line.trim_start();
            let kept = if t.starts_with("- ") || t.starts_with("* ") || t.starts_with("+ ") { format!("\\{l}") } else { l };
            out.push_str(&kept);
            out.push_str("\n\n");
            continue;
        }
        para.push(l);
    }
    flush(&mut para, &mut out);
    out
}

/// `[[Name]]`, `[[Name|text]]` and `[[Name#part]]` as pending links; `![[embeds]]` as text.
fn links(line: &str) -> String {
    let mut out = String::new();
    let mut rest = line;
    while let Some(at) = rest.find("[[") {
        let Some(end) = rest[at + 2..].find("]]") else { break };
        let inner = &rest[at + 2..at + 2 + end];
        let embed = rest[..at].ends_with('!');
        out.push_str(&rest[..if embed { at - 1 } else { at }]);
        let (target, shown) = inner.split_once('|').map_or((inner, inner), |(a, b)| (a, b));
        let target = target.split('#').next().unwrap_or(target).trim();
        if embed || target.is_empty() {
            out.push_str(&label(shown));
        } else {
            out.push_str(&format!("[{}]({PENDING}{})", label(shown.trim()), encode(target)));
        }
        rest = &rest[at + 2 + end + 2..];
    }
    out.push_str(rest);
    out
}

fn note_entry(name: &str, text: &str, folders: &[String], types: &[(&str, &str)]) -> Entry {
    let (fm, body) = front_matter(text);
    let mut e = Entry::new("", name);
    for (k, v) in &fm {
        let key = k.to_lowercase();
        match key.as_str() {
            "aliases" | "alias" => e.aliases.extend(v.iter().cloned()),
            "type" | "kind" | "category" => e.kind = v.first().cloned().unwrap_or_default().to_lowercase(),
            "title" | "name" => {
                if let Some(n) = v.first().filter(|n| !n.is_empty()) {
                    e.name = n.clone();
                }
            }
            _ => {
                e.fields.insert(k.clone(), v.join(if v.len() > 1 { "\n" } else { "" }));
            }
        }
    }
    // The type: from front matter if it is a known type, else from a folder named like a type.
    let by_kind = types.iter().find(|(kind, _)| *kind == e.kind).map(|(k, _)| (*k).to_owned());
    let by_folder = folders.iter().rev().find_map(|f| types.iter().find(|(_, plural)| plural.eq_ignore_ascii_case(f)).map(|(k, _)| (*k).to_owned()));
    e.kind = by_kind.or(by_folder).unwrap_or_else(|| "lore".into());
    e.body = dialect::parse(&to_dialect(body));
    e
}

/// Resolve pending links by name against the notes and `others` (entries already in the project).
pub fn resolve(folder: &mut NotesFolder, others: &[(ItemId, Entry)]) {
    let all: Vec<(ItemId, Entry)> = folder.notes.iter().map(|n| (n.id, n.entry.clone())).chain(others.iter().cloned()).collect();
    let refs: Vec<(&ItemId, &Entry)> = all.iter().map(|(i, e)| (i, e)).collect();
    let mut resolved = 0;
    let mut unresolved = Vec::new();
    for note in &mut folder.notes {
        let mut fix = |content: &mut Vec<Inline>| {
            for i in content.iter_mut() {
                let marks = match i {
                    Inline::Span { marks, .. } | Inline::NoteRef { marks, .. } => marks,
                };
                if let Some(name) = marks.link.as_deref().and_then(|l| l.strip_prefix(PENDING)).map(decode) {
                    match entries::find_by_name(refs.iter().copied(), &name) {
                        Some(id) => {
                            marks.link = Some(format!("{SCHEME}{id}"));
                            resolved += 1;
                        }
                        None => {
                            marks.link = None;
                            if !unresolved.contains(&(note.path.clone(), name.clone())) {
                                unresolved.push((note.path.clone(), name));
                            }
                        }
                    }
                }
            }
            wordsmith_manuscript_model::text::normalize_inlines(content);
        };
        for b in &mut note.entry.body.blocks {
            match b {
                Block::Para(c) | Block::Heading { content: c, .. } => fix(c),
                Block::Quote(ps) | Block::Fenced { paras: ps, .. } => ps.iter_mut().for_each(&mut fix),
                _ => {}
            }
        }
    }
    folder.report.resolved = resolved;
    folder.report.unresolved = unresolved;
}

/// The letters and digits of a text, word by word: what a note's words are, whatever its markup.
pub fn bare_words(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()).map(str::to_owned).collect()
}

/// Check that every note's words arrived: the note's text without markup (links as their shown
/// text) against the entry's body.
pub fn check_words(root: &Path, folder: &mut NotesFolder) {
    for note in &folder.notes {
        let Ok(text) = std::fs::read_to_string(root.join(&note.path)) else { continue };
        let (_, body) = front_matter(&text);
        let shown = shown_text(body);
        let want = bare_words(&shown);
        let got: Vec<String> = note.entry.body.plain_paragraphs().iter().flat_map(|p| bare_words(p)).collect();
        if want != got {
            folder.report.word_mismatches.push(note.path.clone());
        }
    }
}

/// A note's text with `[[links]]` as the text they show.
fn shown_text(body: &str) -> String {
    let mut out = String::new();
    let mut rest = body;
    while let Some(at) = rest.find("[[") {
        let Some(end) = rest[at + 2..].find("]]") else { break };
        let inner = &rest[at + 2..at + 2 + end];
        out.push_str(&rest[..at]);
        out.push(' ');
        out.push_str(inner.split_once('|').map_or(inner, |(_, b)| b));
        out.push(' ');
        rest = &rest[at + 2 + end + 2..];
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn front_matter_in_yaml_and_toml_styles() {
        let (fm, body) = front_matter("---\naliases: [Annie, \"The Fox\"]\nage: 34\ntags:\n  - brave\n  - old\n---\nBody.\n");
        assert_eq!(fm["aliases"], ["Annie", "The Fox"]);
        assert_eq!(fm["age"], ["34"]);
        assert_eq!(fm["tags"], ["brave", "old"]);
        assert_eq!(body, "Body.\n");
        let (fm, _) = front_matter("+++\nage = 34\n+++\nx");
        assert_eq!(fm["age"], ["34"]);
        let (fm, body) = front_matter("---\nnot closed\nBody");
        assert!(fm.is_empty());
        assert!(body.starts_with("---"));
    }

    #[test]
    fn links_and_soft_wraps() {
        let d = to_dialect("Anna met [[Bram Oak|Bram]] at\nthe [[Old Mill]] and ![[map]].\n\n- a [[Nobody]] item\n");
        assert!(d.starts_with("Anna met [Bram](wsm-name:Bram%20Oak) at the [Old Mill](wsm-name:Old%20Mill) and map.\n\n"));
        assert!(d.contains("\\- a [Nobody](wsm-name:Nobody) item"));
    }
}
