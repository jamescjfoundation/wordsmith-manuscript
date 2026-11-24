// SPDX-License-Identifier: MIT OR Apache-2.0
//! The binder: the project's tree of folders and documents in fixed top-level areas.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::ItemId;

/// The binder's fixed top-level areas, in the order they are shown.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Area {
    /// The book itself: what exports include.
    Manuscript,
    /// Characters, places and the rest of the story bible.
    Bible,
    /// Research notes and media.
    Research,
    /// The writer's notes and plans.
    Notes,
    /// Deleted items, kept until the writer empties the Trash.
    Trash,
}

impl Area {
    /// Every area in display order.
    pub const ALL: [Area; 5] = [Area::Manuscript, Area::Bible, Area::Research, Area::Notes, Area::Trash];

    /// The area's name as shown in the binder.
    pub fn title(self) -> &'static str {
        match self {
            Area::Manuscript => "Manuscript",
            Area::Bible => "Story Bible",
            Area::Research => "Research",
            Area::Notes => "Notes",
            Area::Trash => "Trash",
        }
    }

    /// The area's key in files (`manuscript`, `bible`, ...).
    pub fn key(self) -> &'static str {
        match self {
            Area::Manuscript => "manuscript",
            Area::Bible => "bible",
            Area::Research => "research",
            Area::Notes => "notes",
            Area::Trash => "trash",
        }
    }

    /// The area for a file key.
    pub fn from_key(key: &str) -> Option<Area> {
        Area::ALL.into_iter().find(|a| a.key() == key)
    }

    fn index(self) -> usize {
        match self {
            Area::Manuscript => 0,
            Area::Bible => 1,
            Area::Research => 2,
            Area::Notes => 3,
            Area::Trash => 4,
        }
    }
}

/// What a binder item is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum Kind {
    /// A folder: holds other items, has no text of its own.
    Folder,
    /// A text document (`text/<id>.md`).
    #[default]
    Text,
    /// A story bible entry (`bible/<id>.md`).
    Entry,
    /// An image, PDF or other file kept in the project (`media/<id>.<ext>`).
    Media,
    /// A web link kept in Research.
    Link,
    /// A screenplay in Wordsmith Script's format (`scripts/<id>.wss`).
    Screenplay,
}

impl Kind {
    /// Whether the item has a text file.
    pub fn has_text(self) -> bool {
        matches!(self, Kind::Text | Kind::Folder)
    }

    /// Whether the item may hold children.
    pub fn can_hold_children(self) -> bool {
        matches!(self, Kind::Folder | Kind::Text | Kind::Entry)
    }
}

/// The structural role of a manuscript item. Roles drive export headings, numbering and scene
/// breaks, so the text itself never carries "Chapter 3".
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum Role {
    /// No role: a plain document or folder.
    #[default]
    None,
    /// A part (a group of chapters).
    Part,
    /// A chapter.
    Chapter,
    /// A scene.
    Scene,
    /// Front matter: title page, dedication, epigraph, contents and the like.
    FrontMatter,
    /// Back matter: acknowledgements, about the author and the like.
    BackMatter,
}

impl Role {
    /// Every role in menu order.
    pub const ALL: [Role; 6] = [Role::None, Role::Part, Role::Chapter, Role::Scene, Role::FrontMatter, Role::BackMatter];

    /// The role's name as shown.
    pub fn title(self) -> &'static str {
        match self {
            Role::None => "No role",
            Role::Part => "Part",
            Role::Chapter => "Chapter",
            Role::Scene => "Scene",
            Role::FrontMatter => "Front matter",
            Role::BackMatter => "Back matter",
        }
    }

    /// The role's key in files.
    pub fn key(self) -> &'static str {
        match self {
            Role::None => "none",
            Role::Part => "part",
            Role::Chapter => "chapter",
            Role::Scene => "scene",
            Role::FrontMatter => "front-matter",
            Role::BackMatter => "back-matter",
        }
    }

    /// The role for a key.
    pub fn from_key(key: &str) -> Option<Role> {
        Role::ALL.into_iter().find(|r| r.key() == key)
    }

    fn is_none(&self) -> bool {
        *self == Role::None
    }
}

