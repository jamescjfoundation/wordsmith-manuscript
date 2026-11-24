// SPDX-License-Identifier: MIT OR Apache-2.0
//! Drafts (DRF-1, decision 0006): several drafts of the manuscript in one project.
//!
//! The Manuscript area is always the **working draft**, the one being written. Starting a new
//! draft keeps the working draft as it is now as a stored draft, and writing carries on in the
//! Manuscript area under the new draft's name. A stored draft holds its own manuscript tree and
//! texts; its items keep the ids they had, so the same scene can be followed from draft to draft
//! and compared scene by scene. Any draft can become the working one again, and any can be the
//! one that exports.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::diff::{self, Diff};
use crate::{Area, Binder, Item, ItemId, ItemText, Place, Project};

/// The most drafts a project keeps.
pub const MAX_DRAFTS: usize = 100;

/// A stored draft's name and when it was kept (in the project settings).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct DraftInfo {
    /// Its id (its folder's name: `drafts/<id>/`).
    pub id: String,
    /// Its name ("First Draft").
    pub name: String,
    /// When it was kept, as a UTC stamp.
    pub created: String,
}

/// A stored draft's manuscript: its tree (the Manuscript area only) and texts.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DraftBody {
    /// The manuscript tree.
    pub binder: Binder,
    /// The texts of its items.
    pub texts: BTreeMap<ItemId, ItemText>,
}

impl DraftBody {
    /// Its words (the included items).
    pub fn words(&self) -> usize {
        let mut p = Project::new();
        p.binder = self.binder.clone();
        p.texts = self.texts.clone();
        p.manuscript_words()
    }
}

/// The working draft's name.
pub fn working_name(p: &Project) -> String {
    if p.settings.draft_name.is_empty() { "First Draft".into() } else { p.settings.draft_name.clone() }
}

/// The working draft as a draft body (a copy).
pub fn working(p: &Project) -> DraftBody {
    let mut binder = Binder::new();
    let mut texts = BTreeMap::new();
    for item in p.binder.area(Area::Manuscript) {
        let _ = binder.insert(Place::end_of(Area::Manuscript), item.clone());
        for i in item.walk() {
            if let Some(t) = p.texts.get(&i.id) {
                texts.insert(i.id, t.clone());
            }
        }
    }
    DraftBody { binder, texts }
}

/// Put a draft body into the Manuscript area, in place of what is there (whose texts leave the
/// project's texts).
fn install(p: &mut Project, body: DraftBody) {
    let old: Vec<ItemId> = p.binder.rows_of(Area::Manuscript).iter().map(|r| r.item.id).collect();
    let tops: Vec<ItemId> = p.binder.area(Area::Manuscript).iter().map(|i| i.id).collect();
    for id in tops {
        let _ = p.binder.remove(id);
    }
    for id in old {
        p.texts.remove(&id);
    }
    // An item of the draft whose id the rest of the binder still uses (the same scene, moved to
    // the Trash in the other draft) takes a new id, with its text, so nothing is dropped.
    let elsewhere: std::collections::HashSet<ItemId> = p.binder.rows().iter().map(|r| r.item.id).collect();
    let mut texts = body.texts;
    fn renew(item: &mut Item, elsewhere: &std::collections::HashSet<ItemId>, texts: &mut BTreeMap<ItemId, ItemText>) {
        if elsewhere.contains(&item.id) {
            let fresh = ItemId::new();
            if let Some(t) = texts.remove(&item.id) {
                texts.insert(fresh, t);
            }
            item.id = fresh;
        }
        for c in &mut item.children {
            renew(c, elsewhere, texts);
        }
    }
    for mut item in body.binder.area(Area::Manuscript).iter().cloned() {
        renew(&mut item, &elsewhere, &mut texts);
        let _ = p.binder.insert(Place::end_of(Area::Manuscript), item);
    }
    p.texts.extend(texts);
}

