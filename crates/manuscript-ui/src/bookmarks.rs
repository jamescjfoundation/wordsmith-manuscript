// SPDX-License-Identifier: MIT OR Apache-2.0
//! Bookmarks (BND-7): quick links to items, shown above the binder and reachable from anywhere
//! (the binder, the command palette, agents). Kept in the project settings
//! (`[extra] bookmarks`, item ids in order).

use serde_json::{Value, json};
use wordsmith_manuscript_model::{Area, ItemId, Project};

use crate::commands::{Spec, id_param};
use crate::session::{Outcome, Session};

/// The settings key.
pub const KEY: &str = "bookmarks";

/// The bookmarked items still in the project (not in the Trash), in order.
pub fn all(p: &Project) -> Vec<ItemId> {
    stored(p).into_iter().filter(|i| p.binder.contains(*i) && p.binder.area_of(*i) != Some(Area::Trash)).collect()
}

fn stored(p: &Project) -> Vec<ItemId> {
    p.settings.extra.get(KEY).and_then(|v| serde_json::from_value(v.clone()).ok()).unwrap_or_default()
}

/// The bookmark commands.
pub fn specs() -> Vec<Spec> {
    vec![
        Spec {
            id: "bookmark.toggle",
            label: "Bookmark",
            key: "Mod+D",
            params: r#"{"id": id}"#,
            about: "Bookmark an item, or remove its bookmark.",
            mutates: true,
        },
        Spec { id: "bookmark.list", label: "List Bookmarks", key: "", params: "{}", about: "The bookmarked items: id and title.", mutates: false },
    ]
}

/// Run a bookmark command, or `None` if `id` is not one.
pub fn run(s: &mut Session, id: &str, v: &Value) -> Option<Outcome> {
    Some(match id {
        "bookmark.toggle" => (|| {
            let item = id_param(v, "id")?;
            if !s.project.binder.contains(item) {
                return Err(format!("no binder item {item}"));
            }
            let mut list = stored(&s.project);
            let on = !list.contains(&item);
            if on {
                list.push(item);
            } else {
                list.retain(|i| *i != item);
            }
            s.change(if on { "Bookmark" } else { "Remove Bookmark" }, &[], &[], |p| {
                if list.is_empty() {
                    p.settings.extra.remove(KEY);
                } else {
                    p.settings.extra.insert(KEY.into(), json!(list.iter().map(|i| i.as_text()).collect::<Vec<_>>()));
                }
                Ok(())
            })?;
            Ok(json!({ "bookmarked": on }))
        })(),
        "bookmark.list" => Ok(Value::Array(
            all(&s.project)
                .iter()
                .map(|i| json!({ "id": i.as_text(), "title": s.project.binder.get(*i).map(|x| x.title.clone()).unwrap_or_default() }))
                .collect(),
        )),
        _ => return None,
    })
}
