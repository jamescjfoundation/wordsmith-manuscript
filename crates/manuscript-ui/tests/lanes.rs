// SPDX-License-Identifier: MIT OR Apache-2.0
//! Thread lanes (PLN-4): scenes in story order, one lane per plot thread or point of view;
//! moving a scene to another lane relinks it, as one undo step.

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use jcj_shell::egui;
use serde_json::json;
use wordsmith_manuscript_model::{Area, Item, ItemId, ItemText, Kind, Place, Project, Role, Text};
use wordsmith_manuscript_ui::lanes::{self, LanesView};
use wordsmith_manuscript_ui::session::Session;

fn id(n: u64) -> ItemId {
    ItemId::from_counter(n)
}

fn session() -> (Session, ItemId, ItemId) {
    let mut p = Project::new();
    let ch = Item::with_id(id(10), Kind::Folder, "One")
        .role(Role::Chapter)
        .with_children((1..=3).map(|n| Item::with_id(id(n), Kind::Text, format!("Scene {n}")).role(Role::Scene)).collect());
    p.binder.insert(Place::end_of(Area::Manuscript), ch).unwrap();
    for n in 1..=3 {
        p.texts.insert(id(n), ItemText::with_body(Text::from_paragraphs(&["Words."])));
    }
    let mut s = Session::new(p);
    let heist: ItemId = s.run("bible.new", &json!({ "kind": "thread", "name": "The Heist" })).unwrap()["id"].as_str().unwrap().parse().unwrap();
    let love: ItemId = s.run("bible.new", &json!({ "kind": "thread", "name": "The Romance" })).unwrap()["id"].as_str().unwrap().parse().unwrap();
    s.run("item.linkEntry", &json!({ "id": id(1).as_text(), "entry": heist.as_text() })).unwrap();
    s.run("item.linkEntry", &json!({ "id": id(3).as_text(), "entry": heist.as_text() })).unwrap();
    s.run("item.linkEntry", &json!({ "id": id(3).as_text(), "entry": love.as_text() })).unwrap();
    (s, heist, love)
}

#[test]
fn lanes_by_thread_and_point_of_view() {
    let (mut s, heist, love) = session();
    let got = s.run("lanes.get", &json!({})).unwrap();
    let lanes: Vec<(String, Vec<u64>)> = got["lanes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| (l["title"].as_str().unwrap().to_owned(), l["scenes"].as_array().unwrap().iter().map(|x| x.as_u64().unwrap()).collect()))
        .collect();
    assert_eq!(lanes, [("The Heist".into(), vec![0, 2]), ("The Romance".into(), vec![2]), ("No thread".into(), vec![1])]);
    // Scene 2 joins the romance; one undo step.
    s.run("lanes.move", &json!({ "id": id(2).as_text(), "to": love.as_text() })).unwrap();
    assert!(s.project.text(id(2)).unwrap().meta.entries.contains(&love));
    s.run("lanes.move", &json!({ "id": id(1).as_text(), "from": heist.as_text(), "to": love.as_text() })).unwrap();
    assert_eq!(s.project.text(id(1)).unwrap().meta.entries, vec![love]);
    s.undo();
    assert_eq!(s.project.text(id(1)).unwrap().meta.entries, vec![heist]);
    // By point of view.
    s.run("item.setField", &json!({ "id": id(1).as_text(), "field": "Point of view", "value": "Mara" })).unwrap();
    s.run("lanes.move", &json!({ "id": id(2).as_text(), "by": "pov", "to": "Jon" })).unwrap();
    let got = s.run("lanes.get", &json!({ "by": "pov" })).unwrap();
    let titles: Vec<&str> = got["lanes"].as_array().unwrap().iter().map(|l| l["title"].as_str().unwrap()).collect();
    assert_eq!(titles, ["Mara", "Jon", "No point of view"]);
    assert!(s.run("lanes.move", &json!({ "id": id(2).as_text(), "to": id(1).as_text() })).is_err(), "only plot threads are lanes");
}

#[test]
fn the_lanes_view_shows_cards() {
    let (s, _, _) = session();
    let mut h = Harness::builder().with_size(egui::vec2(900.0, 500.0)).build_ui_state(
        |ui, st: &mut (Session, LanesView, bool)| {
            if !st.2 {
                wordsmith_manuscript_ui::brand::BRAND.apply(ui.ctx(), jcj_shell::theme::Mode::Light);
                st.2 = true;
            }
            let _ = lanes::view(ui, &mut st.0, &mut st.1);
        },
        (s, LanesView::default(), false),
    );
    h.run();
    assert!(h.query_by_label("Scene 3, in The Romance").is_some());
    assert!(h.query_by_label("Scene 2, in No thread").is_some());
}