/// Keep the working draft as it is now under its name, and carry on writing it as `new_name`.
/// `id` names the stored draft's folder; `created` is a UTC stamp. Returns the stored draft's id.
pub fn start_new(p: &mut Project, new_name: &str, id: &str, created: &str) -> Result<String, String> {
    let new_name = new_name.trim();
    if new_name.is_empty() {
        return Err("a draft needs a name".into());
    }
    if p.settings.drafts.len() >= MAX_DRAFTS {
        return Err(format!("a project keeps at most {MAX_DRAFTS} drafts"));
    }
    if p.settings.drafts.iter().any(|d| d.name == new_name) || working_name(p) == new_name {
        return Err(format!("there is already a draft named “{new_name}”"));
    }
    let info = DraftInfo { id: id.to_owned(), name: working_name(p), created: created.to_owned() };
    p.drafts.insert(id.to_owned(), working(p));
    p.settings.drafts.push(info);
    p.settings.draft_name = new_name.to_owned();
    Ok(id.to_owned())
}

/// Make a stored draft the working one; the working draft is stored in its place (same id).
pub fn switch(p: &mut Project, id: &str) -> Result<(), String> {
    let at = p.settings.drafts.iter().position(|d| d.id == id).ok_or_else(|| format!("no draft {id}"))?;
    let body = p.drafts.remove(id).unwrap_or_default();
    let current = working(p);
    let current_name = working_name(p);
    install(p, body);
    p.drafts.insert(id.to_owned(), current);
    let info = &mut p.settings.drafts[at];
    p.settings.draft_name = std::mem::replace(&mut info.name, current_name);
    Ok(())
}

/// Remove a stored draft for good.
pub fn delete(p: &mut Project, id: &str) -> Result<(), String> {
    let before = p.settings.drafts.len();
    p.settings.drafts.retain(|d| d.id != id);
    if p.settings.drafts.len() == before {
        return Err(format!("no draft {id}"));
    }
    p.drafts.remove(id);
    if p.settings.export_draft == id {
        p.settings.export_draft.clear();
    }
    Ok(())
}

/// The project as it would be with draft `id` in the Manuscript area (`""`: the working draft),
/// for exporting a draft that is not the working one.
pub fn with_draft(p: &Project, id: &str) -> Result<Project, String> {
    if id.is_empty() {
        return Ok(p.clone());
    }
    let body = p.drafts.get(id).cloned().ok_or_else(|| format!("no draft {id}"))?;
    let mut out = p.clone();
    install(&mut out, body);
    Ok(out)
}

/// The project as it exports: with the draft chosen for export.
pub fn exported(p: &Project) -> Project {
    with_draft(p, &p.settings.export_draft).unwrap_or_else(|_| p.clone())
}

/// How one item differs between two drafts.
#[derive(Clone, Debug, PartialEq)]
pub struct ItemChange {
    /// The item.
    pub id: ItemId,
    /// Its title (in the newer draft, or the older one when it was cut).
    pub title: String,
    /// In the older draft only (cut), the newer only (added), or both.
    pub in_old: bool,
    /// In the newer draft.
    pub in_new: bool,
    /// The texts compared word by word.
    pub diff: Diff,
}

fn tree(body: &DraftBody) -> Vec<Item> {
    body.binder.rows_of(Area::Manuscript).iter().map(|r| r.item.clone()).collect()
}

