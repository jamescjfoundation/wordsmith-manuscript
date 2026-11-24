// SPDX-License-Identifier: MIT OR Apache-2.0
//! Grammar hints (ED-9): offline, off by default, shown only when the writer turns them on;
//! taking a hint's replacement changes only those words.

use serde_json::json;
use wordsmith_manuscript_model::{Area, Item, ItemId, ItemText, Kind, Place, Project, Text};
use wordsmith_manuscript_ui::editor::{Editor, registry};
use wordsmith_manuscript_ui::session::Session;
use wordsmith_manuscript_ui::spelling::{self, SpellCache};

const TEXT: &str = "She walked to the the harbour and waited.";

#[test]
fn hints_are_found_offline_and_only_when_on() {
    let found = spelling::hints(TEXT, "en");
    assert!(!found.is_empty(), "a repeated word is hinted");
    let h = &found[0];
    assert!(TEXT.get(h.range.clone()).is_some_and(|w| w.contains("the")), "{h:?}");
    let mut p = Project::new();
    let id = ItemId::from_counter(1);
    p.binder.insert(Place::end_of(Area::Manuscript), Item::with_id(id, Kind::Text, "Scene")).unwrap();
    p.texts.insert(id, ItemText::with_body(Text::from_paragraphs(&[TEXT])));
    let mut cache = SpellCache::new(&p);
    assert!(cache.hints(0, 1, TEXT).is_empty(), "off by default");
    cache.grammar = true;
    assert_eq!(cache.hints(0, 1, TEXT).len(), found.len());
    cache.ignored.insert((h.message.clone(), TEXT[h.range.clone()].to_owned()));
    assert_eq!(cache.hints(0, 1, TEXT).len(), found.len() - 1, "ignored hints stay hidden");
    let mut s = Session::new(p);
    let out = s.run("grammar.check", &json!({ "id": id.as_text() })).unwrap();
    assert!(!out["hints"].as_array().unwrap().is_empty());
    assert!(out["hints"][0]["message"].as_str().is_some_and(|m| !m.is_empty()));
}

#[test]
fn a_suggestion_replaces_only_its_words() {
    let h = spelling::hints(TEXT, "en").into_iter().next().unwrap();
    let mut ed = Editor::new(ItemId::from_counter(1), &Text::from_paragraphs(&[TEXT]), registry());
    let fix = h.suggestions.first().cloned().unwrap_or_default();
    ed.session.sel = jcj_engine::Selection {
        anchor: wordsmith_manuscript_ui::editor::pos(0, h.range.start),
        focus: wordsmith_manuscript_ui::editor::pos(0, h.range.end),
    };
    ed.type_text(&fix).unwrap();
    let after = ed.paragraph(0).unwrap().text.clone();
    assert_eq!(after.replace(&fix, ""), format!("{}{}", &TEXT[..h.range.start], &TEXT[h.range.end..]).replace(&fix, ""));
    assert!(spelling::hints(&after, "en").len() < spelling::hints(TEXT, "en").len(), "{after}");
}
