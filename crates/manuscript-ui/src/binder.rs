// SPDX-License-Identifier: MIT OR Apache-2.0
//! The binder panel: the project's tree in its five areas (BND-1), roles (BND-2), include in
//! export (BND-3) and the Trash (BND-5). Every change goes through a command.

use std::collections::HashSet;

use jcj_shell::egui::{self, Color32, Ui};
use serde_json::json;
use wordsmith_manuscript_model::{Area, ItemId, Kind, Place, Project, Role};

use crate::session::Session;
use crate::tree::{TreeEvent, TreeRow, TreeState, Zone, tree};

/// A row of the binder: an area heading or an item.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Node {
    /// An area's heading.
    Area(Area),
    /// An item.
    Item(ItemId),
}

/// The binder panel's state.
#[derive(Default)]
pub struct BinderView {
    /// The tree widget's state.
    pub tree: TreeState<Node>,
    /// Rows whose children are hidden.
    pub closed: HashSet<Node>,
    /// The selected item.
    pub selected: Option<ItemId>,
    /// An item the writer asked to open (double-click, Enter).
    pub opened: Option<ItemId>,
    /// The context menu: on which row, and where.
    menu: Option<(Node, egui::Pos2)>,
    /// Waiting for the writer to confirm emptying the Trash.
    pub confirm_empty: bool,
    /// Items hidden by the filters (SRC-3); their ancestors stay as context.
    pub hidden: HashSet<ItemId>,
    /// Asking for a picture's or PDF's path (`false`) or a web address (`true`), and where it goes.
    pub add_media: Option<(bool, String, Option<ItemId>)>,
}

fn hex(s: &str) -> Option<Color32> {
    let h = s.strip_prefix('#')?;
    let b = |i: usize| h.get(i..i + 2).and_then(|x| u8::from_str_radix(x, 16).ok());
    Some(Color32::from_rgb(b(0)?, b(2)?, b(4)?))
}

/// The visible rows of the binder.
pub fn rows(p: &Project, view: &BinderView) -> Vec<TreeRow<Node>> {
    let mut out = Vec::new();
    for area in Area::ALL {
        let node = Node::Area(area);
        let items = p.binder.area(area);
        let open = !view.closed.contains(&node);
        let mut r = TreeRow::new(node, 0, area.title());
        r.heading = true;
        r.draggable = false;
        r.has_children = !items.is_empty();
        r.open = open;
        if area == Area::Manuscript {
            r.trailing = format_words(p.manuscript_words());
        }
        out.push(r);
        if !open {
            continue;
        }
        let mut skip_below: Option<usize> = None;
        for row in p.binder.rows_of(area) {
            if let Some(d) = skip_below {
                if row.depth > d {
                    continue;
                }
                skip_below = None;
            }
            if view.hidden.contains(&row.item.id) {
                continue;
            }
            let n = Node::Item(row.item.id);
            let item = row.item;
            let mut r = TreeRow::new(n, row.depth + 1, if item.title.is_empty() { "Untitled".to_owned() } else { item.title.clone() });
            r.has_children = !item.children.is_empty();
            r.open = !view.closed.contains(&n);
            r.dim = !item.include || area == Area::Trash;
            r.accepts_children = item.kind.can_hold_children();
            r.glyph = match (item.kind, item.role) {
                (_, Role::Part) => "P".into(),
                (_, Role::Chapter) => "C".into(),
                (_, Role::Scene) => "S".into(),
                (_, Role::FrontMatter | Role::BackMatter) => "M".into(),
                (Kind::Folder, _) => "F".into(),
                (Kind::Entry, _) => "E".into(),
                (Kind::Media, _) => "I".into(),
                (Kind::Link, _) => "L".into(),
                (Kind::Screenplay, _) => "W".into(),
                (Kind::Text, _) => "·".into(),
            };
            r.mark = item.label.as_deref().and_then(|l| p.settings.label_color(l)).and_then(hex);
            if item.kind.has_text() {
                let w = p.words_under(item.id);
                if w > 0 {
                    r.trailing = format_words(w);
                }
            }
            if r.has_children && !r.open {
                skip_below = Some(row.depth);
            }
            out.push(r);
        }
    }
    out
}

