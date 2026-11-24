// SPDX-License-Identifier: MIT OR Apache-2.0
//! The freeform board (PLN-3): free cards and cards standing for items, lines between them,
//! every change one undo step, kept with the project; the canvas reports gestures as commands.

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use jcj_shell::egui;
use serde_json::json;
use wordsmith_manuscript_model::{Area, Item, ItemId, ItemText, Kind, Place, Project, Text};
use wordsmith_manuscript_ui::board::{self, BoardView};
use wordsmith_manuscript_ui::session::Session;

fn id(n: u64) -> ItemId {
    ItemId::from_counter(n)
}

fn session() -> Session {
    let mut p = Project::new();
    p.binder.insert(Place::end_of(Area::Manuscript), Item::with_id(id(1), Kind::Text, "The Storm")).unwrap();
    let mut t = ItemText::with_body(Text::from_paragraphs(&["Words."]));
    t.meta.synopsis = "The boat is lost.".into();
    p.texts.insert(id(1), t);
    Session::new(p)
}

#[test]
fn cards_links_and_items() {
    let mut s = session();
    let a = s.run("board.addCard", &json!({ "title": "Why does she stay?", "body": "Think about it.", "x": 40, "y": 60 })).unwrap()["card"]
        .as_u64()
        .unwrap();
    let b = s.run("board.addCard", &json!({ "item": id(1).as_text() })).unwrap()["card"].as_u64().unwrap();
    assert!(s.run("board.addCard", &json!({ "item": id(1).as_text() })).is_err(), "an item is on the board once");
    s.run("board.link", &json!({ "from": a, "to": b, "style": "forward" })).unwrap();
    assert!(s.run("board.link", &json!({ "from": a, "to": a })).is_err());
    let got = s.run("board.get", &json!({})).unwrap();
    assert_eq!(got["cards"][1]["title"], "The Storm");
    assert_eq!(got["cards"][1]["body"], "The boat is lost.");
    assert_eq!(got["cards"][1]["item"], id(1).as_text());
    assert_eq!(got["links"].as_array().unwrap().len(), 1);
    // A card standing for an item follows it.
    s.run("binder.rename", &json!({ "id": id(1).as_text(), "title": "The Gale" })).unwrap();
    assert_eq!(s.run("board.get", &json!({})).unwrap()["cards"][1]["title"], "The Gale");
    assert!(s.run("board.updateCard", &json!({ "card": b, "title": "x" })).is_err());
    s.run("board.updateCard", &json!({ "card": a, "x": 400, "y": 100, "color": "#88AACC" })).unwrap();
    let got = s.run("board.get", &json!({})).unwrap();
    assert_eq!((got["cards"][0]["x"].as_f64(), got["cards"][0]["color"].as_str()), (Some(400.0), Some("#88AACC")));
    s.undo();
    assert_eq!(s.run("board.get", &json!({})).unwrap()["cards"][0]["x"].as_f64(), Some(40.0));
    // Deleting a card leaves the item and drops its lines.
    s.run("board.deleteCards", &json!({ "cards": [b] })).unwrap();
    let got = s.run("board.get", &json!({})).unwrap();
    assert_eq!(got["cards"].as_array().unwrap().len(), 1);
    assert!(got["links"].as_array().unwrap().is_empty());
    assert!(s.project.binder.contains(id(1)));
    s.undo();
    s.run("board.arrange", &json!({ "how": "row" })).unwrap();
    // Saved and read back.
    let dir = std::env::temp_dir().join(format!("wsm-board-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    s.save_as(&dir).unwrap();
    let back = Session::open(&dir).unwrap();
    assert_eq!(board::load(&back.project), board::load(&s.project));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_board_view_adds_the_selected_item() {
    struct St {
        s: Session,
        v: BoardView,
        styled: bool,
    }
    let mut h = Harness::builder().with_size(egui::vec2(900.0, 600.0)).build_ui_state(
        |ui, st: &mut St| {
            if !st.styled {
                wordsmith_manuscript_ui::brand::apply(ui.ctx(), jcj_shell::theme::Mode::Light);
                st.styled = true;
            }
            let _ = board::view(ui, &mut st.s, &mut st.v, Some(id(1)));
        },
        St { s: session(), v: BoardView::default(), styled: false },
    );
    h.run();
    h.get_by_label("Add the Selected Item").click();
    h.run();
    h.get_by_label("Add Card").click();
    h.run();
    let got = h.state_mut().s.run("board.get", &json!({})).unwrap();
    assert_eq!(got["cards"].as_array().unwrap().len(), 2);
    assert!(h.query_by_label_contains("The Storm").is_some());
}
