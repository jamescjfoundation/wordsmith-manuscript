// SPDX-License-Identifier: MIT OR Apache-2.0
//! The freeform board (PLN-3): cards anywhere on a canvas, with lines between them, drawn with
//! the core's [`jcj_board::Canvas`]. A card is a free note, or stands for a binder item and then
//! shows that item's title and synopsis as they are now; double-click opens the item.
//!
//! One board per project, kept in the project settings (`[extra] board`: the core board's cards,
//! links and grid, and which card stands for which item). Every change is a command and one
//! undo step.

use std::collections::BTreeMap;

use jcj_board::{Arrange, Board, BoardEvent, Canvas, CanvasState, CardEdit, CardId, LinkStyle, Selection, card_window};
use jcj_shell::egui::{self, Ui};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use wordsmith_manuscript_model::{Area, ItemId, Project};

use crate::commands::{Spec, id_param, opt_str};
use crate::session::{Outcome, Session};

/// The settings key.
pub const KEY: &str = "board";

/// What is stored.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Stored {
    /// The cards, links and grid.
    pub board: Board,
    /// The binder item each card stands for, by card number.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub items: BTreeMap<u32, ItemId>,
}

/// The project's board (repaired if the file was edited by hand).
pub fn load(p: &Project) -> Stored {
    let mut s: Stored = p.settings.extra.get(KEY).and_then(|v| serde_json::from_value(v.clone()).ok()).unwrap_or_default();
    s.board = s.board.repaired();
    let ids: Vec<u32> = s.board.cards.iter().map(|c| c.id.0).collect();
    s.items.retain(|c, _| ids.contains(c));
    s
}

/// The board as shown: cards standing for items take the item's title and synopsis now; cards
/// whose item is gone (or in the Trash) are marked.
pub fn shown(p: &Project, st: &Stored) -> Board {
    let mut b = st.board.clone();
    for c in &mut b.cards {
        if let Some(id) = st.items.get(&c.id.0) {
            match p.binder.get(*id).filter(|_| p.binder.area_of(*id) != Some(Area::Trash)) {
                Some(item) => {
                    c.title.clone_from(&item.title);
                    c.body = p.text(*id).map(|t| t.meta.synopsis.clone()).unwrap_or_default();
                }
                None => {
                    c.title = format!("{} (removed)", c.title);
                }
            }
        }
    }
    b
}

fn save(s: &mut Session, label: &str, st: Stored) -> Outcome {
    s.change(label, &[], &[], |p| {
        if st.board.cards.is_empty() && st.board.links.is_empty() {
            p.settings.extra.remove(KEY);
        } else {
            p.settings.extra.insert(KEY.into(), serde_json::to_value(&st).map_err(|e| e.to_string())?);
        }
        Ok(())
    })?;
    Ok(json!({}))
}