/// A word count as shown: `1,234`.
pub fn format_words(n: usize) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// The `binder.move` parameters for dropping `item` on `target`.
pub fn drop_params(p: &Project, item: ItemId, target: Node, zone: Zone) -> Option<serde_json::Value> {
    let place = match target {
        Node::Area(area) => Place::end_of(area),
        Node::Item(t) => {
            let tp = p.binder.place_of(t)?;
            let can_hold = p.binder.get(t).is_some_and(|i| i.kind.can_hold_children());
            match zone {
                Zone::Before => tp,
                Zone::After => Place { index: tp.index + 1, ..tp },
                Zone::Inside if can_hold => Place::inside(tp.area, t),
                Zone::Inside => Place { index: tp.index + 1, ..tp },
            }
        }
    };
    let mut v = json!({ "id": item.as_text(), "area": place.area.key() });
    if let Some(parent) = place.parent {
        v["parent"] = json!(parent.as_text());
    }
    if place.index != usize::MAX {
        v["index"] = json!(place.index);
    }
    Some(v)
}

/// Draw the binder panel. Errors from commands are returned for the status bar.
pub fn panel(ui: &mut Ui, s: &mut Session, view: &mut BinderView) -> Vec<String> {
    let mut errors = Vec::new();
    view.tree.selected = view.selected.map(Node::Item);
    let rows = rows(&s.project, view);
    let events = tree(ui, "binder", &mut view.tree, &rows);
    let pointer = ui.input(|i| i.pointer.interact_pos());
    for e in events {
        let result = match e {
            TreeEvent::Select(Node::Item(id)) => {
                view.selected = Some(id);
                Ok(())
            }
            TreeEvent::Select(Node::Area(_)) => Ok(()),
            TreeEvent::Activate(Node::Item(id)) => {
                view.selected = Some(id);
                view.opened = Some(id);
                Ok(())
            }
            TreeEvent::Activate(n @ Node::Area(_)) | TreeEvent::Toggle(n) => {
                if !view.closed.remove(&n) {
                    view.closed.insert(n);
                }
                Ok(())
            }
            TreeEvent::Rename(Node::Item(id), title) => s.run("binder.rename", &json!({ "id": id.as_text(), "title": title })).map(|_| ()),
            TreeEvent::Rename(..) => Ok(()),
            TreeEvent::Drop { item: Node::Item(item), target, zone } => match drop_params(&s.project, item, target, zone) {
                Some(v) => s.run("binder.move", &v).map(|_| ()),
                None => Ok(()),
            },
            TreeEvent::Drop { .. } => Ok(()),
            TreeEvent::Menu(n) => {
                view.menu = pointer.map(|p| (n, p));
                Ok(())
            }
        };
        if let Err(e) = result {
            errors.push(e);
        }
    }
    keys(ui, s, view, &mut errors);
    menu(ui, s, view, &mut errors);
    confirm_empty(ui, s, view, &mut errors);
    add_media(ui, s, view, &mut errors);
    errors
}

fn keys(ui: &mut Ui, s: &mut Session, view: &mut BinderView, errors: &mut Vec<String>) {
    let hovered = ui.rect_contains_pointer(ui.max_rect());
    let typing = ui.ctx().memory(|m| m.focused().is_some());
    if !hovered || typing || view.tree.renaming.is_some() {
        return;
    }
    let (up, down, f2, del) = ui.input(|i| {
        (i.key_pressed(egui::Key::ArrowUp), i.key_pressed(egui::Key::ArrowDown), i.key_pressed(egui::Key::F2), i.key_pressed(egui::Key::Delete))
    });
    let items: Vec<ItemId> = rows(&s.project, view)
        .iter()
        .filter_map(|r| match r.key {
            Node::Item(id) => Some(id),
            Node::Area(_) => None,
        })
        .collect();
    let at = view.selected.and_then(|sel| items.iter().position(|i| *i == sel));
    if up || down {
        let next = match (at, up) {
            (Some(i), true) => i.checked_sub(1),
            (Some(i), false) => Some((i + 1).min(items.len().saturating_sub(1))),
            (None, _) => Some(0),
        };
        if let Some(n) = next.and_then(|n| items.get(n)) {
            view.selected = Some(*n);
            view.tree.reveal = true;
        }
    }
    if f2 && let Some(id) = view.selected {
        let title = s.project.binder.get(id).map(|i| i.title.clone()).unwrap_or_default();
        view.tree.rename(Node::Item(id), &title);
    }
    if del && let Some(id) = view.selected {
        if let Err(e) = s.run("binder.trash", &json!({ "id": id.as_text() })) {
            errors.push(e);
        }
    }
}

