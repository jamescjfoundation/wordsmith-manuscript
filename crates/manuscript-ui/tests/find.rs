// SPDX-License-Identifier: MIT OR Apache-2.0
//! Find and replace, spelling with the project's words, and inline links.

use serde_json::json;
use wordsmith_manuscript_format::dialect;
use wordsmith_manuscript_model::{Area, Entry, Inline, Item, ItemId, ItemText, Kind, Marks, Place, Project, Text};
use wordsmith_manuscript_ui::Manuscript;
use wordsmith_manuscript_ui::editor::{Editor, pos, registry};
use wordsmith_manuscript_ui::find::Query;
use wordsmith_manuscript_ui::session::Session;

fn q(text: &str) -> Query {
    Query { text: text.into(), ..Query::default() }
}

fn editor(t: &str) -> Editor {
    Editor::new(ItemId::from_counter(1), &dialect::parse(t), registry())
}

#[test]
fn find_in_the_document_with_case_word_and_patterns() {
    let ed = editor("The cat sat. Cathedral cats.\n\nthe CAT again.\n");
    assert_eq!(ed.find(&q("cat"), false).unwrap().len(), 4);
    assert_eq!(ed.find(&Query { case: true, ..q("cat") }, false).unwrap().len(), 2);
    assert_eq!(ed.find(&Query { word: true, ..q("cat") }, false).unwrap().len(), 2);
    assert_eq!(ed.find(&Query { regex: true, ..q(r"[Cc]at(s|hedral)") }, false).unwrap().len(), 2);
    assert!(ed.find(&Query { regex: true, ..q("(") }, false).is_err());
}

#[test]
fn replace_all_keeps_marks_and_is_one_undo_step() {
    let mut ed = editor("A *grey* day, a grey sea.\n");
    let n = ed.replace_all(&q("grey"), "gray", false).unwrap();
    assert_eq!(n, 2);
    assert_eq!(dialect::write(&ed.text()), "A *gray* day, a gray sea.\n");
    ed.run("edit.undo", &json!({})).unwrap();
    assert_eq!(dialect::write(&ed.text()), "A *grey* day, a grey sea.\n");
    // Groups in patterns.
    let mut ed = editor("Smith, John and Brown, Ann.\n");
    ed.replace_all(&Query { regex: true, ..q(r"(\w+), (\w+)") }, "$2 $1", false).unwrap();
    assert_eq!(ed.paragraph(0).unwrap().text, "John Smith and Ann Brown.");
}

#[test]
fn replace_in_the_selection_only() {
    let mut ed = editor("one one\n\none one\n");
    ed.session.sel = jcj_engine::Selection { anchor: pos(1, 0), focus: pos(1, 7) };
    ed.replace_all(&q("one"), "two", true).unwrap();
    assert_eq!(dialect::write(&ed.text()), "one one\n\ntwo two\n");
}

#[test]
fn next_wraps_around_and_replace_one_moves_on() {
    let mut ed = editor("x a x b x\n");
    ed.set_caret(0, 3, false);
    assert!(ed.find_next(&q("x"), false).unwrap());
    assert_eq!(ed.selected_text(), "x");
    assert_eq!(ed.session.sel.ordered().0.off, 4);
    ed.find_next(&q("x"), false).unwrap();
    ed.find_next(&q("x"), false).unwrap();
    assert_eq!(ed.session.sel.ordered().0.off, 0, "wraps to the start");
    ed.replace_one(&q("x"), "y").unwrap();
    assert_eq!(ed.paragraph(0).unwrap().text, "y a x b x");
}

fn project() -> Project {
    let mut p = Project::new();
    for (n, area, text) in
        [(1, Area::Manuscript, "The lighthouse stood."), (2, Area::Notes, "A lighthouse plan."), (3, Area::Trash, "Old lighthouse.")]
    {
        p.binder.insert(Place::end_of(area), Item::with_id(ItemId::from_counter(n), Kind::Text, format!("Item {n}"))).unwrap();
        p.texts.insert(ItemId::from_counter(n), ItemText::with_body(Text::from_paragraphs(&[text])));
    }
    p.text_mut(ItemId::from_counter(1)).meta.synopsis = "Where the lighthouse is.".into();
    p
}

