// SPDX-License-Identifier: MIT OR Apache-2.0
//! Manuscript's commands: one list, used by the ribbon, the keyboard, the command palette, the
//! command line and agents. Each takes JSON parameters and returns JSON.

use serde_json::{Value, json};
use wordsmith_manuscript_model::{Area, Item, ItemId, Kind, Place, Project, Role};

use crate::session::{Outcome, Session};

/// A command's description.
#[derive(Clone, Copy, Debug)]
pub struct Spec {
    /// The id (`binder.new`).
    pub id: &'static str,
    /// The label in menus and the palette.
    pub label: &'static str,
    /// The default key (`Mod+N`), or empty.
    pub key: &'static str,
    /// The parameters, as a short JSON-like sketch for agents and the command line.
    pub params: &'static str,
    /// What it does, in one line.
    pub about: &'static str,
    /// Whether it changes the project.
    pub mutates: bool,
}

const fn spec(id: &'static str, label: &'static str, key: &'static str, params: &'static str, about: &'static str, mutates: bool) -> Spec {
    Spec { id, label, key, params, about, mutates }
}

/// Every Manuscript command.
pub fn specs() -> Vec<Spec> {
    let mut v = vec![
        spec(
            "binder.new",
            "New Document",
            "Mod+Alt+N",
            r#"{"kind"?: "text"|"folder", "title"?: string, "after"?: id, "inside"?: id, "area"?: area, "role"?: role}"#,
            "Add a document (or folder) after the selection, inside an item, or at the end of an area.",
            true,
        ),
        spec(
            "binder.newFolder",
            "New Folder",
            "Mod+Alt+Shift+N",
            r#"{"title"?: string, "after"?: id, "inside"?: id, "area"?: area}"#,
            "Add a folder.",
            true,
        ),
        spec("binder.rename", "Rename", "F2", r#"{"id": id, "title": string}"#, "Change an item's title.", true),
        spec(
            "binder.move",
            "Move",
            "",
            r#"{"id": id, "area": area, "parent"?: id, "index"?: n}"#,
            "Move an item (with its children); the index is where it is dropped among the new siblings.",
            true,
        ),
        spec("binder.moveUp", "Move Up", "Mod+Alt+Up", r#"{"id": id}"#, "Move an item before its previous sibling.", true),
        spec("binder.moveDown", "Move Down", "Mod+Alt+Down", r#"{"id": id}"#, "Move an item after its next sibling.", true),
        spec("binder.indent", "Move Into Previous", "Mod+Alt+Right", r#"{"id": id}"#, "Make an item the last child of the item before it.", true),
        spec("binder.outdent", "Move Out", "Mod+Alt+Left", r#"{"id": id}"#, "Make an item the next sibling of its parent.", true),
        spec(
            "binder.setRole",
            "Set Role",
            "",
            r#"{"id": id, "role": "none"|"part"|"chapter"|"scene"|"front-matter"|"back-matter"}"#,
            "Set an item's role.",
            true,
        ),
        spec("binder.setInclude", "Include in Export", "", r#"{"id": id, "include": bool}"#, "Include an item in exports, or leave it out.", true),
        spec("binder.trash", "Move to Trash", "Mod+Shift+Backspace", r#"{"id": id}"#, "Move an item and its children to the Trash.", true),
        spec("binder.restore", "Restore from Trash", "", r#"{"id": id}"#, "Put an item from the Trash back where it came from.", true),
        spec(
            "binder.emptyTrash",
            "Empty Trash",
            "",
            r#"{"confirm": true}"#,
            "Remove everything in the Trash for good (asks first in the app).",
            true,
        ),
        spec(
            "settings.typing",
            "Smart Typing",
            "",
            r#"{"quotes"?: bool, "dashes"?: bool, "ellipsis"?: bool, "language"?: string}"#,
            "Switch the smart typing rules on or off, and set the project's language (whose quotation marks are used).",
            true,
        ),
        spec("binder.list", "List Binder", "", r#"{"area"?: area}"#, "The binder's items: id, title, kind, role, depth.", false),
        spec("edit.undo", "Undo", "Mod+Z", "{}", "Undo the last change.", true),
        spec("edit.redo", "Redo", "Mod+Shift+Z", "{}", "Redo the last undone change.", true),
    ];
    v.extend(crate::inspector::specs());
    v.extend(crate::texts::specs());
    v.extend(crate::find::specs());
    v.extend(crate::spelling::specs());
    v.extend(crate::snapshots::specs());
    v.extend(crate::comments::specs());
    v.extend(crate::fields::specs());
    v.extend(crate::collections::specs());
    v.extend(crate::bookmarks::specs());
    v.extend(crate::media::specs());
    v.extend(crate::board::specs());
    v.extend(crate::pages::specs());
    v.extend(crate::revisions::specs());
    v.extend(crate::entry_types::specs());
    v.extend(crate::offers::specs());
    v.extend(crate::drafts::specs());
    v.extend(crate::series::specs());
    v.extend(crate::relations::specs());
    v.extend(crate::lanes::specs());
    v.extend(crate::outline::specs());
    v.extend(crate::bible::specs());
    v.extend(crate::imports::specs());
    v.extend(crate::exports::specs());
    v.extend(crate::progress::specs());
    v.extend(crate::search::specs());
    v
}

/// A command's spec by id.
pub fn find(id: &str) -> Option<Spec> {
    specs().into_iter().find(|s| s.id == id)
}

/// A required item id parameter.
pub fn id_param(v: &Value, key: &str) -> Result<ItemId, String> {
    v.get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("`{key}` (an item id) is required"))?
        .parse()
        .map_err(|e: wordsmith_manuscript_model::BadId| e.to_string())
}

/// An optional item id parameter.
pub fn opt_id(v: &Value, key: &str) -> Result<Option<ItemId>, String> {
    match v.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(_) => id_param(v, key).map(Some),
    }
}

/// A required string parameter.
pub fn str_param<'a>(v: &'a Value, key: &str) -> Result<&'a str, String> {
    v.get(key).and_then(Value::as_str).ok_or_else(|| format!("`{key}` (text) is required"))
}

/// An optional string parameter.
pub fn opt_str<'a>(v: &'a Value, key: &str) -> Option<&'a str> {
    v.get(key).and_then(Value::as_str)
}

/// A required bool parameter.
pub fn bool_param(v: &Value, key: &str) -> Result<bool, String> {
    v.get(key).and_then(Value::as_bool).ok_or_else(|| format!("`{key}` (true or false) is required"))
}

fn area_param(v: &Value, key: &str) -> Result<Option<Area>, String> {
    match v.get(key).and_then(Value::as_str) {
        None => Ok(None),
        Some(a) => Area::from_key(a).map(Some).ok_or_else(|| format!("`{a}` is not an area (manuscript, bible, research, notes, trash)")),
    }
}

fn exists(p: &Project, id: ItemId) -> Result<&Item, String> {
    p.binder.get(id).ok_or_else(|| format!("no binder item {id}"))
}

/// The role a new item gets under a parent (SPEC 1.5).
pub fn child_role(parent: Option<Role>) -> Role {
    match parent {
        Some(Role::Part) => Role::Chapter,
        Some(Role::Chapter) => Role::Scene,
        Some(Role::Scene) => Role::Scene,
        _ => Role::None,
    }
}

/// Where a new item goes and the role it takes, from `after` / `inside` / `area`.
fn new_place(p: &Project, v: &Value) -> Result<(Place, Role), String> {
    if let Some(after) = opt_id(v, "after")? {
        let place = p.binder.place_of(after).ok_or_else(|| format!("no binder item {after}"))?;
        let role = match place.parent {
            Some(parent) => child_role(p.binder.get(parent).map(|i| i.role)),
            None => p.binder.get(after).map_or(Role::None, |i| i.role),
        };
        let role = if place.area == Area::Manuscript { role } else { Role::None };
        return Ok((Place { index: place.index + 1, ..place }, role));
    }
    if let Some(inside) = opt_id(v, "inside")? {
        let area = p.binder.area_of(inside).ok_or_else(|| format!("no binder item {inside}"))?;
        let role = if area == Area::Manuscript { child_role(p.binder.get(inside).map(|i| i.role)) } else { Role::None };
        return Ok((Place::inside(area, inside), role));
    }
    let area = area_param(v, "area")?.unwrap_or(Area::Manuscript);
    Ok((Place::end_of(area), Role::None))
}

fn new_item(s: &mut Session, v: &Value, kind: Kind) -> Outcome {
    let (place, role) = new_place(&s.project, v)?;
    let role = match opt_str(v, "role") {
        Some(r) => Role::from_key(r).ok_or_else(|| format!("`{r}` is not a role"))?,
        None => role,
    };
    let title = opt_str(v, "title").map(str::to_owned).unwrap_or_else(|| if kind == Kind::Folder { "New Folder".into() } else { "Untitled".into() });
    let kind = match opt_str(v, "kind") {
        Some("folder") => Kind::Folder,
        Some("text") | None => kind,
        Some(other) => return Err(format!("`{other}` is not a kind this command makes (text, folder)")),
    };
    let item = Item::new(kind, title).role(role);
    let id = item.id;
    s.change("New Item", &[], &[], |p| p.binder.insert(place, item).map_err(|e| e.to_string()))?;
    Ok(json!({ "id": id.as_text() }))
}

fn sibling_move(s: &mut Session, v: &Value, delta: i64) -> Outcome {
    let id = id_param(v, "id")?;
    let place = s.project.binder.place_of(id).ok_or_else(|| format!("no binder item {id}"))?;
    let index = match delta {
        -1 if place.index == 0 => return Ok(json!({ "moved": false })),
        -1 => place.index - 1,
        _ => place.index + 2,
    };
    let label = if delta < 0 { "Move Up" } else { "Move Down" };
    s.change(label, &[], &[], |p| p.binder.move_to(id, Place { index, ..place }).map_err(|e| e.to_string()))?;
    Ok(json!({ "moved": true }))
}

fn rows_json(p: &Project, area: Option<Area>) -> Value {
    let rows = match area {
        Some(a) => p.binder.rows_of(a),
        None => p.binder.rows(),
    };
    Value::Array(
        rows.iter()
            .map(|r| {
                json!({
                    "id": r.item.id.as_text(),
                    "title": r.item.title,
                    "kind": format!("{:?}", r.item.kind).to_lowercase(),
                    "role": r.item.role.key(),
                    "area": r.area.key(),
                    "depth": r.depth,
                    "include": r.item.include,
                    "words": p.item_words(r.item.id),
                })
            })
            .collect(),
    )
}

/// Run a command on a session.
pub fn run(s: &mut Session, id: &str, v: &Value) -> Outcome {
    match id {
        "binder.new" => new_item(s, v, Kind::Text),
        "binder.newFolder" => new_item(s, v, Kind::Folder),
        "binder.rename" => {
            let id = id_param(v, "id")?;
            let title = str_param(v, "title")?.to_owned();
            exists(&s.project, id)?;
            s.change("Rename", &[], &[id], |p| {
                // An entry's name is its title in the binder.
                if let Some(e) = p.entries.get_mut(&id) {
                    e.name.clone_from(&title);
                }
                if let Some(i) = p.binder.get_mut(id) {
                    i.title = title;
                }
                Ok(())
            })?;
            Ok(json!({}))
        }
        "binder.move" => {
            let id = id_param(v, "id")?;
            let area = area_param(v, "area")?.ok_or("`area` is required")?;
            let parent = opt_id(v, "parent")?;
            let index = v.get("index").and_then(Value::as_u64).map_or(usize::MAX, |n| usize::try_from(n).unwrap_or(usize::MAX));
            let placed = s.change("Move", &[], &[], |p| p.binder.move_to(id, Place { area, parent, index }).map_err(|e| e.to_string()))?;
            Ok(json!({ "index": placed.index }))
        }
        "binder.moveUp" => sibling_move(s, v, -1),
        "binder.moveDown" => sibling_move(s, v, 1),
        "binder.indent" => {
            let id = id_param(v, "id")?;
            let place = s.project.binder.place_of(id).ok_or_else(|| format!("no binder item {id}"))?;
            if place.index == 0 {
                return Err("there is no item before this one to move into".into());
            }
            let siblings: Vec<ItemId> = match place.parent {
                Some(p) => exists(&s.project, p)?.children.iter().map(|c| c.id).collect(),
                None => s.project.binder.area(place.area).iter().map(|c| c.id).collect(),
            };
            let prev = *siblings.get(place.index - 1).ok_or("no previous item")?;
            s.change("Move Into Previous", &[], &[], |p| p.binder.move_to(id, Place::inside(place.area, prev)).map_err(|e| e.to_string()))?;
            Ok(json!({}))
        }
        "binder.outdent" => {
            let id = id_param(v, "id")?;
            let place = s.project.binder.place_of(id).ok_or_else(|| format!("no binder item {id}"))?;
            let Some(parent) = place.parent else { return Err("the item is already at the top of its area".into()) };
            let pp = s.project.binder.place_of(parent).ok_or("no parent")?;
            s.change("Move Out", &[], &[], |p| p.binder.move_to(id, Place { index: pp.index + 1, ..pp }).map_err(|e| e.to_string()))?;
            Ok(json!({}))
        }
        "binder.setRole" => {
            let id = id_param(v, "id")?;
            let r = str_param(v, "role")?;
            let role = Role::from_key(r).ok_or_else(|| format!("`{r}` is not a role"))?;
            exists(&s.project, id)?;
            s.change("Set Role", &[], &[], |p| {
                if let Some(i) = p.binder.get_mut(id) {
                    i.role = role;
                }
                Ok(())
            })?;
            Ok(json!({}))
        }
        "binder.setInclude" => {
            let id = id_param(v, "id")?;
            let include = bool_param(v, "include")?;
            exists(&s.project, id)?;
            s.change("Include in Export", &[], &[], |p| {
                if let Some(i) = p.binder.get_mut(id) {
                    i.include = include;
                }
                Ok(())
            })?;
            Ok(json!({}))
        }
        "binder.trash" => {
            let id = id_param(v, "id")?;
            s.change("Move to Trash", &[], &[], |p| p.binder.trash(id).map_err(|e| e.to_string()))?;
            Ok(json!({}))
        }
        "binder.restore" => {
            let id = id_param(v, "id")?;
            s.change("Restore", &[], &[], |p| p.binder.restore(id).map_err(|e| e.to_string()))?;
            Ok(json!({}))
        }
        "binder.emptyTrash" => {
            if !bool_param(v, "confirm").unwrap_or(false) {
                return Err("emptying the Trash removes its items for good: pass {\"confirm\": true}".into());
            }
            let gone: Vec<ItemId> = s.project.binder.rows_of(Area::Trash).iter().map(|r| r.item.id).collect();
            let words: usize = gone.iter().map(|i| s.project.item_words(*i)).sum();
            let n = gone.len();
            s.change("Empty Trash", &gone, &gone, |p| {
                p.binder.empty_trash();
                p.prune();
                Ok(())
            })?;
            Ok(json!({ "removed": n, "words": words }))
        }
        "binder.list" => Ok(rows_json(&s.project, area_param(v, "area")?)),
        "settings.typing" => {
            let mut typing = s.project.settings.extra.get("typing").cloned().unwrap_or_else(|| json!({}));
            for k in ["quotes", "dashes", "ellipsis"] {
                if let Some(b) = v.get(k).and_then(Value::as_bool) {
                    typing[k] = json!(b);
                }
            }
            let lang = opt_str(v, "language").map(str::trim).map(str::to_owned);
            if let Some(l) = &lang
                && (l.is_empty() || !l.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'))
            {
                return Err(format!("`{l}` is not a language tag (such as en, fr or de-CH)"));
            }
            s.change("Smart Typing", &[], &[], |p| {
                p.settings.extra.insert("typing".into(), typing);
                if let Some(l) = lang {
                    p.settings.book.language = l;
                }
                Ok(())
            })?;
            Ok(json!({}))
        }
        "edit.undo" => Ok(json!({ "undone": s.undo() })),
        "edit.redo" => Ok(json!({ "redone": s.redo() })),
        other => {
            if let Some(out) = crate::inspector::run(s, other, v) {
                return out;
            }
            if let Some(out) = crate::texts::run(s, other, v) {
                return out;
            }
            if let Some(out) = crate::find::run(s, other, v) {
                return out;
            }
            if let Some(out) = crate::spelling::run(s, other, v) {
                return out;
            }
            if let Some(out) = crate::snapshots::run(s, other, v) {
                return out;
            }
            if let Some(out) = crate::comments::run(s, other, v) {
                return out;
            }
            if let Some(out) = crate::fields::run(s, other, v) {
                return out;
            }
            if let Some(out) = crate::collections::run(s, other, v) {
                return out;
            }
            if let Some(out) = crate::bookmarks::run(s, other, v) {
                return out;
            }
            if let Some(out) = crate::media::run(s, other, v) {
                return out;
            }
            if let Some(out) = crate::board::run(s, other, v) {
                return out;
            }
            if let Some(out) = crate::pages::run(s, other, v) {
                return out;
            }
            if let Some(out) = crate::revisions::run(s, other, v) {
                return out;
            }
            if let Some(out) = crate::entry_types::run(s, other, v) {
                return out;
            }
            if let Some(out) = crate::offers::run(s, other, v) {
                return out;
            }
            if let Some(out) = crate::drafts::run(s, other, v) {
                return out;
            }
            if let Some(out) = crate::series::run(s, other, v) {
                return out;
            }
            if let Some(out) = crate::relations::run(s, other, v) {
                return out;
            }
            if let Some(out) = crate::lanes::run(s, other, v) {
                return out;
            }
            if let Some(out) = crate::outline::run(s, other, v) {
                return out;
            }
            if let Some(out) = crate::bible::run(s, other, v) {
                return out;
            }
            if let Some(out) = crate::imports::run(s, other, v) {
                return out;
            }
            if let Some(out) = crate::exports::run(s, other, v) {
                return out;
            }
            if let Some(out) = crate::progress::run(s, other, v) {
                return out;
            }
            if let Some(out) = crate::search::run(s, other, v) {
                return out;
            }
            Err(format!("unknown command {other}"))
        }
    }
}
