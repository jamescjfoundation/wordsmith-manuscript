// SPDX-License-Identifier: MIT OR Apache-2.0
//! Series with a shared bible (BND-8, BIB-5, decision 0007).
//!
//! A series keeps one story bible for several books. The series' bible is itself a project
//! folder that holds only a Story Bible: its settings name the series, list its books and keep
//! the entry types; its binder holds the Bible area; its `bible/` folder the entries. A book in
//! the series names that folder (`series` in its settings, a path from the book's folder). In
//! memory the book is whole: [`merge`] puts the series' bible into the book's Bible area, and
//! [`split`] takes it out again for saving, so every view and command works as before.

use serde::{Deserialize, Serialize};

use crate::{Area, Binder, Place, Project};

/// The settings key of a series bible's own details.
pub const KEY: &str = "series";
/// The settings key the entry types are kept under (as in a book).
pub const TEMPLATES: &str = "templates";

/// A series bible's details (in its project settings).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SeriesInfo {
    /// The series' name.
    pub name: String,
    /// The books in the series: paths from the series folder to each book's folder, in order.
    pub books: Vec<String>,
}

/// A series bible's details, if a project is one.
pub fn info(p: &Project) -> Option<SeriesInfo> {
    p.settings.extra.get(KEY).and_then(|v| serde_json::from_value(v.clone()).ok())
}

/// Set a series bible's details.
pub fn set_info(p: &mut Project, info: &SeriesInfo) {
    if let Ok(v) = serde_json::to_value(info) {
        p.settings.extra.insert(KEY.into(), v);
    }
}

/// A new, empty series bible.
pub fn new_bible(name: &str) -> Project {
    let mut p = Project::new();
    p.settings.book.title = name.to_owned();
    set_info(&mut p, &SeriesInfo { name: name.to_owned(), books: Vec::new() });
    p
}

/// Split a book of a series into what its own folder keeps (everything but the Story Bible) and
/// the series' bible (the Bible area, its entries and texts, and the entry types), keeping the
/// series bible's own details from `series`.
pub fn split(book: &Project, series: &Project) -> (Project, Project) {
    let mut own = book.clone();
    let mut bible = Project { settings: series.settings.clone(), ..Project::default() };
    match book.settings.extra.get(TEMPLATES) {
        Some(t) => {
            bible.settings.extra.insert(TEMPLATES.into(), t.clone());
        }
        None => {
            bible.settings.extra.remove(TEMPLATES);
        }
    }
    own.binder = Binder::new();
    for area in Area::ALL {
        for item in book.binder.area(area) {
            let target = if area == Area::Bible { &mut bible.binder } else { &mut own.binder };
            let _ = target.insert(Place::end_of(area), item.clone());
        }
    }
    for row in bible.binder.rows() {
        let id = row.item.id;
        if let Some(e) = own.entries.remove(&id) {
            bible.entries.insert(id, e);
        }
        if let Some(t) = own.texts.remove(&id) {
            bible.texts.insert(id, t);
        }
        if let Some(s) = own.snapshots.remove(&id) {
            bible.snapshots.insert(id, s);
        }
    }
    own.settings.extra.remove(TEMPLATES);
    (own, bible)
}

/// A book of a series, whole: its own folder's project with the series' Story Bible and entry
/// types. Anything the book's folder still holds in its Bible area (from before it joined) stays,
/// after the series' items.
pub fn merge(own: &Project, bible: &Project) -> Project {
    let mut out = own.clone();
    let mut book_bible: Vec<crate::Item> = Vec::new();
    for item in own.binder.area(Area::Bible) {
        book_bible.push(item.clone());
    }
    for item in &book_bible {
        let _ = out.binder.remove(item.id);
    }
    for item in bible.binder.area(Area::Bible) {
        let _ = out.binder.insert(Place::end_of(Area::Bible), item.clone());
    }
    for item in book_bible {
        if !out.binder.contains(item.id) {
            let _ = out.binder.insert(Place::end_of(Area::Bible), item);
        }
    }
    out.entries.extend(bible.entries.clone());
    out.texts.extend(bible.texts.clone());
    out.snapshots.extend(bible.snapshots.clone());
    if let Some(t) = bible.settings.extra.get(TEMPLATES) {
        out.settings.extra.insert(TEMPLATES.into(), t.clone());
    }
    out
}

/// A path from `from` (a folder) to `to`, through their common folder (`../Series`), or `to`
/// itself when they share none.
pub fn relative(from: &std::path::Path, to: &std::path::Path) -> String {
    let a: Vec<_> = from.components().collect();
    let b: Vec<_> = to.components().collect();
    let common = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
    if common == 0 {
        return to.to_string_lossy().into_owned();
    }
    let mut parts: Vec<String> = std::iter::repeat_n("..".to_owned(), a.len() - common).collect();
    parts.extend(b[common..].iter().map(|c| c.as_os_str().to_string_lossy().into_owned()));
    if parts.is_empty() { ".".into() } else { parts.join("/") }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Entry, Item, ItemId, ItemText, Kind, Text};

    fn id(n: u64) -> ItemId {
        ItemId::from_counter(n)
    }

    #[test]
    fn a_book_splits_into_its_own_part_and_the_series_bible_and_back() {
        let mut p = Project::new();
        p.binder.insert(Place::end_of(Area::Manuscript), Item::with_id(id(1), Kind::Text, "Scene")).unwrap();
        p.texts.insert(id(1), ItemText::with_body(Text::from_paragraphs(&["Mara walked."])));
        p.binder.insert(Place::end_of(Area::Bible), Item::with_id(id(10), Kind::Folder, "Characters")).unwrap();
        p.binder.insert(Place::inside(Area::Bible, id(10)), Item::with_id(id(11), Kind::Entry, "Mara")).unwrap();
        p.entries.insert(id(11), Entry::new("character", "Mara"));
        p.settings.extra.insert(TEMPLATES.into(), serde_json::json!([]));
        let series = new_bible("The Coast");
        let (own, bible) = split(&p, &series);
        assert!(own.binder.area(Area::Bible).is_empty() && own.entries.is_empty());
        assert!(!own.settings.extra.contains_key(TEMPLATES));
        assert_eq!(own.texts.len(), 1);
        assert_eq!(bible.entries.len(), 1);
        assert!(bible.binder.area(Area::Manuscript).is_empty());
        assert_eq!(info(&bible).unwrap().name, "The Coast");
        assert_eq!(merge(&own, &bible), p);
    }

    #[test]
    fn relative_paths() {
        use std::path::Path;
        assert_eq!(relative(Path::new("/w/Book One.wsm"), Path::new("/w/Coast.wsm")), "../Coast.wsm");
        assert_eq!(relative(Path::new("/w/a/Book.wsm"), Path::new("/w/Coast.wsm")), "../../Coast.wsm");
    }
}