fn is_true(b: &bool) -> bool {
    *b
}

fn yes() -> bool {
    true
}

/// One item in the binder.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Item {
    /// The stable id; files are named by it.
    pub id: ItemId,
    /// What the item is.
    #[serde(default)]
    pub kind: Kind,
    /// Its role in the manuscript.
    #[serde(default, skip_serializing_if = "Role::is_none")]
    pub role: Role,
    /// The title shown in the binder (and, for chapters and parts, in exports).
    #[serde(default)]
    pub title: String,
    /// Whether exports include it.
    #[serde(default = "yes", skip_serializing_if = "is_true")]
    pub include: bool,
    /// The item's label (a name from the project's label list).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// The item's status (a name from the project's status list).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    /// The icon's name, when not the kind's own.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    /// For media: the file extension (`png`, `pdf`); for a web link: the address.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    /// For an item in the Trash: the area it was deleted from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<Area>,
    /// Children, in order.
    #[serde(skip)]
    pub children: Vec<Item>,
}

impl Item {
    /// A new item with a fresh id.
    pub fn new(kind: Kind, title: impl Into<String>) -> Item {
        Item::with_id(ItemId::new(), kind, title)
    }

    /// A new item with the given id.
    pub fn with_id(id: ItemId, kind: Kind, title: impl Into<String>) -> Item {
        Item {
            id,
            kind,
            role: Role::None,
            title: title.into(),
            include: true,
            label: None,
            status: None,
            icon: None,
            file: None,
            from: None,
            children: Vec::new(),
        }
    }

    /// The same item with a role.
    pub fn role(mut self, role: Role) -> Item {
        self.role = role;
        self
    }

    /// The same item with children appended.
    pub fn with_children(mut self, children: Vec<Item>) -> Item {
        self.children.extend(children);
        self
    }

    /// The item and everything under it, depth first.
    pub fn walk(&self) -> Vec<&Item> {
        let mut out = Vec::new();
        let mut stack = vec![self];
        while let Some(item) = stack.pop() {
            out.push(item);
            stack.extend(item.children.iter().rev());
        }
        out
    }

    /// How many levels the deepest descendant is below this item (0 for a leaf).
    pub fn height(&self) -> usize {
        let mut max = 0;
        let mut stack = vec![(self, 0usize)];
        while let Some((item, d)) = stack.pop() {
            max = max.max(d);
            stack.extend(item.children.iter().map(|c| (c, d + 1)));
        }
        max
    }

    fn find(&self, id: ItemId) -> Option<&Item> {
        let mut stack = vec![self];
        while let Some(item) = stack.pop() {
            if item.id == id {
                return Some(item);
            }
            stack.extend(item.children.iter());
        }
        None
    }
}

/// Where an item sits: its area, its parent (none at the top of the area) and its index among
/// its siblings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Place {
    /// The area.
    pub area: Area,
    /// The parent item, or `None` at the top of the area.
    pub parent: Option<ItemId>,
    /// The index among the parent's children.
    pub index: usize,
}

impl Place {
    /// The end of an area's top level.
    pub fn end_of(area: Area) -> Place {
        Place { area, parent: None, index: usize::MAX }
    }

    /// The end of a parent's children.
    pub fn inside(area: Area, parent: ItemId) -> Place {
        Place { area, parent: Some(parent), index: usize::MAX }
    }
}

/// Why a binder change was refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BinderError {
    /// No item has this id.
    NotFound(ItemId),
    /// An item with this id is already in the binder.
    Duplicate(ItemId),
    /// An item cannot be moved into itself or its own descendants.
    IntoItself(ItemId),
    /// The parent cannot hold children (media, links, screenplays).
    CannotHoldChildren(ItemId),
    /// The tree would be deeper than the limit.
    TooDeep(usize),
}

