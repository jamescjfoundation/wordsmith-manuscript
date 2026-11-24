// SPDX-License-Identifier: MIT OR Apache-2.0
//! Series with a shared bible (BND-8, BIB-5): books share one Story Bible kept in its own folder;
//! a change one book saves reaches the others; leaving keeps a copy.

use serde_json::json;
use wordsmith_manuscript_model::{Area, Item, ItemId, ItemText, Kind, Place, Project, Text};
use wordsmith_manuscript_ui::session::Session;

fn book(root: &std::path::Path, scene: &str) -> Session {
    let mut p = Project::new();
    let sid = ItemId::new();
    p.binder.insert(Place::end_of(Area::Manuscript), Item::with_id(sid, Kind::Text, "Scene")).unwrap();
    p.texts.insert(sid, ItemText::with_body(Text::from_paragraphs(&[scene])));
    Session::create(root, p).unwrap()
}

fn names(s: &Session) -> Vec<String> {
    let mut v: Vec<String> = s.project.entries.values().map(|e| e.name.clone()).collect();
    v.sort();
    v
}

#[test]
fn books_share_one_bible() {
    let base = std::env::temp_dir().join(format!("wsm-series-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let (a_root, b_root, series) = (base.join("Book One.wsm"), base.join("Book Two.wsm"), base.join("The Coast.wsm"));
    let mut a = book(&a_root, "Mara walked to the harbour.");
    let mara: ItemId = a.run("bible.new", &json!({ "kind": "character", "name": "Mara" })).unwrap()["id"].as_str().unwrap().parse().unwrap();
    a.save().unwrap();
    assert!(a_root.join(format!("bible/{mara}.md")).exists());
    // Book One starts the series: its bible moves into the series folder.
    a.run("series.create", &json!({ "path": series.to_str(), "name": "The Coast" })).unwrap();
    assert!(series.join(format!("bible/{mara}.md")).exists());
    assert!(!a_root.join(format!("bible/{mara}.md")).exists(), "the book's folder keeps no copy");
    assert_eq!(a.project.settings.series, "../The Coast.wsm");
    let mut reopened = Session::open(&a_root).unwrap();
    assert_eq!(names(&reopened), ["Mara"]);
    let back = reopened.run("bible.backlinks", &json!({ "id": mara.as_text() })).unwrap();
    assert!(back["mentioned"].as_array().is_some_and(|m| !m.is_empty()), "{back}");
    // Book Two joins with an entry of its own; both are in the series now.
    let mut b = book(&b_root, "Jon mended the nets.");
    b.run("bible.new", &json!({ "kind": "character", "name": "Jon" })).unwrap();
    b.save().unwrap();
    b.run("series.join", &json!({ "path": series.to_str() })).unwrap();
    assert_eq!(names(&b), ["Jon", "Mara"]);
    let info = b.run("series.info", &json!({})).unwrap();
    assert_eq!(info["books"], json!(["../Book One.wsm", "../Book Two.wsm"]), "paths from the series folder");
    // Book One sees what Book Two saved.
    b.run("entry.setField", &json!({ "id": mara.as_text(), "field": "Age", "value": "34" })).unwrap();
    b.save().unwrap();
    let o = a.check_outside().unwrap();
    assert!(o.project_reloaded);
    assert_eq!(names(&a), ["Jon", "Mara"]);
    assert_eq!(a.project.entries[&mara].fields["Age"], "34");
    // Book One leaves and keeps a copy.
    a.run("series.leave", &json!({})).unwrap();
    assert!(a_root.join(format!("bible/{mara}.md")).exists());
    assert!(a.project.settings.series.is_empty());
    let info = b.run("series.info", &json!({})).unwrap();
    let _ = info;
    let series_now = Session::open(&series).unwrap();
    assert_eq!(wordsmith_manuscript_model::series::info(&series_now.project).unwrap().books, ["../Book Two.wsm"]);
    let _ = std::fs::remove_dir_all(&base);
}
