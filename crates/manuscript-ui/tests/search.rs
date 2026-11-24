// SPDX-License-Identifier: MIT OR Apache-2.0
//! Project search: results through the index equal a full scan, and a 300,000-word project with
//! 1,000 scenes answers within the plan's 200 ms.

use std::path::Path;
use std::time::{Duration, Instant};

use serde_json::json;
use wordsmith_manuscript_model::{Area, Item, ItemId, ItemText, Kind, Place, Project, Text};
use wordsmith_manuscript_ui::find::{Query, find_in_project};
use wordsmith_manuscript_ui::search::Searcher;
use wordsmith_manuscript_ui::session::Session;

/// War and Peace in 1,000 scenes.
fn big() -> Project {
    let text = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/novels/war-and-peace.txt")).unwrap();
    let paras: Vec<String> = text.split("\n\n").map(|p| p.split_whitespace().collect::<Vec<_>>().join(" ")).filter(|p| !p.is_empty()).collect();
    let mut p = Project::new();
    for n in 0..1000 {
        let chunk = &paras[n * paras.len() / 1000..(n + 1) * paras.len() / 1000];
        let id = ItemId::from_counter(n as u64 + 1);
        p.binder.insert(Place::end_of(Area::Manuscript), Item::with_id(id, Kind::Text, format!("Scene {n}"))).unwrap();
        p.texts.insert(id, ItemText::with_body(Text::from_paragraphs(chunk)));
    }
    p
}

#[test]
fn the_index_gives_what_a_scan_gives() {
    let p = big();
    let s = Searcher::default();
    s.build_now(&p, 1);
    for (text, word, case) in [
        ("Moscow", false, true),
        ("moscow", false, false),
        ("Natásha", false, true),
        ("the Emperor", false, false),
        ("peace", true, false),
        ("oscow", false, false),
        ("—", false, false),
    ] {
        let q = Query { text: text.into(), word, case, regex: false };
        let via_index = s.search(&p, 1, &q).unwrap();
        let scan = find_in_project(&p, &q.matcher().unwrap());
        assert_eq!(via_index.len(), scan.len(), "{text}");
        if text != "—" {
            assert!(!scan.is_empty(), "{text} is in the novel");
        }
        assert!(via_index.iter().zip(&scan).all(|(a, b)| a.item == b.item && a.range == b.range), "{text}");
    }
}

#[test]
fn a_300000_word_project_answers_within_200_ms() {
    let p = big();
    assert!(p.manuscript_words() > 300_000);
    assert_eq!(p.binder.len(), 1000);
    let s = Searcher::default();
    let start = Instant::now();
    s.refresh(&p, 7);
    while s.ready(7).is_none() {
        assert!(start.elapsed() < Duration::from_secs(60), "the index is built in the background");
        std::thread::sleep(Duration::from_millis(5));
    }
    eprintln!("index built in {:?}", start.elapsed());
    let start = Instant::now();
    let found = s.search(&p, 7, &Query { text: "Moscow".into(), ..Query::default() }).unwrap();
    let took = start.elapsed();
    eprintln!("search took {took:?} for {} matches", found.len());
    assert!(!found.is_empty());
    let budget = if cfg!(debug_assertions) { Duration::from_millis(600) } else { Duration::from_millis(200) };
    assert!(took < budget, "{took:?}");
}

#[test]
fn the_command_lists_items_with_their_matches() {
    let mut p = Project::new();
    for (n, t) in [(1, "The lamp burned."), (2, "No light here."), (3, "A lamp, a lamp.")] {
        let id = ItemId::from_counter(n);
        p.binder.insert(Place::end_of(Area::Manuscript), Item::with_id(id, Kind::Text, format!("S{n}"))).unwrap();
        p.texts.insert(id, ItemText::with_body(Text::from_paragraphs(&[t])));
    }
    let mut s = Session::new(p);
    let out = s.run("search.project", &json!({ "query": "lamp" })).unwrap();
    assert_eq!(out["count"], 3);
    assert_eq!(out["items"].as_array().unwrap().len(), 2);
    assert_eq!(out["items"][1]["matches"], 2);
}
