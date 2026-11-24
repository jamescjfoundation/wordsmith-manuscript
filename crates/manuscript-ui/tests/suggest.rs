// SPDX-License-Identifier: MIT OR Apache-2.0
//! Suggested bible links (INS-4): entries a scene mentions but does not link, most mentioned
//! first; linking one takes it off the list.

use serde_json::json;
use wordsmith_manuscript_model::{Area, Item, ItemId, ItemText, Kind, Place, Project, Text};
use wordsmith_manuscript_ui::session::Session;

#[test]
fn mentioned_entries_are_suggested() {
    let mut p = Project::new();
    let scene = ItemId::from_counter(1);
    p.binder.insert(Place::end_of(Area::Manuscript), Item::with_id(scene, Kind::Text, "Scene")).unwrap();
    p.texts.insert(scene, ItemText::with_body(Text::from_paragraphs(&["Mara met Jon at the Anchor.", "Mara laughed; Annabel did not."])));
    let mut s = Session::new(p);
    let entry = |s: &mut Session, kind: &str, name: &str| -> String {
        s.run("bible.new", &json!({ "kind": kind, "name": name })).unwrap()["id"].as_str().unwrap().to_owned()
    };
    let mara = entry(&mut s, "character", "Mara");
    let jon = entry(&mut s, "character", "Jonathan");
    s.run("entry.set", &json!({ "id": jon, "aliases": ["Jon"] })).unwrap();
    entry(&mut s, "character", "Anna");
    entry(&mut s, "place", "The Anchor");
    let names = |s: &mut Session| -> Vec<(String, u64)> {
        s.run("item.suggestEntries", &json!({ "id": scene.as_text() }))
            .unwrap()
            .as_array()
            .unwrap()
            .iter()
            .map(|x| (x["name"].as_str().unwrap().to_owned(), x["mentions"].as_u64().unwrap()))
            .collect()
    };
    assert_eq!(
        names(&mut s),
        [("Mara".into(), 2), ("Jonathan".into(), 1)],
        "whole words only: Anna is not in Annabel; \"the Anchor\" is not \"The Anchor\""
    );
    s.run("item.linkEntry", &json!({ "id": scene.as_text(), "entry": mara })).unwrap();
    assert_eq!(names(&mut s), [("Jonathan".into(), 1)]);
}
