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

/// Settings a template leaves behind: they belong to one book's working life.
pub const NOT_IN_TEMPLATES: [&str; 5] = ["bookmarks", "collections", "board", "revisions", "export_last"];

/// A template made from a project (BND-4): its binder (the Trash left out), lists, fields, entry
/// types, targets and export settings, the synopses and notes of every item, and the text of
/// everything that is not a chapter or scene (front and back matter, notes, research). The
/// manuscript's own text, the bible's entries, snapshots and comments stay with the project;
/// so do the book's title and other details of this one book (the writer's name, contact and
/// language stay).
pub fn skeleton(p: &Project) -> Project {
    let mut out = Project::new();
    out.settings = p.settings.clone();
    out.settings.extra.retain(|k, _| !NOT_IN_TEMPLATES.contains(&k.as_str()));
    let b = &p.settings.book;
    out.settings.book = crate::BookInfo {
        author: b.author.clone(),
        pen_name: b.pen_name.clone(),
        surname: b.surname.clone(),
        contact: b.contact.clone(),
        language: b.language.clone(),
        publisher: b.publisher.clone(),
        ..crate::BookInfo::default()
    };
    for area in [Area::Manuscript, Area::Bible, Area::Research, Area::Notes] {
        for item in p.binder.area(area) {
            let kept = keep(item);
            if let Some(k) = kept {
                let _ = out.binder.insert(Place::end_of(area), k);
            }
        }
    }
    for row in out.binder.rows() {
        let id = row.item.id;
        let Some(t) = p.texts.get(&id) else { continue };
        let mut t = t.clone();
        if row.area == Area::Manuscript && !matches!(row.item.role, Role::FrontMatter | Role::BackMatter) {
            t.body = Text::new();
        }
        t.meta.comments.clear();
        for run in t.body.inline_runs_mut() {
            for i in run.iter_mut() {
                match i {
                    crate::Inline::Span { marks, .. } | crate::Inline::NoteRef { marks, .. } => {
                        marks.comment = None;
                        marks.rev = None;
                    }
                }
            }
        }
        t.body.normalize();
        out.texts.insert(id, t);
    }
    out
}

/// The item and what is under it, bible entries and media files left out (a template keeps the
/// bible's folders, and web links).
fn keep(item: &Item) -> Option<Item> {
    if matches!(item.kind, Kind::Entry | Kind::Media) {
        return None;
    }
    let mut out = item.clone();
    out.children = item.children.iter().filter_map(keep).collect();
    Some(out)
}

/// The same project with new ids from `next_id` (each project made from a template is its own):
/// the binder, texts and links between items follow.
pub fn with_fresh_ids(p: &Project, next_id: &mut dyn FnMut() -> ItemId) -> Project {
    let mut map: std::collections::BTreeMap<ItemId, ItemId> = std::collections::BTreeMap::new();
    for row in p.binder.rows() {
        map.insert(row.item.id, next_id());
    }
    fn renew(item: &Item, map: &std::collections::BTreeMap<ItemId, ItemId>) -> Item {
        let mut out = item.clone();
        out.id = map.get(&item.id).copied().unwrap_or(item.id);
        out.children = item.children.iter().map(|c| renew(c, map)).collect();
        out
    }
    let mut out = Project::new();
    out.settings = p.settings.clone();
    for area in [Area::Manuscript, Area::Bible, Area::Research, Area::Notes, Area::Trash] {
        for item in p.binder.area(area) {
            let _ = out.binder.insert(Place::end_of(area), renew(item, &map));
        }
    }
    let relink = |link: &mut Option<String>| {
        if let Some(l) = link.as_mut()
            && let Some(old) = l.strip_prefix("wsm:").and_then(|x| x.parse::<ItemId>().ok())
            && let Some(new) = map.get(&old)
        {
            *l = format!("wsm:{}", new.as_text());
        }
    };
    for (id, t) in &p.texts {
        let Some(new) = map.get(id) else { continue };
        let mut t = t.clone();
        for run in t.body.inline_runs_mut() {
            for i in run.iter_mut() {
                match i {
                    crate::Inline::Span { marks, .. } | crate::Inline::NoteRef { marks, .. } => relink(&mut marks.link),
                }
            }
        }
        t.meta.entries = t.meta.entries.iter().filter_map(|e| map.get(e).copied()).collect();
        out.texts.insert(*new, t);
    }
    for (id, e) in &p.entries {
        if let Some(new) = map.get(id) {
            let mut e = e.clone();
            for (_, to) in e.relations.iter_mut() {
                if let Some(n) = map.get(to) {
                    *to = *n;
                }
            }
            for run in e.body.inline_runs_mut() {
                for i in run.iter_mut() {
                    match i {
                        crate::Inline::Span { marks, .. } | crate::Inline::NoteRef { marks, .. } => relink(&mut marks.link),
                    }
                }
            }
            out.entries.insert(*new, e);
        }
    }
    out
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
    fn a_skeleton_keeps_the_shape_and_guidance_but_not_the_book() {
        let mut p = build("novel", "The River", &mut ids()).unwrap();
        let rows: Vec<(ItemId, Role)> = p.binder.rows_of(Area::Manuscript).iter().map(|r| (r.item.id, r.item.role)).collect();
        let scene = rows.iter().find(|r| r.1 == Role::Scene).map(|r| r.0).unwrap();
        let tp = rows.iter().find(|r| r.1 == Role::FrontMatter).map(|r| r.0).unwrap();
        p.text_mut(scene).body = Text::from_paragraphs(&["The book's own words."]);
        p.text_mut(scene).meta.synopsis = "Open on the river.".into();
        p.text_mut(tp).body = Text::from_paragraphs(&["By the author"]);
        p.settings.extra.insert("bookmarks".into(), serde_json::json!([scene.as_text()]));
        p.settings.book.author = "A. Writer".into();
        let entry = Item::with_id(ItemId::from_counter(900), Kind::Entry, "Mara");
        let chars = p.binder.area(Area::Bible)[0].id;
        p.binder.insert(Place::inside(Area::Bible, chars), entry).unwrap();
        let s = skeleton(&p);
        assert_eq!(s.settings.book.title, crate::BookInfo::default().title);
        assert_eq!(s.settings.book.author, "A. Writer");
        assert!(!s.settings.extra.contains_key("bookmarks"));
        assert!(s.texts[&scene].body.is_empty(), "the manuscript's words stay with the book");
        assert_eq!(s.texts[&scene].meta.synopsis, "Open on the river.");
        assert_eq!(s.texts[&tp].body, Text::from_paragraphs(&["By the author"]));
        assert!(!s.binder.contains(ItemId::from_counter(900)), "entries stay with the book");
        assert_eq!(s.binder.area(Area::Bible).len(), BIBLE_FOLDERS.len());
        let fresh = with_fresh_ids(&s, &mut ItemId::new);
        assert_eq!(fresh.binder.rows().len(), s.binder.rows().len());
        assert!(fresh.binder.rows().iter().all(|r| !s.binder.contains(r.item.id)));
        assert_eq!(fresh.texts.len(), s.texts.len());
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