#[test]
fn project_find_covers_text_synopses_and_notes() {
    let mut s = Session::new(project());
    let out = s.run("find.project", &json!({ "query": "lighthouse" })).unwrap();
    assert_eq!(out["count"], 4);
    let fields: Vec<&str> = out["matches"].as_array().unwrap().iter().map(|m| m["field"].as_str().unwrap()).collect();
    assert!(fields.contains(&"synopsis"));
}

#[test]
fn project_replace_all_snapshots_first_skips_the_trash_and_undoes_in_one_step() {
    let mut s = Session::new(project());
    let before = s.project.texts.clone();
    let out = s.run("find.replaceProject", &json!({ "query": "lighthouse", "with": "beacon" })).unwrap();
    assert_eq!(out, json!({ "replaced": 2, "items": 2 }));
    assert_eq!(s.project.texts[&ItemId::from_counter(1)].body, Text::from_paragraphs(&["The beacon stood."]));
    assert_eq!(s.project.texts[&ItemId::from_counter(3)].body, Text::from_paragraphs(&["Old lighthouse."]), "the Trash is left alone");
    let snaps = &s.project.snapshots[&ItemId::from_counter(1)];
    assert_eq!(snaps.len(), 1);
    assert_eq!(snaps[0].text, Text::from_paragraphs(&["The lighthouse stood."]));
    assert!(snaps[0].name.contains("lighthouse"));
    s.undo();
    assert_eq!(s.project.texts, before);
}

#[test]
fn project_words_quiet_the_spelling_check() {
    let mut p = Project::new();
    p.binder.insert(Place::end_of(Area::Bible), Item::with_id(ItemId::from_counter(9), Kind::Entry, "Qwilleran")).unwrap();
    p.entries.insert(ItemId::from_counter(9), Entry::new("character", "Qwilleran Thrup"));
    let mut s = Session::new(p);
    let words = wordsmith_manuscript_ui::spelling::project_words(&s.project);
    let text = "Qwilleran saw a zorblax.";
    let marked: Vec<&str> = wordsmith_manuscript_ui::spelling::marks(text, "en", &words).into_iter().map(|r| &text[r]).collect();
    assert_eq!(marked, ["zorblax"]);
    s.run("spelling.addWord", &json!({ "word": "zorblax" })).unwrap();
    let words = wordsmith_manuscript_ui::spelling::project_words(&s.project);
    assert!(wordsmith_manuscript_ui::spelling::marks(text, "en", &words).is_empty());
}

#[test]
fn links_round_trip_preview_and_follow() {
    let mut p = Project::new();
    let target = ItemId::from_counter(5);
    p.binder.insert(Place::end_of(Area::Manuscript), Item::with_id(ItemId::from_counter(1), Kind::Text, "Scene")).unwrap();
    p.binder.insert(Place::end_of(Area::Bible), Item::with_id(target, Kind::Entry, "Mara")).unwrap();
    let mut e = Entry::new("character", "Mara");
    e.body = Text::from_paragraphs(&["The keeper of the light."]);
    p.entries.insert(target, e);
    p.texts.insert(ItemId::from_counter(1), ItemText::with_body(Text::from_paragraphs(&["Mara climbed."])));
    let mut app = Manuscript::with_session(Session::new(p));
    app.open_item(ItemId::from_counter(1));
    let ed = app.editor.as_mut().unwrap();
    ed.session.sel = jcj_engine::Selection { anchor: pos(0, 0), focus: pos(0, 4) };
    let link = format!("wsm:{target}");
    ed.command("write.link", &json!({ "target": link })).unwrap();
    let t = ed.text();
    assert_eq!(
        t.blocks[0],
        wordsmith_manuscript_model::Block::Para(vec![
            Inline::marked("Mara", Marks { link: Some(link.clone()), ..Marks::default() }),
            Inline::plain(" climbed.")
        ])
    );
    assert_eq!(dialect::parse(&dialect::write(&t)), t);
    assert_eq!(app.link_target(&link), Some(("Mara".into(), "The keeper of the light.".into())));
    assert_eq!(app.link_target(&format!("wsm:{}", ItemId::from_counter(77))).map(|x| x.0), Some("A deleted item".into()));
    app.follow_link(&link);
    assert_eq!(app.binder.selected, Some(target));
}
