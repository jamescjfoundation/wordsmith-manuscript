// SPDX-License-Identifier: MIT OR Apache-2.0
//! Relationships between entries (BIB-4): kept on one entry, read from both sides, saved.

use serde_json::json;
use wordsmith_manuscript_model::{ItemId, Project};
use wordsmith_manuscript_ui::session::Session;

fn entry(s: &mut Session, kind: &str, name: &str) -> ItemId {
    s.run("bible.new", &json!({ "kind": kind, "name": name })).unwrap()["id"].as_str().unwrap().parse().unwrap()
}

#[test]
fn relations_read_both_ways_and_are_kept() {
    let mut s = Session::new(Project::new());
    let mara = entry(&mut s, "character", "Mara");
    let jon = entry(&mut s, "character", "Jon");
    let inn = entry(&mut s, "place", "The Anchor");
    s.run("entry.relate", &json!({ "id": mara.as_text(), "relation": "Sibling of", "to": jon.as_text() })).unwrap();
    s.run("entry.relate", &json!({ "id": jon.as_text(), "relation": "works for", "to": inn.as_text() })).unwrap();
    s.run("entry.relate", &json!({ "id": mara.as_text(), "relation": "owes money to", "to": inn.as_text() })).unwrap();
    assert!(s.run("entry.relate", &json!({ "id": mara.as_text(), "relation": "sibling of", "to": jon.as_text() })).is_err(), "once");
    assert!(s.run("entry.relate", &json!({ "id": mara.as_text(), "relation": "x", "to": mara.as_text() })).is_err());
    let read = |s: &mut Session, id: ItemId| -> Vec<(String, String)> {
        s.run("entry.relations", &json!({ "id": id.as_text() }))
            .unwrap()
            .as_array()
            .unwrap()
            .iter()
            .map(|r| (r["relation"].as_str().unwrap().to_owned(), r["name"].as_str().unwrap().to_owned()))
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect()
    };
    assert_eq!(read(&mut s, jon), [("sibling of".into(), "Mara".into()), ("works for".into(), "The Anchor".into())]);
    assert_eq!(read(&mut s, inn), [("employs".into(), "Jon".into()), ("owes money to (from)".into(), "Mara".into())]);
    // Saved with the entries.
    let dir = std::env::temp_dir().join(format!("wsm-relations-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    s.save_as(&dir).unwrap();
    let mut back = Session::open(&dir).unwrap();
    assert_eq!(read(&mut back, inn).len(), 2);
    back.run("entry.unrelate", &json!({ "id": jon.as_text(), "relation": "works for", "to": inn.as_text() })).unwrap();
    assert_eq!(read(&mut back, inn).len(), 1);
    back.undo();
    assert_eq!(read(&mut back, inn).len(), 2);
    let _ = std::fs::remove_dir_all(&dir);
}
