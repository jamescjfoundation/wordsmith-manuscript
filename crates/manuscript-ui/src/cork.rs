// SPDX-License-Identifier: MIT OR Apache-2.0
//! The corkboard (PLN-1, SPEC 7.1): a folder's children as index cards through `jcj-board`.
//! A card shows the item's title, its synopsis, its label's color and its status. Dragging a
//! card reorders the binder; a new card is a new item; double-clicking edits title and synopsis.

use std::collections::HashSet;

use jcj_board::{Board, BoardEvent, Card, CardEdit, CardId, Corkboard, Selection, card_window};
use jcj_shell::egui::Ui;
use serde_json::json;
use wordsmith_manuscript_model::{Area, ItemId, Project};

use crate::outline::Ask;
use crate::session::Session;

/// The color of a card with no label.
pub const PLAIN: &str = "#FFF2A8";

/// The items a corkboard of `root` shows: its children (the Manuscript's top level without one).
pub fn items(p: &Project, root: Option<ItemId>, hidden: &HashSet<ItemId>) -> Vec<ItemId> {
    let kids: Vec<ItemId> = match root.and_then(|r| p.binder.get(r)) {
        Some(item) => item.children.iter().map(|c| c.id).collect(),
        None => p.binder.area(Area::Manuscript).iter().map(|c| c.id).collect(),
    };
    kids.into_iter().filter(|i| !hidden.contains(i)).collect()
}

/// A board of the items, card `n` for item `n`.
pub fn board(p: &Project, items: &[ItemId]) -> Board {
    let mut b = Board::default();
    for (n, id) in items.iter().enumerate() {
        let Some(item) = p.binder.get(*id) else { continue };
        let meta = p.text(*id).map(|t| t.meta.clone()).unwrap_or_default();
        let color = item.label.as_deref().and_then(|l| p.settings.label_color(l)).filter(|c| jcj_board::is_color(c)).unwrap_or(PLAIN);
        let card = Card {
            id: CardId(n as u32 + 1),
            title: if item.title.is_empty() { "Untitled".into() } else { item.title.clone() },
            body: meta.synopsis,
            color: color.to_owned(),
            label: item.status.clone().unwrap_or_default(),
            ..Card::default()
        };
        let _ = b.insert(card);
    }
    b
}

fn item_of(items: &[ItemId], card: CardId) -> Option<ItemId> {
    items.get((card.0 as usize).checked_sub(1)?).copied()
}

/// The corkboard's state.
#[derive(Default)]
pub struct CorkView {
    selection: Selection,
    edit: Option<(CardEdit, ItemId)>,
}

/// Draw the corkboard of `root`.
pub fn view(
    ui: &mut Ui,
    s: &mut Session,
    v: &mut CorkView,
    root: Option<ItemId>,
    hidden: &HashSet<ItemId>,
    selected: Option<ItemId>,
) -> (Vec<Ask>, Vec<String>) {
    let mut asks = Vec::new();
    let mut errors = Vec::new();
    let items = items(&s.project, root, hidden);
    let b = board(&s.project, &items);
    let title = root.and_then(|r| s.project.binder.get(r)).map_or_else(|| "Manuscript".to_owned(), |i| i.title.clone());
    ui.horizontal(|ui| {
        ui.heading(title);
        if ui.button("New Card").clicked() {
            let params = match root {
                Some(r) => json!({ "inside": r.as_text() }),
                None => json!({ "area": "manuscript" }),
            };
            match s.run("binder.new", &params) {
                Ok(out) => {
                    if let Some(id) = out["id"].as_str().and_then(|x| x.parse().ok()) {
                        asks.push(Ask::Select(id));
                    }
                }
                Err(e) => errors.push(e),
            }
        }
    });
    v.selection.select(selected.and_then(|sel| items.iter().position(|i| *i == sel)).map(|n| CardId(n as u32 + 1)));
    let events = Corkboard::new("manuscript-corkboard").empty_hint("No cards here yet: New Card adds one.").show(ui, &b, &v.selection);
    for e in events {
        let r = match e {
            BoardEvent::Select(Some(c)) | BoardEvent::Toggle(c) => {
                if let Some(id) = item_of(&items, c) {
                    asks.push(Ask::Select(id));
                }
                Ok(())
            }
            BoardEvent::Select(None) => Ok(()),
            BoardEvent::Open(c) => {
                if let (Some(id), Some(card)) = (item_of(&items, c), b.card(c)) {
                    v.edit = Some((CardEdit::of(card), id));
                }
                Ok(())
            }
            BoardEvent::Reorder { from, before } => match items.get(from).copied() {
                Some(id) => {
                    let Some(place) = s.project.binder.place_of(id) else { continue };
                    // `before` counts cards; the binder's index counts all siblings.
                    let index = match before.and_then(|b| items.get(b)) {
                        Some(t) => s.project.binder.place_of(*t).map_or(usize::MAX, |p| p.index),
                        None => usize::MAX,
                    };
                    let mut p = json!({ "id": id.as_text(), "area": place.area.key() });
                    if let Some(parent) = place.parent {
                        p["parent"] = json!(parent.as_text());
                    }
                    if index != usize::MAX {
                        p["index"] = json!(index);
                    }
                    s.run("binder.move", &p).map(|_| ())
                }
                None => Ok(()),
            },
            BoardEvent::Delete(cards) => {
                let mut r = Ok(());
                for c in cards {
                    if let Some(id) = item_of(&items, c) {
                        r = r.and(s.run("binder.trash", &json!({ "id": id.as_text() })).map(|_| ()));
                    }
                }
                r
            }
            BoardEvent::Moved { .. } | BoardEvent::AddAt { .. } | BoardEvent::Link { .. } => Ok(()),
        };
        if let Err(e) = r {
            errors.push(e);
        }
    }
    let mut editing = v.edit.as_ref().map(|(e, _)| e.clone());
    if let Some(done) = card_window(ui.ctx(), &mut editing)
        && let Some((_, id)) = v.edit.as_ref()
    {
        let id = id.as_text();
        if let Err(e) = s.run("binder.rename", &json!({ "id": id, "title": done.title })) {
            errors.push(e);
        }
        if let Err(e) = s.run("item.setSynopsis", &json!({ "id": id, "synopsis": done.body })) {
            errors.push(e);
        }
    }
    match editing {
        Some(e) => {
            if let Some((slot, _)) = v.edit.as_mut() {
                *slot = e;
            }
        }
        None => v.edit = None,
    }
    (asks, errors)
}
