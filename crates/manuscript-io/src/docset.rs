// SPDX-License-Identifier: MIT OR Apache-2.0
//! Importing a set of documents (IMP-2, SPEC 15.5): each file is placed by the role its name
//! gives in parentheses ("(Plan)" into Notes, "(Rough Work)" into Research, a draft into the
//! manuscript); several files of one draft are merged in name order into one manuscript; when a
//! set holds several drafts, the latest goes into the manuscript and the others into Research.
//! The name rules are editable and kept in the project.

use std::cmp::Ordering;
use std::path::{Path, PathBuf};

use jcj_doc::Document;
use jcj_doc::para::InlineObject;
use serde::{Deserialize, Serialize};

use crate::import::{self, TextRules};
use crate::mapping::{self, Plan, Rules};

/// Where a file of a set goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Dest {
    /// The writer's notes and plans.
    Notes,
    /// Research and rough work.
    Research,
    /// A draft of the manuscript.
    Draft,
}

/// A name rule: a marker in a file's name and where such files go.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NameRule {
    /// The text in the name, such as `(Plan)` (matched without regard to case).
    pub marker: String,
    /// Where the file goes.
    pub dest: Dest,
    /// For drafts: which draft (higher is later).
    #[serde(default)]
    pub draft: u32,
}

impl NameRule {
    fn new(marker: &str, dest: Dest, draft: u32) -> NameRule {
        NameRule { marker: marker.into(), dest, draft }
    }
}

/// The set's rules: name rules, tried in order.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SetRules {
    /// The name rules.
    pub names: Vec<NameRule>,
}

impl Default for SetRules {
    fn default() -> Self {
        SetRules {
            names: vec![
                NameRule::new("(Plan)", Dest::Notes, 0),
                NameRule::new("(Outline)", Dest::Notes, 0),
                NameRule::new("(Rough Work)", Dest::Research, 0),
                NameRule::new("(Notes)", Dest::Research, 0),
                NameRule::new("(Research)", Dest::Research, 0),
                NameRule::new("(First Draft)", Dest::Draft, 1),
                NameRule::new("(Second Draft)", Dest::Draft, 2),
                NameRule::new("(Third Draft)", Dest::Draft, 3),
                NameRule::new("(Fourth Draft)", Dest::Draft, 4),
                NameRule::new("(Final Draft)", Dest::Draft, 99),
                NameRule::new("(Draft)", Dest::Draft, 1),
            ],
        }
    }
}

/// One file of a set and what its name says.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SetFile {
    /// The file.
    pub path: PathBuf,
    /// Its name without extension.
    pub name: String,
    /// Where it goes.
    pub dest: Dest,
    /// Its draft (for drafts).
    pub draft: u32,
    /// The marker that placed it (empty: none, so a draft by default).
    pub marker: String,
}

/// Compare names as people read numbers in them ("Part 2" before "Part 10").
pub fn natural(a: &str, b: &str) -> Ordering {
    let (mut x, mut y) = (a.chars().peekable(), b.chars().peekable());
    loop {
        match (x.peek().copied(), y.peek().copied()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(c), Some(d)) if c.is_ascii_digit() && d.is_ascii_digit() => {
                let mut na = String::new();
                while let Some(c) = x.peek().filter(|c| c.is_ascii_digit()) {
                    na.push(*c);
                    x.next();
                }
                let mut nb = String::new();
                while let Some(c) = y.peek().filter(|c| c.is_ascii_digit()) {
                    nb.push(*c);
                    y.next();
                }
                let o = na
                    .trim_start_matches('0')
                    .len()
                    .cmp(&nb.trim_start_matches('0').len())
                    .then_with(|| na.trim_start_matches('0').cmp(nb.trim_start_matches('0')));
                if o != Ordering::Equal {
                    return o;
                }
            }
            (Some(c), Some(d)) => {
                let o = c.to_lowercase().cmp(d.to_lowercase());
                if o != Ordering::Equal {
                    return o;
                }
                x.next();
                y.next();
            }
        }
    }
}

/// Place each file by its name.
pub fn classify(paths: &[PathBuf], rules: &SetRules) -> Vec<SetFile> {
    let mut out: Vec<SetFile> = paths
        .iter()
        .map(|p| {
            let name = p.file_stem().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            let lower = name.to_lowercase();
            let rule = rules.names.iter().find(|r| lower.contains(&r.marker.to_lowercase()));
            match rule {
                Some(r) => SetFile { path: p.clone(), name, dest: r.dest, draft: r.draft, marker: r.marker.clone() },
                None => SetFile { path: p.clone(), name, dest: Dest::Draft, draft: 1, marker: String::new() },
            }
        })
        .collect();
    out.sort_by(|a, b| natural(&a.name, &b.name));
    out
}

/// The files of a folder that can be imported (not hidden), for a set.
pub fn folder_files(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut out = Vec::new();
    for e in std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?.flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') || !e.file_type().is_ok_and(|t| t.is_file()) {
            continue;
        }
        if jcj_interchange::Format::from_name(&name).is_some_and(jcj_interchange::Format::can_import) {
            out.push(e.path());
        }
        if out.len() > 1000 {
            return Err("a set holds at most 1,000 files".into());
        }
    }
    Ok(out)
}

