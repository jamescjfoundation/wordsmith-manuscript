// SPDX-License-Identifier: MIT OR Apache-2.0
//! Project templates (BND-4, SPEC 2): the binder, lists and targets a new project starts with.

use crate::{Area, Item, ItemId, ItemText, Kind, Place, Project, Role, Text};

/// A template's description.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TemplateInfo {
    /// The id (`novel`).
    pub id: &'static str,
    /// The name shown.
    pub name: &'static str,
    /// What it holds.
    pub about: &'static str,
}

/// The templates, in the order shown.
pub const TEMPLATES: [TemplateInfo; 2] = [
    TemplateInfo { id: "novel", name: "Novel", about: "Parts, chapters and scenes; a story bible; notes for the premise and synopsis." },
    TemplateInfo {
        id: "stories",
        name: "Short Story Collection",
        about: "One folder per story with its scenes; front matter with a contents page; a story bible.",
    },
];

/// The story bible's starting folders.
pub const BIBLE_FOLDERS: [&str; 6] = ["Characters", "Places", "Objects", "Groups", "Lore", "Plot Threads"];

fn add(p: &mut Project, place: Place, item: Item) -> ItemId {
    let id = item.id;
    // A template builds a fresh tree, so an insert cannot fail; if it ever did, the item is left
    // out rather than the project.
    let _ = p.binder.insert(place, item);
    id
}

/// Build a new project from a template, with ids from `next_id`. `None` for an unknown template.
pub fn build(template: &str, title: &str, next_id: &mut dyn FnMut() -> ItemId) -> Option<Project> {
    let mut p = Project::new();
    p.settings.book.title = title.to_owned();
    let mut item = |kind: Kind, t: &str, role: Role| Item::with_id(next_id(), kind, t).role(role);
    match template {
        "novel" => {
            let front = item(Kind::Folder, "Front Matter", Role::None);
            let tp = item(Kind::Text, "Title Page", Role::FrontMatter);
            let front_id = front.id;
            let tp_id = tp.id;
            add(&mut p, Place::end_of(Area::Manuscript), front.with_children(vec![tp]));
            let scene = item(Kind::Text, "Scene One", Role::Scene);
            let chapter = item(Kind::Folder, "Chapter One", Role::Chapter).with_children(vec![scene]);
            add(&mut p, Place::end_of(Area::Manuscript), item(Kind::Folder, "Part One", Role::Part).with_children(vec![chapter]));
            title_page_note(&mut p, tp_id);
            let _ = front_id;
            p.settings.targets.manuscript = Some(80_000);
        }
        "stories" => {
            let tp = item(Kind::Text, "Title Page", Role::FrontMatter);
            let contents = item(Kind::Text, "Contents", Role::FrontMatter);
            let tp_id = tp.id;
            let contents_id = contents.id;
            add(&mut p, Place::end_of(Area::Manuscript), item(Kind::Folder, "Front Matter", Role::None).with_children(vec![tp, contents]));
            let scene = item(Kind::Text, "Scene One", Role::Scene);
            add(&mut p, Place::end_of(Area::Manuscript), item(Kind::Folder, "Story One", Role::Chapter).with_children(vec![scene]));
            title_page_note(&mut p, tp_id);
            p.text_mut(contents_id).meta.synopsis = "The list of stories, made at export from the stories' titles.".into();
            p.settings.targets.manuscript = Some(60_000);
        }
        _ => return None,
    }
    for name in BIBLE_FOLDERS {
        add(&mut p, Place::end_of(Area::Bible), item(Kind::Folder, name, Role::None));
    }
    let premise = item(Kind::Text, "Premise", Role::None);
    let premise_id = premise.id;
    add(&mut p, Place::end_of(Area::Notes), premise);
    add(&mut p, Place::end_of(Area::Notes), item(Kind::Text, "Synopsis", Role::None));
    p.texts.insert(premise_id, ItemText::with_body(Text::new()));
    p.text_mut(premise_id).meta.synopsis = "What the book is about, in a sentence or two.".into();
    Some(p)
}

fn title_page_note(p: &mut Project, id: ItemId) {
    p.text_mut(id).meta.synopsis = "Made at export from the book's title, author and word count.".into();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids() -> impl FnMut() -> ItemId {
        let mut n = 0;
        move || {
            n += 1;
            ItemId::from_counter(n)
        }
    }

    #[test]
    fn the_novel_template_has_parts_chapters_scenes_and_a_bible() {
        let p = build("novel", "The River", &mut ids()).unwrap();
        let roles: Vec<(usize, Role)> = p.binder.rows_of(Area::Manuscript).iter().map(|r| (r.depth, r.item.role)).collect();
        assert_eq!(roles, vec![(0, Role::None), (1, Role::FrontMatter), (0, Role::Part), (1, Role::Chapter), (2, Role::Scene)]);
        let bible: Vec<&str> = p.binder.area(Area::Bible).iter().map(|i| i.title.as_str()).collect();
        assert_eq!(bible, BIBLE_FOLDERS);
        assert_eq!(p.binder.area(Area::Notes).len(), 2);
        assert_eq!(p.settings.book.title, "The River");
    }

    #[test]
    fn templates_give_fresh_ids_each_time() {
        let a = build("stories", "Tales", &mut ItemId::new).unwrap();
        let b = build("stories", "Tales", &mut ItemId::new).unwrap();
        let ia: Vec<ItemId> = a.binder.rows().iter().map(|r| r.item.id).collect();
        let ib: Vec<ItemId> = b.binder.rows().iter().map(|r| r.item.id).collect();
        assert_eq!(ia.len(), ib.len());
        assert!(ia.iter().all(|i| !ib.contains(i)));
        assert!(build("screenplay", "x", &mut ItemId::new).is_none());
    }
}
