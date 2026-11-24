// SPDX-License-Identifier: MIT OR Apache-2.0
//! The files of a project folder, as text: `project.toml`, `binder.toml`, `text/<id>.md` and
//! `bible/<id>.md`. Everything here works on strings; [`crate::folder`] reads and writes them.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use wordsmith_manuscript_model::{Area, Binder, Entry, Item, ItemId, ItemMeta, ItemText, Kind, MAX_DEPTH, Place, Project, Settings};

use crate::{Error, SCHEMA, dialect, limits};

/// The front matter fence.
pub const FENCE: &str = "+++";

#[derive(Serialize, Deserialize)]
struct ProjectFile {
    schema: u32,
    #[serde(flatten)]
    settings: Settings,
}

/// `project.toml` for these settings.
pub fn write_project(settings: &Settings) -> Result<String, Error> {
    let file = ProjectFile { schema: SCHEMA, settings: settings.clone() };
    toml::to_string(&file).map_err(|e| Error::Write(format!("project.toml: {e}")))
}

/// The schema version a `project.toml` declares.
pub fn schema_of(text: &str) -> Result<u32, Error> {
    let table: toml::Table = text.parse().map_err(|e| Error::Damaged { file: "project.toml".into(), why: short(&e) })?;
    let schema = table.get("schema").and_then(toml::Value::as_integer).ok_or(Error::NotAProject)?;
    u32::try_from(schema).map_err(|_| Error::Damaged { file: "project.toml".into(), why: format!("schema {schema} is not a version") })
}

/// Read `project.toml`, refusing a newer schema and migrating an older one.
pub fn read_project(text: &str) -> Result<Settings, Error> {
    let schema = schema_of(text)?;
    if schema > SCHEMA {
        return Err(Error::Newer { found: schema, supported: SCHEMA });
    }
    if schema == 0 {
        return Err(Error::Damaged { file: "project.toml".into(), why: "schema 0 is not a version".into() });
    }
    let file: ProjectFile = toml::from_str(text).map_err(|e| Error::Damaged { file: "project.toml".into(), why: short(&e) })?;
    Ok(file.settings)
}

fn short(e: &impl std::fmt::Display) -> String {
    e.to_string().lines().take(6).collect::<Vec<_>>().join(" ")
}

#[derive(Serialize, Deserialize)]
struct BinderRow {
    area: Area,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    parent: Option<ItemId>,
    #[serde(flatten)]
    item: Item,
}

#[derive(Serialize, Deserialize)]
struct BinderFile {
    schema: u32,
    #[serde(default, rename = "item")]
    items: Vec<BinderRow>,
}

/// `binder.toml`: one `[[item]]` per item, depth first, area by area, so each parent comes before
/// its children and the order is the binder's.
pub fn write_binder(binder: &Binder) -> Result<String, Error> {
    let items = binder
        .rows()
        .into_iter()
        .map(|r| BinderRow { area: r.area, parent: r.parent, item: Item { children: Vec::new(), ..r.item.clone() } })
        .collect();
    toml::to_string(&BinderFile { schema: SCHEMA, items }).map_err(|e| Error::Write(format!("binder.toml: {e}")))
}

/// Read `binder.toml`. Every parent must come before its children and be in the same area; ids
/// are unique; the tree is no deeper than [`MAX_DEPTH`] and holds at most
/// [`limits::MAX_ITEMS`] items.
pub fn read_binder(text: &str) -> Result<Binder, Error> {
    let damaged = |why: String| Error::Damaged { file: "binder.toml".into(), why };
    let file: BinderFile = toml::from_str(text).map_err(|e| damaged(short(&e)))?;
    if file.schema > SCHEMA {
        return Err(Error::Newer { found: file.schema, supported: SCHEMA });
    }
    if file.items.len() > limits::MAX_ITEMS {
        return Err(Error::TooLarge { what: "binder.toml (items)".into(), limit: limits::MAX_ITEMS as u64 });
    }
    let mut binder = Binder::new();
    for row in file.items {
        let id = row.item.id;
        let place = match row.parent {
            None => Place::end_of(row.area),
            Some(p) => Place::inside(row.area, p),
        };
        binder.insert(place, Item { children: Vec::new(), ..row.item }).map_err(|e| damaged(format!("item {id}: {e}")))?;
    }
    if binder.max_depth() > MAX_DEPTH {
        return Err(damaged(format!("deeper than {MAX_DEPTH} levels")));
    }
    Ok(binder)
}

