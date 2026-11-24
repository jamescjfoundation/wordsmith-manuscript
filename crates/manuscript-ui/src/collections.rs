// SPDX-License-Identifier: MIT OR Apache-2.0
//! Collections (SRC-2): named lists of items beside the binder. A hand-picked collection holds
//! the items put in it, in that order; a saved search holds whatever matches its query now, in
//! binder order. Items stay where they are in the binder; a collection only points at them.
//!
//! Kept in the project settings (`[extra] collections`), so they travel with the project.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use wordsmith_manuscript_model::{Area, ItemId, Project};

use crate::commands::{Spec, id_param, str_param};
use crate::find::Query;
use crate::search::Searcher;
use crate::session::{Outcome, Session};

/// The settings key.
pub const KEY: &str = "collections";

/// A saved search's query.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SavedQuery {
    /// The text or pattern.
    pub query: String,
    /// Match case.
    pub case: bool,
    /// Whole words only.
    pub word: bool,
    /// A regular expression.
    pub regex: bool,
}

/// One collection.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Collection {
    /// Its name.
    pub name: String,
    /// The items of a hand-picked collection, in order.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub items: Vec<ItemId>,
    /// The query of a saved search.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search: Option<SavedQuery>,
}

/// The project's collections, in order.
pub fn all(p: &Project) -> Vec<Collection> {
    p.settings.extra.get(KEY).and_then(|v| serde_json::from_value(v.clone()).ok()).unwrap_or_default()
}

fn store(p: &mut Project, list: &[Collection]) {
    if list.is_empty() {
        p.settings.extra.remove(KEY);
    } else if let Ok(v) = serde_json::to_value(list) {
        p.settings.extra.insert(KEY.into(), v);
    }
}

/// The items a collection shows now: a hand-picked one's items still in the project (not in the
/// Trash), a saved search's matches in binder order.
pub fn items_of(p: &Project, c: &Collection) -> Result<Vec<ItemId>, String> {
    match &c.search {
        Some(q) => {
            let query = Query { text: q.query.clone(), case: q.case, word: q.word, regex: q.regex };
            let found = Searcher::default().search(p, u64::MAX, &query)?;
            let mut out: Vec<ItemId> = Vec::new();
            for f in found {
                if !out.contains(&f.item) && p.binder.area_of(f.item) != Some(Area::Trash) {
                    out.push(f.item);
                }
            }
            Ok(out)
        }
        None => Ok(c.items.iter().copied().filter(|i| p.binder.contains(*i) && p.binder.area_of(*i) != Some(Area::Trash)).collect()),
    }
}