impl fmt::Display for BinderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BinderError::NotFound(id) => write!(f, "no binder item {id}"),
            BinderError::Duplicate(id) => write!(f, "binder item {id} is already in the binder"),
            BinderError::IntoItself(id) => write!(f, "binder item {id} cannot be moved inside itself"),
            BinderError::CannotHoldChildren(id) => write!(f, "binder item {id} cannot hold other items"),
            BinderError::TooDeep(n) => write!(f, "the binder cannot be more than {n} levels deep"),
        }
    }
}

impl std::error::Error for BinderError {}

/// The deepest a binder may nest, counting the top level as 1.
pub const MAX_DEPTH: usize = 32;

/// The binder: one ordered tree per area.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Binder {
    areas: [Vec<Item>; 5],
}

/// One row of a depth-first walk of the binder.
#[derive(Clone, Copy, Debug)]
pub struct Row<'a> {
    /// The area.
    pub area: Area,
    /// How deep: 0 at the top of the area.
    pub depth: usize,
    /// The parent, if any.
    pub parent: Option<ItemId>,
    /// The item.
    pub item: &'a Item,
}

impl Binder {
    /// An empty binder.
    pub fn new() -> Binder {
        Binder::default()
    }

    /// An area's top-level items.
    pub fn area(&self, area: Area) -> &[Item] {
        self.areas.get(area.index()).map_or(&[], Vec::as_slice)
    }

    /// An area's top-level items, to change.
    fn area_vec(&mut self, area: Area) -> &mut Vec<Item> {
        // The array has one entry per area, so the index is always in range.
        &mut self.areas[area.index() % 5]
    }