/// Compare two drafts item by item (scene by scene): the newer draft's items in its order,
/// each with the older draft's item of the same id, then the items the newer draft cut.
pub fn compare(old: &DraftBody, new: &DraftBody) -> Vec<ItemChange> {
    let old_items = tree(old);
    let new_items = tree(new);
    let mut out = Vec::new();
    let empty = ItemText::default();
    for item in &new_items {
        let was = old_items.iter().find(|i| i.id == item.id);
        let a = old.texts.get(&item.id).unwrap_or(&empty);
        let b = new.texts.get(&item.id).unwrap_or(&empty);
        out.push(ItemChange { id: item.id, title: item.title.clone(), in_old: was.is_some(), in_new: true, diff: diff::texts(&a.body, &b.body) });
    }
    for item in old_items.iter().filter(|i| !new_items.iter().any(|n| n.id == i.id)) {
        let a = old.texts.get(&item.id).unwrap_or(&empty);
        out.push(ItemChange { id: item.id, title: item.title.clone(), in_old: true, in_new: false, diff: diff::texts(&a.body, &crate::Text::new()) });
    }
    out
}

/// A draft's body by id (`""`: the working draft).
pub fn body(p: &Project, id: &str) -> Option<DraftBody> {
    if id.is_empty() { Some(working(p)) } else { p.drafts.get(id).cloned() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Kind, Role, Text};

    fn id(n: u64) -> ItemId {
        ItemId::from_counter(n)
    }

    fn project() -> Project {
        let mut p = Project::new();
        let ch = Item::with_id(id(10), Kind::Folder, "One").role(Role::Chapter).with_children(vec![
            Item::with_id(id(1), Kind::Text, "Dock").role(Role::Scene),
            Item::with_id(id(2), Kind::Text, "Inn").role(Role::Scene),
        ]);
        p.binder.insert(Place::end_of(Area::Manuscript), ch).unwrap();
        p.binder.insert(Place::end_of(Area::Notes), Item::with_id(id(50), Kind::Text, "Notes")).unwrap();
        p.texts.insert(id(1), ItemText::with_body(Text::from_paragraphs(&["The boat came in."])));
        p.texts.insert(id(2), ItemText::with_body(Text::from_paragraphs(&["The inn was full."])));
        p.texts.insert(id(50), ItemText::with_body(Text::from_paragraphs(&["A note."])));
        p
    }

    #[test]
    fn a_new_draft_keeps_the_old_one_and_writing_goes_on() {
        let mut p = project();
        start_new(&mut p, "Second Draft", "d1", "20261124T100000Z").unwrap();
        assert_eq!(p.settings.drafts[0].name, "First Draft");
        assert_eq!(working_name(&p), "Second Draft");
        p.text_mut(id(1)).body = Text::from_paragraphs(&["The boat came in late."]);
        let d1 = &p.drafts["d1"];
        assert_eq!(d1.texts[&id(1)].body, Text::from_paragraphs(&["The boat came in."]));
        assert!(!d1.texts.contains_key(&id(50)), "a draft holds the manuscript only");
        // Scene by scene.
        let changes = compare(d1, &working(&p));
        assert_eq!(changes.len(), 3, "the chapter and its two scenes");
        let dock = changes.iter().find(|c| c.id == id(1)).unwrap();
        assert_eq!((dock.diff.added, dock.diff.removed), (1, 0));
        assert!(changes.iter().find(|c| c.id == id(2)).unwrap().diff.is_same());
        // Switching puts the first draft back in the Manuscript, and keeps the second.
        switch(&mut p, "d1").unwrap();
        assert_eq!(working_name(&p), "First Draft");
        assert_eq!(p.settings.drafts[0].name, "Second Draft");
        assert_eq!(p.texts[&id(1)].body, Text::from_paragraphs(&["The boat came in."]));
        assert_eq!(p.texts[&id(50)].body, Text::from_paragraphs(&["A note."]), "the rest of the project stays");
        // Export a stored draft without switching.
        let out = with_draft(&p, "d1").unwrap();
        assert_eq!(out.texts[&id(1)].body, Text::from_paragraphs(&["The boat came in late."]));
        assert!(start_new(&mut p, "Second Draft", "d2", "x").is_err(), "names are unique");
        delete(&mut p, "d1").unwrap();
        assert!(p.drafts.is_empty() && p.settings.drafts.is_empty());
    }
}
