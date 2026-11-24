// SPDX-License-Identifier: MIT OR Apache-2.0
//! Counts and targets.

use serde_json::json;
use wordsmith_manuscript_model::{Area, Item, ItemId, ItemText, Kind, Place, Project, Text};
use wordsmith_manuscript_ui::progress::{self, Progress};
use wordsmith_manuscript_ui::session::Session;

fn session() -> Session {
    let mut p = Project::new();
    let ch = Item::with_id(ItemId::from_counter(1), Kind::Folder, "One");
    let a = Item::with_id(ItemId::from_counter(2), Kind::Text, "A");
    let mut b = Item::with_id(ItemId::from_counter(3), Kind::Text, "B");
    b.include = false;
    p.binder.insert(Place::end_of(Area::Manuscript), ch.with_children(vec![a, b])).unwrap();
    p.texts.insert(ItemId::from_counter(2), ItemText::with_body(Text::from_paragraphs(&["one two three", "four"])));
    p.texts.insert(ItemId::from_counter(3), ItemText::with_body(Text::from_paragraphs(&["left out"])));
    Session::new(p)
}

#[test]
fn counts_for_items_folders_and_the_manuscript() {
    let mut s = session();
    let c = s.run("counts.get", &json!({ "id": ItemId::from_counter(1).as_text() })).unwrap();
    assert_eq!(c["words"], 6, "a folder counts everything under it");
    let c = s.run("counts.get", &json!({})).unwrap();
    assert_eq!(c["words"], 4, "the manuscript counts only what is included");
    assert_eq!(c["characters"], 17);
}

#[test]
fn targets_deadlines_and_sessions() {
    let mut s = session();
    let today = progress::today();
    let deadline = today.plus(9);
    s.run("targets.set", &json!({ "manuscript": 1004, "deadline": deadline.iso(), "writing_days": [], "session": 10 })).unwrap();
    // 1,000 words left over 10 days.
    assert_eq!(progress::daily_need(&s.project, today), Some(100));
    assert!(s.run("targets.set", &json!({ "deadline": "next week" })).is_err());
    assert!(s.run("targets.set", &json!({ "writing_days": ["someday"] })).is_err());
    let mut pr = Progress::start(&s.project);
    s.run("text.set", &json!({ "id": ItemId::from_counter(2).as_text(), "text": "one two three\n\nfour five six seven\n" })).unwrap();
    pr.observe(&s);
    assert_eq!(pr.writing.session_words(), 3);
    assert_eq!(pr.writing.progress(), Some(0.3));
    s.undo();
    assert!(s.project.settings.targets.manuscript.is_some());
}