fn menu(ui: &mut Ui, s: &mut Session, view: &mut BinderView, errors: &mut Vec<String>) {
    let Some((node, at)) = view.menu else { return };
    let mut close = false;
    let mut todo: Vec<(&'static str, serde_json::Value)> = Vec::new();
    let mut rename: Option<String> = None;
    let area = egui::Area::new(egui::Id::unique("manuscript-binder-menu")).order(egui::Order::Foreground).fixed_pos(at).show(ui.ctx(), |ui| {
        egui::Frame::popup(ui.style()).show(ui, |ui| {
            ui.set_min_width(180.0);
            let mut run = |id: &'static str, v: serde_json::Value| todo.push((id, v));
            match node {
                Node::Area(Area::Trash) => {
                    if ui.button("Empty Trash…").clicked() {
                        todo.push(("confirm-empty", serde_json::Value::Null));
                        close = true;
                    }
                }
                Node::Area(Area::Bible) => {
                    for t in wordsmith_manuscript_model::bible::templates(&s.project.settings) {
                        if ui.button(format!("New {}", t.title)).clicked() {
                            run("bible.new", json!({ "kind": t.kind, "name": format!("New {}", t.title) }));
                            close = true;
                        }
                    }
                    if ui.button("New Folder").clicked() {
                        run("binder.newFolder", json!({ "area": "bible" }));
                        close = true;
                    }
                }
                Node::Area(a) => {
                    if ui.button("New Document").clicked() {
                        run("binder.new", json!({ "area": a.key() }));
                        close = true;
                    }
                    if a == Area::Research {
                        if ui.button("Add Image or PDF…").clicked() {
                            run("ask-media", json!(false));
                            close = true;
                        }
                        if ui.button("Add Web Link…").clicked() {
                            run("ask-media", json!(true));
                            close = true;
                        }
                    }
                    if ui.button("New Folder").clicked() {
                        run("binder.newFolder", json!({ "area": a.key() }));
                        close = true;
                    }
                }
                Node::Item(id) => {
                    let item = s.project.binder.get(id).cloned();
                    let in_trash = s.project.binder.area_of(id) == Some(Area::Trash);
                    if !in_trash {
                        if ui.button("New Document After").clicked() {
                            run("binder.new", json!({ "after": id.as_text() }));
                            close = true;
                        }
                        if item.as_ref().is_some_and(|i| i.kind.can_hold_children()) && ui.button("New Document Inside").clicked() {
                            run("binder.new", json!({ "inside": id.as_text() }));
                            close = true;
                        }
                        if ui.button("New Folder After").clicked() {
                            run("binder.newFolder", json!({ "after": id.as_text() }));
                            close = true;
                        }
                        if s.project.binder.area_of(id) == Some(Area::Research) && item.as_ref().is_some_and(|i| i.kind.can_hold_children()) {
                            if ui.button("Add Image or PDF Inside…").clicked() {
                                run("ask-media", json!(false));
                                close = true;
                            }
                            if ui.button("Add Web Link Inside…").clicked() {
                                run("ask-media", json!(true));
                                close = true;
                            }
                        }
                    }
                    ui.separator();
                    if ui.button("Rename").clicked() {
                        rename = Some(item.as_ref().map_or(String::new(), |i| i.title.clone()));
                        close = true;
                    }
                    if s.project.binder.area_of(id) == Some(Area::Manuscript) {
                        ui.menu_button("Role", |ui| {
                            for r in Role::ALL {
                                let on = item.as_ref().is_some_and(|i| i.role == r);
                                if ui.selectable_label(on, r.title()).clicked() {
                                    run("binder.setRole", json!({ "id": id.as_text(), "role": r.key() }));
                                    close = true;
                                }
                            }
                        });
                        let include = item.as_ref().is_some_and(|i| i.include);
                        if ui.selectable_label(include, "Include in Export").clicked() {
                            run("binder.setInclude", json!({ "id": id.as_text(), "include": !include }));
                            close = true;
                        }
                    }
                    if !in_trash {
                        let marked = crate::bookmarks::all(&s.project).contains(&id);
                        if ui.button(if marked { "Remove Bookmark" } else { "Bookmark" }).clicked() {
                            run("bookmark.toggle", json!({ "id": id.as_text() }));
                            close = true;
                        }
                        ui.menu_button("Add to Collection", |ui| {
                            for c in crate::collections::all(&s.project).into_iter().filter(|c| c.search.is_none()) {
                                if ui.button(&c.name).clicked() {
                                    run("collection.add", json!({ "name": c.name, "items": [id.as_text()] }));
                                    close = true;
                                }
                            }
                            if ui.button("New Collection").clicked() {
                                let names: Vec<String> = crate::collections::all(&s.project).into_iter().map(|c| c.name).collect();
                                let name = (1..).map(|n| format!("Collection {n}")).find(|n| !names.contains(n)).unwrap_or_default();
                                run("collection.new", json!({ "name": name, "items": [id.as_text()] }));
                                close = true;
                            }
                        });
                    }
                    ui.separator();
                    if in_trash {
                        if ui.button("Restore").clicked() {
                            run("binder.restore", json!({ "id": id.as_text() }));
                            close = true;
                        }
                    } else if ui.button("Move to Trash").clicked() {
                        run("binder.trash", json!({ "id": id.as_text() }));
                        close = true;
                    }
                }
            }
        });
    });
    for (id, v) in todo {
        if id == "confirm-empty" {
            view.confirm_empty = true;
        } else if id == "ask-media" {
            let inside = if let Node::Item(i) = node { Some(i) } else { None };
            view.add_media = Some((v == json!(true), String::new(), inside));
        } else if let Err(e) = s.run(id, &v) {
            errors.push(e);
        }
    }
    if let (Some(title), Node::Item(id)) = (rename, node) {
        view.tree.rename(Node::Item(id), &title);
    }
    let clicked_away = ui.input(|i| i.pointer.any_pressed()) && !area.response.contains_pointer();
    if close || clicked_away || ui.input(|i| i.key_pressed(egui::Key::Escape)) {
        view.menu = None;
    }
}

fn add_media(ui: &mut Ui, s: &mut Session, view: &mut BinderView, errors: &mut Vec<String>) {
    let Some((link, text, inside)) = view.add_media.as_mut() else { return };
    let mut done = false;
    egui::Window::new(if *link { "Add Web Link" } else { "Add Image or PDF" })
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ui.ctx(), |ui| {
            let label = if *link { "Web address" } else { "File (PNG, JPEG, GIF, WebP or PDF)" };
            let l = ui.label(label);
            let r = ui.add(egui::TextEdit::singleline(text).desired_width(360.0)).labelled_by(l.id);
            let enter = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            ui.horizontal(|ui| {
                if ui.button("Add").clicked() || enter {
                    let mut v = if *link { json!({ "address": text.trim() }) } else { json!({ "path": text.trim() }) };
                    if let Some(i) = inside {
                        v["inside"] = json!(i.as_text());
                    }
                    match s.run(if *link { "link.new" } else { "media.import" }, &v) {
                        Ok(out) => {
                            view.selected = out["id"].as_str().and_then(|i| i.parse().ok());
                            done = true;
                        }
                        Err(e) => errors.push(e),
                    }
                }
                if ui.button("Cancel").clicked() {
                    done = true;
                }
            });
        });
    if done {
        view.add_media = None;
    }
}

fn confirm_empty(ui: &mut Ui, s: &mut Session, view: &mut BinderView, errors: &mut Vec<String>) {
    if !view.confirm_empty {
        return;
    }
    let ids: Vec<ItemId> = s.project.binder.rows_of(Area::Trash).iter().map(|r| r.item.id).collect();
    let words: usize = ids.iter().map(|i| s.project.item_words(*i)).sum();
    egui::Window::new("Empty Trash").collapsible(false).resizable(false).anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0]).show(ui.ctx(), |ui| {
        ui.label(format!(
            "Remove {} item{} and {} word{} for good? This cannot be undone once the project is saved.",
            ids.len(),
            if ids.len() == 1 { "" } else { "s" },
            format_words(words),
            if words == 1 { "" } else { "s" }
        ));
        ui.horizontal(|ui| {
            if ui.button("Empty Trash").clicked() {
                if let Err(e) = s.run("binder.emptyTrash", &json!({ "confirm": true })) {
                    errors.push(e);
                }
                view.confirm_empty = false;
            }
            if ui.button("Cancel").clicked() {
                view.confirm_empty = false;
            }
        });
    });
}
