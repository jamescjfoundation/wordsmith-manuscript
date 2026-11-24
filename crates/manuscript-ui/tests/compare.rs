// SPDX-License-Identifier: MIT OR Apache-2.0
//! Snapshot compare (INS-5): word by word with the current text or another snapshot, from the
//! inspector into a window.

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use jcj_shell::egui;
use serde_json::json;
use wordsmith_manuscript_model::{Area, Item, ItemId, ItemText, Kind, Place, Project, Text};
use wordsmith_manuscript_ui::session::Session;
use wordsmith_manuscript_ui::snapshots::{self, CompareView};

fn id(n: u64) -> ItemId {
    ItemId::from_counter(n)
}

fn session() -> (Session, String) {
    let mut p = Project::new();
    p.binder.insert(Place::end_of(Area::Manuscript), Item::with_id(id(1), Kind::Text, "Arrival")).unwrap();
    p.texts.insert(id(1), ItemText::with_body(Text::from_paragraphs(&["The keeper came at dusk.", "Rain fell."])));
    let mut s = Session::new(p);
    let taken = s.run("snapshot.take", &json!({ "id": id(1).as_text(), "name": "First" })).unwrap()["taken"].as_str().unwrap().to_owned();
    s.run("text.set", &json!({ "id": id(1).as_text(), "text": "The old keeper came at dawn.\n\nRain fell.\n\nThe lamp was lit.\n" })).unwrap();
    (s, taken)
}

#[test]
fn a_snapshot_compares_word_by_word() {
    let (mut s, taken) = session();
    let out = s.run("snapshot.compare", &json!({ "id": id(1).as_text(), "taken": taken })).unwrap();
    assert_eq!(out["added"], 6, "old, dawn, and The lamp was lit");
    assert_eq!(out["removed"], 1, "dusk");
    let first = out["paragraphs"][0].as_array().unwrap();
    assert!(first.contains(&json!({ "change": "added", "text": "old " })));
    assert!(first.contains(&json!({ "change": "removed", "text": "dusk" })));
    assert_eq!(out["paragraphs"][1], json!([{ "change": "same", "text": "Rain fell." }]));
    // With itself: the same.
    assert_eq!(s.run("snapshot.compare", &json!({ "id": id(1).as_text(), "taken": taken, "with": taken })).unwrap()["same"], true);
    assert!(s.run("snapshot.compare", &json!({ "id": id(1).as_text(), "taken": "nope" })).is_err());
}

#[test]
fn the_compare_window_shows_the_changes() {
    let (s, taken) = session();
    struct St {
        s: Session,
        v: CompareView,
        styled: bool,
    }
    let mut h = Harness::builder().with_size(egui::vec2(700.0, 600.0)).build_ui_state(
        |ui, st: &mut St| {
            if !st.styled {
                wordsmith_manuscript_ui::brand::apply(ui.ctx(), jcj_shell::theme::Mode::Light);
                st.styled = true;
            }
            snapshots::compare_window(ui.ctx(), &st.s, &mut st.v);
        },
        St { s, v: CompareView { open: Some((id(1), taken)) }, styled: false },
    );
    h.run();
    assert!(h.query_by_label("6 words added, 1 removed.").is_some());
    assert!(h.query_by_label("The old keeper came at duskdawn.").is_some(), "both versions' words, in order");
}
