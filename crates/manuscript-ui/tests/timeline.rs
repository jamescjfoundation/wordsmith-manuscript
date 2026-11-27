// SPDX-License-Identifier: MIT OR Apache-2.0
//! The timeline (PLN-5): scenes in the story's own time from their date field, story order kept
//! beside it; undated scenes listed; the view offers a date field when there is none.

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use jcj_shell::egui;
use serde_json::json;
use wordsmith_manuscript_model::{Area, Item, ItemId, ItemText, Kind, Place, Project, Role, Text};
use wordsmith_manuscript_ui::session::Session;
use wordsmith_manuscript_ui::timeline;

fn id(n: u64) -> ItemId {
    ItemId::from_counter(n)
}

fn session() -> Session {
    let mut p = Project::new();
    let ch = Item::with_id(id(10), Kind::Folder, "One").role(Role::Chapter).with_children(
        ["The Storm", "Childhood", "The Wreck", "Later"]
            .iter()
            .enumerate()
            .map(|(n, t)| Item::with_id(id(n as u64 + 1), Kind::Text, *t).role(Role::Scene))
            .collect(),
    );
    p.binder.insert(Place::end_of(Area::Manuscript), ch).unwrap();
    for n in 1..=4 {
        p.texts.insert(id(n), ItemText::with_body(Text::from_paragraphs(&["Words."])));
    }
    Session::new(p)
}

#[test]
fn scenes_in_story_time() {
    let mut s = session();
    assert!(s.run("timeline.get", &json!({})).unwrap()["field"].is_null());
    s.run("project.setFields", &json!({ "fields": [{ "name": "Point of view" }, { "name": "Scene date", "kind": "date" }] })).unwrap();
    for (n, d) in [(1, "1890-10-03"), (2, "1871-05-01"), (3, "1890-10-03")] {
        s.run("item.setField", &json!({ "id": id(n).as_text(), "field": "Scene date", "value": d })).unwrap();
    }
    let got = s.run("timeline.get", &json!({})).unwrap();
    let dated: Vec<(&str, &str, u64)> = got["dated"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| (d["title"].as_str().unwrap(), d["date"].as_str().unwrap(), d["story_order"].as_u64().unwrap()))
        .collect();
    assert_eq!(dated, [("Childhood", "1871-05-01", 2), ("The Storm", "1890-10-03", 1), ("The Wreck", "1890-10-03", 3)]);
    assert_eq!(got["undated"][0]["title"], "Later");
}

#[test]
fn the_view_offers_a_date_field_then_shows_scenes() {
    let mut h = Harness::builder().with_size(egui::vec2(900.0, 500.0)).build_ui_state(
        |ui, st: &mut (Session, bool)| {
            if !st.1 {
                wordsmith_manuscript_ui::brand::BRAND.apply(ui.ctx(), jcj_shell::theme::Mode::Light);
                st.1 = true;
            }
            let _ = timeline::view(ui, &mut st.0);
        },
        (session(), false),
    );
    h.run();
    h.get_by_label("Add a Scene Date Field").click();
    h.run();
    h.state_mut().0.run("item.setField", &json!({ "id": id(2).as_text(), "field": "Scene date", "value": "1871-05-01" })).unwrap();
    h.run();
    assert!(h.query_by_label("Childhood, 1871-05-01, scene 2 of the book").is_some());
    assert!(h.query_by_label("Later").is_some(), "undated scenes are listed");
}
