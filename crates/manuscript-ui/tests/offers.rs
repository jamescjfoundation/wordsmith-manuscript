// SPDX-License-Identifier: MIT OR Apache-2.0
//! Fixes offered back by Book: read from the file beside the project, accepted (one undo step,
//! formatting kept) or rejected, stale when the text moved on; decisions kept for Book.

use serde_json::json;
use wordsmith_manuscript_format::dialect;
use wordsmith_manuscript_format::offers::{self, Decision, Offer, OffersFile};
use wordsmith_manuscript_model::{Area, Item, ItemId, ItemText, Kind, Place, Project};
use wordsmith_manuscript_ui::session::Session;

fn id(n: u64) -> ItemId {
    ItemId::from_counter(n)
}

fn offer(oid: &str, find: &str, replace: &str) -> Offer {
    Offer { id: oid.into(), item: id(1), find: find.into(), replace: replace.into(), note: "typo".into(), made: "2026-11-24".into() }
}

#[test]
fn offers_are_accepted_rejected_or_stale() {
    let root = std::env::temp_dir().join(format!("wsm-offers-{}", std::process::id())).join("Novel.wsm");
    let _ = std::fs::remove_dir_all(root.parent().unwrap());
    let mut p = Project::new();
    p.binder.insert(Place::end_of(Area::Manuscript), Item::with_id(id(1), Kind::Text, "Scene")).unwrap();
    p.texts.insert(id(1), ItemText::with_body(dialect::parse("She walked *teh* harbour wall.\n\nThe lamp was lit.\n")));
    let mut s = Session::create(&root, p).unwrap();
    assert!(s.run("offers.list", &json!({})).unwrap()["offers"].as_array().unwrap().is_empty(), "no file: no offers");
    // Book writes its offers beside the project folder.
    let file = OffersFile { version: 1, offers: vec![offer("a", "teh", "the"), offer("b", "lamp", "lantern"), offer("c", "boat", "ship")] };
    std::fs::write(offers::offers_path(&root), offers::write(&file).unwrap()).unwrap();
    let list = s.run("offers.list", &json!({})).unwrap();
    let states: Vec<&str> = list["offers"].as_array().unwrap().iter().map(|o| o["state"].as_str().unwrap()).collect();
    assert_eq!(states, ["pending", "pending", "stale"]);
    s.run("offers.accept", &json!({ "id": "a" })).unwrap();
    assert_eq!(
        dialect::write(&s.project.text(id(1)).unwrap().body),
        "She walked *the* harbour wall.\n\nThe lamp was lit.\n",
        "the new words keep the formatting"
    );
    assert!(s.run("offers.accept", &json!({ "id": "a" })).is_err(), "decided once");
    assert!(s.run("offers.accept", &json!({ "id": "c" })).is_err(), "stale offers cannot be taken");
    s.run("offers.reject", &json!({ "id": "b" })).unwrap();
    assert_eq!(offers::decisions(&s.project).get("a"), Some(&Decision::Accepted));
    assert_eq!(offers::decisions(&s.project).get("b"), Some(&Decision::Rejected));
    assert_eq!(wordsmith_manuscript_ui::offers::pending(&s), 0);
    s.undo();
    s.undo();
    assert_eq!(dialect::write(&s.project.text(id(1)).unwrap().body), "She walked *teh* harbour wall.\n\nThe lamp was lit.\n");
    assert!(offers::decisions(&s.project).is_empty(), "undo takes the decisions back too");
    // Decisions are saved with the project, where Book reads them.
    s.run("offers.accept", &json!({ "id": "a" })).unwrap();
    s.save().unwrap();
    let back = Session::open(&root).unwrap();
    assert_eq!(offers::decisions(&back.project).get("a"), Some(&Decision::Accepted));
    // Book never wrote inside the project folder.
    assert!(!root.join("offers").exists());
    let _ = std::fs::remove_dir_all(root.parent().unwrap());
}