/// The collection commands.
pub fn specs() -> Vec<Spec> {
    let spec = |id, label, params, about, mutates| Spec { id, label, key: "", params, about, mutates };
    vec![
        spec("collection.new", "New Collection", r#"{"name": string, "items"?: [id]}"#, "A hand-picked collection, empty or with these items.", true),
        spec(
            "collection.saveSearch",
            "Save Search",
            r#"{"name": string, "query": string, "case"?: bool, "word"?: bool, "regex"?: bool}"#,
            "A collection holding whatever matches the query, kept up to date.",
            true,
        ),
        spec(
            "collection.add",
            "Add to Collection",
            r#"{"name": string, "items": [id]}"#,
            "Add items to a hand-picked collection (at its end).",
            true,
        ),
        spec(
            "collection.remove",
            "Remove from Collection",
            r#"{"name": string, "items": [id]}"#,
            "Take items out of a hand-picked collection (they stay in the binder).",
            true,
        ),
        spec("collection.rename", "Rename Collection", r#"{"name": string, "to": string}"#, "Give a collection another name.", true),
        spec("collection.delete", "Delete Collection", r#"{"name": string}"#, "Delete a collection (its items stay in the binder).", true),
        spec("collection.list", "List Collections", "{}", "The collections: name, kind and how many items each shows now.", false),
        spec("collection.items", "Collection Items", r#"{"name": string}"#, "The items a collection shows now: id and title.", false),
    ]
}

fn ids(v: &Value, p: &Project) -> Result<Vec<ItemId>, String> {
    let list = v.get("items").and_then(Value::as_array).ok_or("`items` (a list of ids) is required")?;
    let mut out = Vec::new();
    for x in list {
        let id = id_param(&json!({ "id": x }), "id")?;
        if !p.binder.contains(id) {
            return Err(format!("no binder item {id}"));
        }
        if !out.contains(&id) {
            out.push(id);
        }
    }
    Ok(out)
}

fn edit(s: &mut Session, label: &str, f: impl FnOnce(&mut Vec<Collection>, &Project) -> Result<(), String>) -> Outcome {
    let mut list = all(&s.project);
    f(&mut list, &s.project)?;
    s.change(label, &[], &[], |p| {
        store(p, &list);
        Ok(())
    })?;
    Ok(json!({}))
}

fn named<'a>(list: &'a mut [Collection], name: &str) -> Result<&'a mut Collection, String> {
    list.iter_mut().find(|c| c.name == name).ok_or_else(|| format!("no collection named `{name}`"))
}

fn new_name(list: &[Collection], name: &str) -> Result<String, String> {
    let n = name.trim();
    if n.is_empty() {
        return Err("a collection needs a name".into());
    }
    if list.iter().any(|c| c.name == n) {
        return Err(format!("there is already a collection named `{n}`"));
    }
    Ok(n.to_owned())
}

/// Run a collection command, or `None` if `id` is not one.
pub fn run(s: &mut Session, id: &str, v: &Value) -> Option<Outcome> {
    Some(match id {
        "collection.new" => (|| {
            let items = if v.get("items").is_some() { ids(v, &s.project)? } else { Vec::new() };
            let name = str_param(v, "name")?.to_owned();
            edit(s, "New Collection", |list, _| {
                let name = new_name(list, &name)?;
                list.push(Collection { name, items, search: None });
                Ok(())
            })
        })(),
        "collection.saveSearch" => (|| {
            let q = Query::from_params(v)?;
            q.matcher()?;
            let name = str_param(v, "name")?.to_owned();
            edit(s, "Save Search", |list, _| {
                let name = new_name(list, &name)?;
                list.push(Collection {
                    name,
                    items: Vec::new(),
                    search: Some(SavedQuery { query: q.text, case: q.case, word: q.word, regex: q.regex }),
                });
                Ok(())
            })
        })(),
        "collection.add" | "collection.remove" => (|| {
            let items = ids(v, &s.project)?;
            let name = str_param(v, "name")?.to_owned();
            let adding = id == "collection.add";
            edit(s, if adding { "Add to Collection" } else { "Remove from Collection" }, |list, _| {
                let c = named(list, &name)?;
                if c.search.is_some() {
                    return Err(format!("`{name}` is a saved search: what it holds follows its query"));
                }
                if adding {
                    for i in items {
                        if !c.items.contains(&i) {
                            c.items.push(i);
                        }
                    }
                } else {
                    c.items.retain(|i| !items.contains(i));
                }
                Ok(())
            })
        })(),
        "collection.rename" => (|| {
            let name = str_param(v, "name")?.to_owned();
            let to = str_param(v, "to")?.to_owned();
            edit(s, "Rename Collection", |list, _| {
                let to = if to.trim() == name { name.clone() } else { new_name(list, &to)? };
                named(list, &name)?.name = to;
                Ok(())
            })
        })(),
        "collection.delete" => (|| {
            let name = str_param(v, "name")?.to_owned();
            edit(s, "Delete Collection", |list, _| {
                let before = list.len();
                list.retain(|c| c.name != name);
                if list.len() == before { Err(format!("no collection named `{name}`")) } else { Ok(()) }
            })
        })(),
        "collection.list" => (|| {
            let rows: Vec<Value> = all(&s.project)
                .iter()
                .map(|c| {
                    json!({
                        "name": c.name,
                        "kind": if c.search.is_some() { "search" } else { "list" },
                        "query": c.search.as_ref().map(|q| q.query.clone()),
                        "count": items_of(&s.project, c).map(|i| i.len()).unwrap_or(0),
                    })
                })
                .collect();
            Ok(Value::Array(rows))
        })(),
        "collection.items" => (|| {
            let name = str_param(v, "name")?;
            let c = all(&s.project).into_iter().find(|c| c.name == name).ok_or_else(|| format!("no collection named `{name}`"))?;
            let items = items_of(&s.project, &c)?;
            Ok(Value::Array(
                items
                    .iter()
                    .map(|i| json!({ "id": i.as_text(), "title": s.project.binder.get(*i).map(|x| x.title.clone()).unwrap_or_default() }))
                    .collect(),
            ))
        })(),
        _ => return None,
    })
}