/// Fix each paragraph's heading level on the paragraph itself (from its own document's styles),
/// so documents can be joined whatever their style sheets.
fn settle_headings(doc: &mut Document) {
    let levels: Vec<Option<u8>> = doc.body.iter().map(|b| b.as_para().and_then(|p| mapping::heading_level(doc, p)).map(|l| (l - 1) as u8)).collect();
    for (b, l) in doc.body.iter_mut().zip(levels) {
        if let (Some(l), jcj_doc::Block::Para(p)) = (l, std::sync::Arc::make_mut(b)) {
            p.props.outline_level = Some(l);
        }
    }
}

/// Join documents into one, in order: notes and comments renumbered so none collide.
pub fn join(docs: Vec<Document>) -> Document {
    let mut out = jcj_doc::new_document();
    out.body.clear();
    let mut next_part = 1u32;
    let mut next_comment = 1u32;
    for mut d in docs {
        settle_headings(&mut d);
        let part_map: std::collections::BTreeMap<u32, u32> = d.parts.keys().map(|k| (*k, next_part + k)).collect();
        let comment_map: std::collections::BTreeMap<u32, u32> = d.comments.keys().map(|k| (*k, next_comment + k)).collect();
        next_part += d.parts.keys().max().map_or(0, |m| m + 1);
        next_comment += d.comments.keys().max().map_or(0, |m| m + 1);
        for (k, part) in d.parts {
            out.parts.insert(part_map.get(&k).copied().unwrap_or(k), part);
        }
        for (k, mut c) in d.comments {
            c.part = part_map.get(&c.part).copied().unwrap_or(c.part);
            out.comments.insert(comment_map.get(&k).copied().unwrap_or(k), c);
        }
        for mut b in d.body {
            if let jcj_doc::Block::Para(p) = std::sync::Arc::make_mut(&mut b) {
                let offsets = p.object_offsets();
                for off in offsets {
                    match p.object_at_mut(off) {
                        Some(InlineObject::NoteRef { id, .. }) => *id = part_map.get(id).copied().unwrap_or(*id),
                        Some(InlineObject::CommentStart { id } | InlineObject::CommentEnd { id }) => {
                            *id = comment_map.get(id).copied().unwrap_or(*id)
                        }
                        _ => {}
                    }
                }
            }
            out.body.push(b);
        }
    }
    if out.body.is_empty() {
        out.body.push(std::sync::Arc::new(jcj_doc::Block::Para(jcj_doc::Paragraph::new())));
    }
    out
}

/// A planned set: the manuscript (the latest draft's files joined), and the other files.
#[derive(Clone, Debug, Default)]
pub struct SetPlan {
    /// The files, as classified.
    pub files: Vec<SetFile>,
    /// The latest draft's plan, if the set has a draft.
    pub manuscript: Option<Plan>,
    /// The latest draft's name (the marker, or the first file's name).
    pub draft_name: String,
    /// Everything else: (where, folder title, plan).
    pub others: Vec<(Dest, String, Plan)>,
}

/// Plan a set's import.
pub fn plan_set(paths: &[PathBuf], set_rules: &SetRules, rules: &Rules, text: &TextRules) -> Result<SetPlan, String> {
    let files = classify(paths, set_rules);
    let latest = files.iter().filter(|f| f.dest == Dest::Draft).map(|f| f.draft).max();
    let mut out = SetPlan { files: files.clone(), ..SetPlan::default() };
    // Drafts, grouped by draft number.
    let mut drafts: std::collections::BTreeMap<u32, Vec<&SetFile>> = std::collections::BTreeMap::new();
    for f in files.iter().filter(|f| f.dest == Dest::Draft) {
        drafts.entry(f.draft).or_default().push(f);
    }
    for (n, group) in drafts {
        let docs = group.iter().map(|f| import::document(&f.path, text)).collect::<Result<Vec<_>, _>>()?;
        let plan = mapping::plan(&join(docs), rules);
        let name = group
            .first()
            .map(|f| if f.marker.is_empty() { f.name.clone() } else { f.marker.trim_matches(['(', ')']).to_owned() })
            .unwrap_or_default();
        if Some(n) == latest {
            out.manuscript = Some(plan);
            out.draft_name = name;
        } else {
            out.others.push((Dest::Research, name, plan));
        }
    }
    for f in files.iter().filter(|f| f.dest != Dest::Draft) {
        let plan = mapping::plan(&import::document(&f.path, text)?, rules);
        out.others.push((f.dest, f.name.clone(), plan));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_sort_as_people_read_them() {
        let mut v = vec!["Book Part 10", "Book Part 2", "Book Part 1", "book part 3"];
        v.sort_by(|a, b| natural(a, b));
        assert_eq!(v, ["Book Part 1", "Book Part 2", "book part 3", "Book Part 10"]);
    }

    #[test]
    fn markers_place_files() {
        let paths: Vec<PathBuf> = [
            "River (Plan).docx",
            "River (First Draft) 2.docx",
            "River (First Draft) 1.docx",
            "River (Rough Work).docx",
            "River (Second Draft).docx",
            "Loose.docx",
        ]
        .iter()
        .map(PathBuf::from)
        .collect();
        let files = classify(&paths, &SetRules::default());
        let got: Vec<(&str, Dest, u32)> = files.iter().map(|f| (f.name.as_str(), f.dest, f.draft)).collect();
        assert_eq!(
            got,
            [
                ("Loose", Dest::Draft, 1),
                ("River (First Draft) 1", Dest::Draft, 1),
                ("River (First Draft) 2", Dest::Draft, 1),
                ("River (Plan)", Dest::Notes, 0),
                ("River (Rough Work)", Dest::Research, 0),
                ("River (Second Draft)", Dest::Draft, 2),
            ]
        );
    }
}
