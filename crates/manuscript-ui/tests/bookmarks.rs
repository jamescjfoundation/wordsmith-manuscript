// SPDX-License-Identifier: MIT OR Apache-2.0
//! Bookmarks (BND-7): toggled on an item from anywhere (the key fills in the selected item),
//! listed in order, kept with the project; trashed items drop out.

use jcj_shell::{App, Cmd, Shell};
use serde_json::json;
use wordsmith_manuscript_model::{Area, Item, ItemId, Kind, Place, Project};
use wordsmith_manuscript_ui::Manuscript;
use wordsmith_manuscript_ui::session::Session;

fn id(n: u64) -> ItemId {
    ItemId::from_counter(n)
}

#[test]
fn bookmarks_toggle_list_and_keep() {
    let mut p = Project::new();
    for n in 1..=3 {
        p.binder.insert(Place::end_of(Area::Manuscript), Item::with_id(id(n), Kind::Text, format!("Scene {n}"))).unwrap();
    }
    let mut app = Manuscript::with_session(Session::new(p));
    let mut sh = Shell::new("Test", "0", wordsmith_manuscript_ui::brand::tokens());
    app.binder.selected = Some(id(3));
    app.run(&Cmd::new("bookmark.toggle"), &mut sh).unwrap();
    let s = app.session.as_mut().unwrap();
    s.run("bookmark.toggle", &json!({ "id": id(1).as_text() })).unwrap();
    let titles = |s: &mut Session| -> Vec<String> {
        s.run("bookmark.list", &json!({})).unwrap().as_array().unwrap().iter().map(|x| x["title"].as_str().unwrap().to_owned()).collect()
    };
    assert_eq!(titles(s), ["Scene 3", "Scene 1"]);
    assert_eq!(s.run("bookmark.toggle", &json!({ "id": id(3).as_text() })).unwrap()["bookmarked"], false);
    assert_eq!(titles(s), ["Scene 1"]);
    s.run("binder.trash", &json!({ "id": id(1).as_text() })).unwrap();
    assert!(titles(s).is_empty());
    s.undo();
    assert_eq!(titles(s), ["Scene 1"]);
    let dir = std::env::temp_dir().join(format!("wsm-bookmarks-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    s.save_as(&dir).unwrap();
    let (_, back) = wordsmith_manuscript_format::folder::Folder::open(&dir).unwrap();
    assert_eq!(wordsmith_manuscript_ui::bookmarks::all(&back), vec![id(1)]);
    let _ = std::fs::remove_dir_all(&dir);
}