/// Split a file into its front matter (without fences) and body.
pub fn split_front_matter(text: &str) -> Result<(Option<&str>, &str), Error> {
    let Some(rest) = text.strip_prefix("+++\n").or_else(|| text.strip_prefix("+++\r\n")) else {
        return Ok((None, text));
    };
    let mut offset = 0;
    for line in rest.split_inclusive('\n') {
        if line.trim_end_matches(['\r', '\n']) == FENCE {
            let fm = &rest[..offset];
            if fm.len() > limits::MAX_FRONT_MATTER {
                return Err(Error::TooLarge { what: "front matter".into(), limit: limits::MAX_FRONT_MATTER as u64 });
            }
            let body = &rest[offset + line.len()..];
            // One blank line separates the front matter from the body.
            let body = body.strip_prefix('\n').or_else(|| body.strip_prefix("\r\n")).unwrap_or(body);
            return Ok((Some(fm), body));
        }
        offset += line.len();
        if offset > limits::MAX_FRONT_MATTER {
            return Err(Error::TooLarge { what: "front matter".into(), limit: limits::MAX_FRONT_MATTER as u64 });
        }
    }
    Err(Error::Damaged { file: String::new(), why: "front matter is not closed with +++".into() })
}

fn join_front_matter(fm: &str, body: &str) -> String {
    let mut out = String::with_capacity(fm.len() + body.len() + 12);
    out.push_str("+++\n");
    out.push_str(fm);
    if !fm.ends_with('\n') && !fm.is_empty() {
        out.push('\n');
    }
    out.push_str("+++\n");
    if !body.is_empty() {
        out.push('\n');
        out.push_str(body);
    }
    out
}

/// A text item's file: front matter (only when there is metadata), then the body.
pub fn write_item(text: &ItemText) -> Result<String, Error> {
    let body = dialect::write(&text.body);
    if text.meta.is_empty() {
        return Ok(body);
    }
    let fm = toml::to_string(&text.meta).map_err(|e| Error::Write(format!("item metadata: {e}")))?;
    Ok(join_front_matter(&fm, &body))
}

/// Read a text item's file.
pub fn read_item(text: &str, file: &str) -> Result<ItemText, Error> {
    let (fm, body) = split_front_matter(text).map_err(|e| e.in_file(file))?;
    let meta = match fm {
        None => ItemMeta::default(),
        Some(fm) => toml::from_str(fm).map_err(|e| Error::Damaged { file: file.into(), why: short(&e) })?,
    };
    Ok(ItemText { meta, body: dialect::parse(body) })
}

#[derive(Serialize, Deserialize, Default)]
struct EntryFront {
    kind: String,
    name: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    aliases: Vec<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    fields: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    relations: Vec<Relation>,
}

#[derive(Serialize, Deserialize)]
struct Relation {
    relation: String,
    entry: ItemId,
}

/// A bible entry's file: its kind, name, aliases, fields and relations in front matter, the body
/// below.
pub fn write_entry(e: &Entry) -> Result<String, Error> {
    let front = EntryFront {
        kind: e.kind.clone(),
        name: e.name.clone(),
        aliases: e.aliases.clone(),
        fields: e.fields.clone(),
        relations: e.relations.iter().map(|(r, id)| Relation { relation: r.clone(), entry: *id }).collect(),
    };
    let fm = toml::to_string(&front).map_err(|e| Error::Write(format!("entry: {e}")))?;
    Ok(join_front_matter(&fm, &dialect::write(&e.body)))
}