/// The board commands.
pub fn specs() -> Vec<Spec> {
    let spec = |id, label, params, about, mutates| Spec { id, label, key: "", params, about, mutates };
    vec![
        spec("board.get", "Get Board", "{}", "The freeform board: cards (number, title, body, color, place, item) and links.", false),
        spec(
            "board.addCard",
            "Add Card",
            r##"{"title"?: string, "body"?: string, "item"?: id, "x"?: n, "y"?: n, "color"?: "#RRGGBB"}"##,
            "Add a card to the board: a free note, or one standing for a binder item. Returns its number.",
            true,
        ),
        spec(
            "board.updateCard",
            "Change Card",
            r##"{"card": n, "title"?: string, "body"?: string, "x"?: n, "y"?: n, "color"?: "#RRGGBB"}"##,
            "Change a card's text (free notes only), place or color.",
            true,
        ),
        spec("board.deleteCards", "Delete Cards", r#"{"cards": [n]}"#, "Take cards off the board (items they stand for stay in the binder).", true),
        spec("board.link", "Link Cards", r#"{"from": n, "to": n, "style"?: "line"|"forward"|"back"|"both"}"#, "Draw a line between two cards.", true),
        spec("board.unlink", "Unlink Cards", r#"{"from": n, "to": n}"#, "Remove the line between two cards.", true),
        spec(
            "board.arrange",
            "Arrange Cards",
            r#"{"cards"?: [n], "how": "grid"|"column"|"row"|"cascade"}"#,
            "Line cards up (all of them when none are given).",
            true,
        ),
    ]
}

fn num(v: &Value, key: &str) -> Result<u32, String> {
    v.get(key).and_then(Value::as_u64).and_then(|n| u32::try_from(n).ok()).ok_or_else(|| format!("`{key}` (a card number) is required"))
}

fn f(v: &Value, key: &str) -> Option<f32> {
    v.get(key).and_then(Value::as_f64).map(|x| x as f32)
}

/// Run a board command, or `None` if `id` is not one.
pub fn run(s: &mut Session, id: &str, v: &Value) -> Option<Outcome> {
    Some(match id {
        "board.get" => {
            let st = load(&s.project);
            let b = shown(&s.project, &st);
            Ok(json!({
                "cards": b.cards.iter().map(|c| json!({
                    "card": c.id.0, "title": c.title, "body": c.body, "color": c.color, "x": c.x, "y": c.y,
                    "item": st.items.get(&c.id.0).map(|i| i.as_text()),
                })).collect::<Vec<_>>(),
                "links": b.links.iter().map(|l| json!({ "from": l.from.0, "to": l.to.0 })).collect::<Vec<_>>(),
            }))
        }
        "board.addCard" => (|| {
            let mut st = load(&s.project);
            let item = match opt_str(v, "item") {
                Some(_) => {
                    let i = id_param(v, "item")?;
                    let it = s.project.binder.get(i).ok_or_else(|| format!("no binder item {i}"))?;
                    if st.items.values().any(|x| *x == i) {
                        return Err(format!("“{}” is already on the board", it.title));
                    }
                    Some((i, it.title.clone()))
                }
                None => None,
            };
            let title = item.as_ref().map_or_else(|| opt_str(v, "title").unwrap_or("").to_owned(), |(_, t)| t.clone());
            let card = st.board.add_card(&title, opt_str(v, "body").unwrap_or("")).map_err(|e| e.to_string())?;
            if let (Some(x), Some(y)) = (f(v, "x"), f(v, "y")) {
                st.board.set_position(card, x, y).map_err(|e| e.to_string())?;
            }
            if let Some(c) = opt_str(v, "color") {
                st.board.set_color(card, c).map_err(|e| e.to_string())?;
            }
            if let Some((i, _)) = item {
                st.items.insert(card.0, i);
            }
            save(s, "Add Card", st)?;
            Ok(json!({ "card": card.0 }))
        })(),
        "board.updateCard" => (|| {
            let mut st = load(&s.project);
            let card = CardId(num(v, "card")?);
            let c = st.board.card(card).ok_or_else(|| format!("no card {}", card.0))?.clone();
            if (v.get("title").is_some() || v.get("body").is_some()) && st.items.contains_key(&card.0) {
                return Err("this card shows a binder item: change the item's title or synopsis instead".into());
            }
            let title = opt_str(v, "title").unwrap_or(&c.title).to_owned();
            let body = opt_str(v, "body").unwrap_or(&c.body).to_owned();
            st.board.set_text(card, &title, &body).map_err(|e| e.to_string())?;
            if f(v, "x").is_some() || f(v, "y").is_some() {
                st.board.set_position(card, f(v, "x").unwrap_or(c.x), f(v, "y").unwrap_or(c.y)).map_err(|e| e.to_string())?;
            }
            if let Some(col) = opt_str(v, "color") {
                st.board.set_color(card, col).map_err(|e| e.to_string())?;
            }
            save(s, "Change Card", st)
        })(),
        "board.deleteCards" => (|| {
            let mut st = load(&s.project);
            let list = v.get("cards").and_then(Value::as_array).ok_or("`cards` (a list of numbers) is required")?;
            let mut n = 0;
            for c in list.iter().filter_map(Value::as_u64).filter_map(|c| u32::try_from(c).ok()) {
                if st.board.remove(CardId(c)).is_some() {
                    st.items.remove(&c);
                    n += 1;
                }
            }
            if n == 0 {
                return Err("none of those cards is on the board".into());
            }
            save(s, "Delete Cards", st)
        })(),
        "board.link" => (|| {
            let mut st = load(&s.project);
            let style = match opt_str(v, "style").unwrap_or("line") {
                "line" => LinkStyle::Line,
                "forward" => LinkStyle::Forward,
                "back" => LinkStyle::Back,
                "both" => LinkStyle::Both,
                other => return Err(format!("`{other}` is not a line style (line, forward, back, both)")),
            };
            st.board.link(CardId(num(v, "from")?), CardId(num(v, "to")?), style).map_err(|e| e.to_string())?;
            save(s, "Link Cards", st)
        })(),
        "board.unlink" => (|| {
            let mut st = load(&s.project);
            if !st.board.unlink(CardId(num(v, "from")?), CardId(num(v, "to")?)) {
                return Err("those cards are not linked".into());
            }
            save(s, "Unlink Cards", st)
        })(),
        "board.arrange" => (|| {
            let mut st = load(&s.project);
            let how = match opt_str(v, "how").unwrap_or("grid") {
                "grid" => Arrange::Grid,
                "column" => Arrange::Column,
                "row" => Arrange::Row,
                "cascade" => Arrange::Cascade,
                other => return Err(format!("`{other}` is not a way to arrange (grid, column, row, cascade)")),
            };
            let ids: Vec<CardId> = match v.get("cards").and_then(Value::as_array) {
                Some(l) => l.iter().filter_map(Value::as_u64).filter_map(|c| u32::try_from(c).ok()).map(CardId).collect(),
                None => st.board.cards.iter().map(|c| c.id).collect(),
            };
            st.board.arrange(&ids, how);
            save(s, "Arrange Cards", st)
        })(),
        _ => return None,
    })
}

/// The board view's state.
#[derive(Default)]
pub struct BoardView {
    canvas: CanvasState,
    selection: Selection,
    edit: Option<CardEdit>,
}

/// What the board asks of the app.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ask {
    /// Open this item in the editor.
    Open(ItemId),
}

/// Draw the board. `selected` is the binder's selection (for "Add the Selected Item").
pub fn view(ui: &mut Ui, s: &mut Session, v: &mut BoardView, selected: Option<ItemId>) -> (Vec<Ask>, Vec<String>) {
    let mut asks = Vec::new();
    let mut errors = Vec::new();
    let st = load(&s.project);
    let mut run = |s: &mut Session, cmd: &str, args: Value| {
        if let Err(e) = s.run(cmd, &args) {
            errors.push(e);
        }
    };
    ui.horizontal(|ui| {
        ui.heading("Board");
        if ui.button("Add Card").clicked() {
            run(s, "board.addCard", json!({ "title": "New card" }));
        }
        let on_board = selected.is_some_and(|i| st.items.values().any(|x| *x == i));
        if ui.add_enabled(selected.is_some() && !on_board, egui::Button::new("Add the Selected Item")).clicked()
            && let Some(i) = selected
        {
            run(s, "board.addCard", json!({ "item": i.as_text() }));
        }
        if ui.button("Arrange").on_hover_text("Line the selected cards (or all) up in a grid").clicked() {
            let ids = v.selection.ids();
            let args = if ids.is_empty() {
                json!({ "how": "grid" })
            } else {
                json!({ "how": "grid", "cards": ids.iter().map(|c| c.0).collect::<Vec<_>>() })
            };
            run(s, "board.arrange", args);
        }
    });
    let st = load(&s.project);
    let board = shown(&s.project, &st);
    let events = Canvas::new("manuscript-board")
        .links(true)
        .empty_hint("Double-click to add a card. Drag from a card's edge to another card to link them.")
        .show(ui, &board, &v.selection, &mut v.canvas);
    for e in events {
        match e {
            BoardEvent::Select(c) => v.selection.select(c),
            BoardEvent::Toggle(c) => v.selection.toggle(c),
            BoardEvent::Open(c) => match st.items.get(&c.0) {
                Some(i) => asks.push(Ask::Open(*i)),
                None => v.edit = board.card(c).map(CardEdit::of),
            },
            BoardEvent::Moved { id, x, y } => run(s, "board.updateCard", json!({ "card": id.0, "x": x, "y": y })),
            BoardEvent::AddAt { x, y } => {
                let w = jcj_board::CARD_W;
                let h = jcj_board::CARD_H;
                run(s, "board.addCard", json!({ "title": "New card", "x": (x - w / 2.0).max(0.0), "y": (y - h / 2.0).max(0.0) }));
            }
            BoardEvent::Link { from, to } => run(s, "board.link", json!({ "from": from.0, "to": to.0 })),
            BoardEvent::Delete(cards) => {
                run(s, "board.deleteCards", json!({ "cards": cards.iter().map(|c| c.0).collect::<Vec<_>>() }));
                v.selection.retain(|c| !cards.contains(&c));
            }
            BoardEvent::Reorder { .. } => {}
        }
    }
    if let Some(done) = card_window(ui.ctx(), &mut v.edit) {
        run(s, "board.updateCard", json!({ "card": done.id.0, "title": done.title, "body": done.body }));
    }
    (asks, errors)
}