    /// Every item, depth first, area by area.
    pub fn rows(&self) -> Vec<Row<'_>> {
        let mut out = Vec::new();
        for area in Area::ALL {
            self.rows_into(area, &mut out);
        }
        out
    }

    /// The items of one area, depth first.
    pub fn rows_of(&self, area: Area) -> Vec<Row<'_>> {
        let mut out = Vec::new();
        self.rows_into(area, &mut out);
        out
    }

    fn rows_into<'a>(&'a self, area: Area, out: &mut Vec<Row<'a>>) {
        let mut stack: Vec<(usize, Option<ItemId>, &Item)> = self.area(area).iter().rev().map(|i| (0, None, i)).collect();
        while let Some((depth, parent, item)) = stack.pop() {
            out.push(Row { area, depth, parent, item });
            stack.extend(item.children.iter().rev().map(|c| (depth + 1, Some(item.id), c)));
        }
    }

    /// How many items the binder holds.
    pub fn len(&self) -> usize {
        Area::ALL.iter().map(|a| self.area(*a).iter().map(|i| i.walk().len()).sum::<usize>()).sum()
    }

    /// Whether the binder is empty.
    pub fn is_empty(&self) -> bool {
        self.areas.iter().all(Vec::is_empty)
    }

    /// The item with this id.
    pub fn get(&self, id: ItemId) -> Option<&Item> {
        Area::ALL.iter().flat_map(|a| self.area(*a)).find_map(|i| i.find(id))
    }

    /// The item with this id, to change. Changing `children` here bypasses the binder's checks;
    /// use [`Binder::insert`] and [`Binder::move_to`] for the tree.
    pub fn get_mut(&mut self, id: ItemId) -> Option<&mut Item> {
        let mut stack: Vec<&mut Item> = self.areas.iter_mut().flat_map(|a| a.iter_mut()).collect();
        while let Some(item) = stack.pop() {
            if item.id == id {
                return Some(item);
            }
            stack.extend(item.children.iter_mut());
        }
        None
    }

    /// Whether an item with this id is in the binder.
    pub fn contains(&self, id: ItemId) -> bool {
        self.get(id).is_some()
    }

    /// Where an item sits.
    pub fn place_of(&self, id: ItemId) -> Option<Place> {
        for area in Area::ALL {
            let mut stack: Vec<(Option<ItemId>, &[Item])> = vec![(None, self.area(area))];
            while let Some((parent, items)) = stack.pop() {
                for (index, item) in items.iter().enumerate() {
                    if item.id == id {
                        return Some(Place { area, parent, index });
                    }
                    stack.push((Some(item.id), &item.children));
                }
            }
        }
        None
    }

    /// The area an item is in.
    pub fn area_of(&self, id: ItemId) -> Option<Area> {
        self.place_of(id).map(|p| p.area)
    }

    /// The ids from the top of the area down to the item's parent.
    pub fn ancestors(&self, id: ItemId) -> Vec<ItemId> {
        let mut out = Vec::new();
        let mut cur = self.place_of(id).and_then(|p| p.parent);
        while let Some(p) = cur {
            out.push(p);
            cur = self.place_of(p).and_then(|pl| pl.parent);
            if out.len() > MAX_DEPTH {
                break;
            }
        }
        out.reverse();
        out
    }

    /// How deep the item is: 1 at the top of its area.
    pub fn depth_of(&self, id: ItemId) -> Option<usize> {
        self.contains(id).then(|| self.ancestors(id).len() + 1)
    }

    fn siblings_mut(&mut self, area: Area, parent: Option<ItemId>) -> Result<&mut Vec<Item>, BinderError> {
        match parent {
            None => Ok(self.area_vec(area)),
            Some(p) => {
                let item = self.get_mut(p).ok_or(BinderError::NotFound(p))?;
                if !item.kind.can_hold_children() {
                    return Err(BinderError::CannotHoldChildren(p));
                }
                Ok(&mut item.children)
            }
        }
    }

    /// Insert an item (with its children) at a place. The index is clamped to the end.
    pub fn insert(&mut self, place: Place, item: Item) -> Result<(), BinderError> {
        for i in item.walk() {
            if self.contains(i.id) {
                return Err(BinderError::Duplicate(i.id));
            }
        }
        if let Some(p) = place.parent {
            match self.place_of(p) {
                Some(pp) if pp.area == place.area => {}
                _ => return Err(BinderError::NotFound(p)),
            }
        }
        let depth = place.parent.and_then(|p| self.depth_of(p)).unwrap_or(0) + 1 + item.height();
        if depth > MAX_DEPTH {
            return Err(BinderError::TooDeep(MAX_DEPTH));
        }
        let siblings = self.siblings_mut(place.area, place.parent)?;
        let index = place.index.min(siblings.len());
        siblings.insert(index, item);
        Ok(())
    }

    /// Take an item (with its children) out of the binder.
    pub fn remove(&mut self, id: ItemId) -> Option<(Place, Item)> {
        let place = self.place_of(id)?;
        let siblings = self.siblings_mut(place.area, place.parent).ok()?;
        (place.index < siblings.len()).then(|| (place, siblings.remove(place.index)))
    }

    /// Move an item to a new place. The index is the index among the new siblings *before* the
    /// item is taken out, as in a drag and drop; the move is refused into the item itself.
    pub fn move_to(&mut self, id: ItemId, to: Place) -> Result<Place, BinderError> {
        let from = self.place_of(id).ok_or(BinderError::NotFound(id))?;
        if let Some(p) = to.parent {
            if p == id || self.ancestors(p).contains(&id) {
                return Err(BinderError::IntoItself(id));
            }
            match self.get(p) {
                None => return Err(BinderError::NotFound(p)),
                Some(item) if !item.kind.can_hold_children() => return Err(BinderError::CannotHoldChildren(p)),
                Some(_) => {}
            }
        }
        let height = self.get(id).map_or(0, Item::height);
        let depth = to.parent.and_then(|p| self.depth_of(p)).unwrap_or(0) + 1 + height;
        if depth > MAX_DEPTH {
            return Err(BinderError::TooDeep(MAX_DEPTH));
        }
        let (_, item) = self.remove(id).ok_or(BinderError::NotFound(id))?;
        let mut index = to.index;
        if from.area == to.area && from.parent == to.parent && from.index < index && index != usize::MAX {
            index -= 1;
        }
        let place = Place { index, ..to };
        let siblings = self.siblings_mut(place.area, place.parent)?;
        let index = place.index.min(siblings.len());
        siblings.insert(index, item);
        Ok(Place { index, ..place })
    }

    /// Move an item (and its children) to the end of the Trash, remembering its area.
    pub fn trash(&mut self, id: ItemId) -> Result<Place, BinderError> {
        let from = self.place_of(id).ok_or(BinderError::NotFound(id))?;
        if from.area == Area::Trash {
            return Ok(from);
        }
        let place = self.move_to(id, Place::end_of(Area::Trash))?;
        if let Some(item) = self.get_mut(id) {
            item.from = Some(from.area);
        }
        Ok(place)
    }

    /// Put an item from the Trash back at the end of the area it came from.
    pub fn restore(&mut self, id: ItemId) -> Result<Place, BinderError> {
        let area = self.get(id).ok_or(BinderError::NotFound(id))?.from.unwrap_or(Area::Manuscript);
        let place = self.move_to(id, Place::end_of(area))?;
        if let Some(item) = self.get_mut(id) {
            item.from = None;
        }
        Ok(place)
    }

    /// Remove everything in the Trash for good, returning what was removed.
    pub fn empty_trash(&mut self) -> Vec<Item> {
        std::mem::take(self.area_vec(Area::Trash))
    }

    /// The items of an area in reading order (depth first), as ids.
    pub fn order(&self, area: Area) -> Vec<ItemId> {
        self.rows_of(area).into_iter().map(|r| r.item.id).collect()
    }

    /// The deepest level in the binder (1 for top-level items only, 0 when empty).
    pub fn max_depth(&self) -> usize {
        self.rows().iter().map(|r| r.depth + 1).max().unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(n: u64, title: &str) -> Item {
        Item::with_id(ItemId::from_counter(n), Kind::Text, title)
    }

    fn folder(n: u64, title: &str) -> Item {
        Item::with_id(ItemId::from_counter(n), Kind::Folder, title)
    }

    fn sample() -> Binder {
        let mut b = Binder::new();
        let ch1 = folder(1, "Chapter One").role(Role::Chapter).with_children(vec![text(2, "Arrival").role(Role::Scene), text(3, "Dinner")]);
        let ch2 = folder(4, "Chapter Two").role(Role::Chapter).with_children(vec![text(5, "Morning")]);
        b.insert(Place::end_of(Area::Manuscript), ch1).unwrap();
        b.insert(Place::end_of(Area::Manuscript), ch2).unwrap();
        b.insert(Place::end_of(Area::Notes), text(6, "Plan")).unwrap();
        b
    }

    fn id(n: u64) -> ItemId {
        ItemId::from_counter(n)
    }

    #[test]
    fn rows_walk_depth_first() {
        let b = sample();
        let titles: Vec<_> = b.rows_of(Area::Manuscript).iter().map(|r| (r.depth, r.item.title.clone())).collect();
        assert_eq!(
            titles,
            vec![(0, "Chapter One".into()), (1, "Arrival".into()), (1, "Dinner".into()), (0, "Chapter Two".into()), (1, "Morning".into())]
        );
        assert_eq!(b.len(), 6);
        assert_eq!(b.max_depth(), 2);
    }

    #[test]
    fn moving_within_a_parent_uses_the_drop_index() {
        let mut b = sample();
        // Drop "Arrival" after "Dinner" (index 2 before removal).
        let placed = b.move_to(id(2), Place { area: Area::Manuscript, parent: Some(id(1)), index: 2 }).unwrap();
        assert_eq!(placed.index, 1);
        let kids: Vec<_> = b.get(id(1)).unwrap().children.iter().map(|c| c.title.as_str()).collect();
        assert_eq!(kids, ["Dinner", "Arrival"]);
    }

    #[test]
    fn moving_across_parents_and_areas() {
        let mut b = sample();
        b.move_to(id(3), Place { area: Area::Manuscript, parent: Some(id(4)), index: 0 }).unwrap();
        assert_eq!(b.place_of(id(3)), Some(Place { area: Area::Manuscript, parent: Some(id(4)), index: 0 }));
        b.move_to(id(6), Place::inside(Area::Manuscript, id(4))).unwrap();
        assert_eq!(b.area_of(id(6)), Some(Area::Manuscript));
        assert_eq!(b.len(), 6);
    }

    #[test]
    fn an_item_cannot_move_into_itself() {
        let mut b = sample();
        assert_eq!(b.move_to(id(1), Place::inside(Area::Manuscript, id(2))), Err(BinderError::IntoItself(id(1))));
        assert_eq!(b.move_to(id(1), Place::inside(Area::Manuscript, id(1))), Err(BinderError::IntoItself(id(1))));
        assert_eq!(b, sample());
    }

    #[test]
    fn duplicates_and_missing_parents_are_refused() {
        let mut b = sample();
        assert_eq!(b.insert(Place::end_of(Area::Research), text(2, "Again")), Err(BinderError::Duplicate(id(2))));
        assert_eq!(b.insert(Place::inside(Area::Research, id(1)), text(9, "X")), Err(BinderError::NotFound(id(1))));
    }

    #[test]
    fn media_cannot_hold_children() {
        let mut b = sample();
        b.insert(Place::end_of(Area::Research), Item::with_id(id(10), Kind::Media, "Map")).unwrap();
        assert_eq!(b.insert(Place::inside(Area::Research, id(10)), text(11, "X")), Err(BinderError::CannotHoldChildren(id(10))));
        assert_eq!(b.move_to(id(6), Place::inside(Area::Research, id(10))), Err(BinderError::CannotHoldChildren(id(10))));
    }

    #[test]
    fn trash_keeps_items_until_emptied_and_restores_to_their_area() {
        let mut b = sample();
        b.trash(id(1)).unwrap();
        assert_eq!(b.area_of(id(2)), Some(Area::Trash));
        assert_eq!(b.get(id(1)).unwrap().from, Some(Area::Manuscript));
        b.restore(id(1)).unwrap();
        assert_eq!(b.area_of(id(2)), Some(Area::Manuscript));
        assert_eq!(b.get(id(1)).unwrap().from, None);
        b.trash(id(6)).unwrap();
        let gone = b.empty_trash();
        assert_eq!(gone.len(), 1);
        assert!(!b.contains(id(6)));
        assert_eq!(b.len(), 5);
    }

    #[test]
    fn depth_is_limited() {
        let mut b = Binder::new();
        let mut parent = None;
        for n in 0..MAX_DEPTH as u64 {
            let place = match parent {
                None => Place::end_of(Area::Notes),
                Some(p) => Place::inside(Area::Notes, p),
            };
            b.insert(place, folder(n + 100, "f")).unwrap();
            parent = Some(id(n + 100));
        }
        let place = Place::inside(Area::Notes, parent.unwrap());
        assert_eq!(b.insert(place, folder(999, "f")), Err(BinderError::TooDeep(MAX_DEPTH)));
    }

    #[test]
    fn ancestors_run_from_the_top() {
        let b = sample();
        assert_eq!(b.ancestors(id(2)), vec![id(1)]);
        assert_eq!(b.depth_of(id(2)), Some(2));
        assert_eq!(b.order(Area::Manuscript), vec![id(1), id(2), id(3), id(4), id(5)]);
    }
}