/// Read a bible entry's file.
pub fn read_entry(text: &str, file: &str) -> Result<Entry, Error> {
    let (fm, body) = split_front_matter(text).map_err(|e| e.in_file(file))?;
    let front: EntryFront = match fm {
        Some(fm) => toml::from_str(fm).map_err(|e| Error::Damaged { file: file.into(), why: short(&e) })?,
        None => EntryFront::default(),
    };
    Ok(Entry {
        kind: front.kind,
        name: front.name,
        aliases: front.aliases,
        fields: front.fields,
        relations: front.relations.into_iter().map(|r| (r.relation, r.entry)).collect(),
        body: dialect::parse(body),
    })
}

#[derive(Serialize, Deserialize)]
struct SnapshotFront {
    name: String,
}

/// A snapshot's file: its name in front matter, the text below.
pub fn write_snapshot(snap: &wordsmith_manuscript_model::Snapshot) -> Result<String, Error> {
    let fm = toml::to_string(&SnapshotFront { name: snap.name.clone() }).map_err(|e| Error::Write(format!("snapshot: {e}")))?;
    Ok(join_front_matter(&fm, &dialect::write(&snap.text)))
}

/// Read a snapshot's file taken at `taken`.
pub fn read_snapshot(text: &str, taken: &str, file: &str) -> Result<wordsmith_manuscript_model::Snapshot, Error> {
    let (fm, body) = split_front_matter(text).map_err(|e| e.in_file(file))?;
    let name = match fm {
        Some(fm) => toml::from_str::<SnapshotFront>(fm).map_err(|e| Error::Damaged { file: file.into(), why: short(&e) })?.name,
        None => String::new(),
    };
    Ok(wordsmith_manuscript_model::Snapshot { taken: taken.to_owned(), name, text: dialect::parse(body) })
}

/// The path of an item's file inside the project folder, if it has one.
pub fn path_of(item: &Item) -> Option<String> {
    match item.kind {
        Kind::Text | Kind::Folder => Some(format!("text/{}.md", item.id)),
        Kind::Entry => Some(format!("bible/{}.md", item.id)),
        Kind::Media => item.file.as_ref().map(|ext| format!("media/{}.{ext}", item.id)),
        Kind::Screenplay => Some(format!("scripts/{}.wss", item.id)),
        Kind::Link => None,
    }
}

/// Every text file of a project (`project.toml`, `binder.toml`, `text/`, `bible/`), rendered, in
/// the order they are saved: text and entries first, settings, the binder last.
pub fn render(project: &Project) -> Result<Vec<(String, Vec<u8>)>, Error> {
    let mut out = Vec::new();
    for row in project.binder.rows() {
        let item = row.item;
        match item.kind {
            Kind::Text | Kind::Folder => {
                if let Some(t) = project.texts.get(&item.id)
                    && !t.is_empty()
                {
                    out.push((format!("text/{}.md", item.id), write_item(t)?.into_bytes()));
                }
            }
            Kind::Entry => {
                let e = project.entries.get(&item.id).cloned().unwrap_or_else(|| Entry::new("", &item.title));
                out.push((format!("bible/{}.md", item.id), write_entry(&e)?.into_bytes()));
            }
            Kind::Media | Kind::Link | Kind::Screenplay => {}
        }
    }
    for (id, snaps) in &project.snapshots {
        if !project.binder.contains(*id) {
            continue;
        }
        for snap in snaps {
            if wordsmith_manuscript_model::snapshot::is_stamp(&snap.taken) {
                out.push((format!("snapshots/{id}/{}.md", snap.taken), write_snapshot(snap)?.into_bytes()));
            }
        }
    }
    out.push(("project.toml".into(), write_project(&project.settings)?.into_bytes()));
    out.push(("binder.toml".into(), write_binder(&project.binder)?.into_bytes()));
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use wordsmith_manuscript_model::{Role, Text};

    #[test]
    fn project_settings_round_trip() {
        let mut s = Settings::default();
        s.book.title = "The River".into();
        s.book.author = "A. Writer".into();
        s.targets.manuscript = Some(90_000);
        let text = write_project(&s).unwrap();
        assert!(text.starts_with("schema = 2\n"));
        assert_eq!(read_project(&text).unwrap(), s);
        assert_eq!(write_project(&read_project(&text).unwrap()).unwrap(), text);
    }

    #[test]
    fn a_newer_schema_is_refused_clearly() {
        let text = write_project(&Settings::default()).unwrap().replacen("schema = 2", "schema = 3", 1);
        let err = read_project(&text).unwrap_err();
        assert_eq!(err, Error::Newer { found: 3, supported: 2 });
        assert!(err.to_string().contains("newer version"));
        assert_eq!(read_project("title = 'x'\n").unwrap_err(), Error::NotAProject);
    }

    #[test]
    fn binder_round_trips_in_order() {
        let mut b = Binder::new();
        let ch = Item::with_id(ItemId::from_counter(1), Kind::Folder, "Chapter One").role(Role::Chapter);
        let mut sc = Item::with_id(ItemId::from_counter(2), Kind::Text, "Scene").role(Role::Scene);
        sc.label = Some("Idea".into());
        sc.include = false;
        b.insert(Place::end_of(Area::Manuscript), ch.with_children(vec![sc])).unwrap();
        b.insert(Place::end_of(Area::Notes), Item::with_id(ItemId::from_counter(3), Kind::Text, "Plan")).unwrap();
        let text = write_binder(&b).unwrap();
        assert_eq!(read_binder(&text).unwrap(), b);
        assert_eq!(write_binder(&read_binder(&text).unwrap()).unwrap(), text);
    }

    #[test]
    fn a_child_before_its_parent_is_damage() {
        let text = format!(
            "schema = 1\n[[item]]\narea = \"manuscript\"\nparent = \"{}\"\nid = \"{}\"\ntitle = \"x\"\n",
            ItemId::from_counter(9),
            ItemId::from_counter(1)
        );
        assert!(matches!(read_binder(&text), Err(Error::Damaged { .. })));
        let dup =
            format!("schema = 1\n[[item]]\narea = \"notes\"\nid = \"{0}\"\n[[item]]\narea = \"notes\"\nid = \"{0}\"\n", ItemId::from_counter(1));
        assert!(matches!(read_binder(&dup), Err(Error::Damaged { .. })));
    }

    #[test]
    fn item_files_have_front_matter_only_when_needed() {
        let plain = ItemText::with_body(Text::from_paragraphs(&["One."]));
        assert_eq!(write_item(&plain).unwrap(), "One.\n");
        let mut t = plain.clone();
        t.meta.synopsis = "Arrival at the house.".into();
        t.meta.keywords = vec!["night".into()];
        let text = write_item(&t).unwrap();
        assert_eq!(text, "+++\nsynopsis = \"Arrival at the house.\"\nkeywords = [\"night\"]\n+++\n\nOne.\n");
        assert_eq!(read_item(&text, "x").unwrap(), t);
        let meta_only = ItemText { meta: t.meta.clone(), body: Text::new() };
        let text = write_item(&meta_only).unwrap();
        assert_eq!(read_item(&text, "x").unwrap(), meta_only);
    }

    #[test]
    fn unclosed_front_matter_is_damage() {
        assert!(read_item("+++\nsynopsis = 'x'\n\nbody\n", "f").is_err());
    }

    #[test]
    fn entries_round_trip() {
        let mut e = Entry::new("character", "Anna Field");
        e.aliases = vec!["Annie".into()];
        e.fields.insert("Age".into(), "34".into());
        e.relations.push(("sister of".into(), ItemId::from_counter(4)));
        e.body = Text::from_paragraphs(&["Grew up by the river."]);
        let text = write_entry(&e).unwrap();
        assert_eq!(read_entry(&text, "x").unwrap(), e);
        assert_eq!(write_entry(&read_entry(&text, "x").unwrap()).unwrap(), text);
    }
}
