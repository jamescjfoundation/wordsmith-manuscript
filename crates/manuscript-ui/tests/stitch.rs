// SPDX-License-Identifier: MIT OR Apache-2.0
//! The stitched view: edits land in the right items, items are never merged or lost, undo spans
//! the view, random edits keep every word, and a 200,000-word view stays fast.

use std::time::{Duration, Instant};

use proptest::prelude::*;
use serde_json::json;
use wordsmith_manuscript_model::{Area, Item, ItemId, ItemText, Kind, Place, Project, Role, Text};
use wordsmith_manuscript_ui::editor::{Editor, pos, registry};
use wordsmith_manuscript_ui::stitch;

fn id(n: u64) -> ItemId {
    ItemId::from_counter(n)
}

fn project(scenes: &[&[&str]]) -> Project {
    let mut p = Project::new();
    let mut ch = Item::with_id(id(100), Kind::Folder, "Chapter").role(Role::Chapter);
    for (n, paras) in scenes.iter().enumerate() {
        let sid = id(n as u64 + 1);
        ch.children.push(Item::with_id(sid, Kind::Text, format!("Scene {}", n + 1)).role(Role::Scene));
        if !paras.is_empty() {
            p.texts.insert(sid, ItemText::with_body(Text::from_paragraphs(paras)));
        }
    }
    p.binder.insert(Place::end_of(Area::Manuscript), ch).unwrap();
    p
}

fn open(p: &Project) -> Editor {
    stitch::open(p, &stitch::items_under(p, id(100)), registry()).unwrap()
}

fn texts(ed: &Editor) -> Vec<(ItemId, Text)> {
    stitch::split(&ed.session.doc)
}

fn para_of(ed: &Editor, text: &str) -> usize {
    (0..ed.paragraphs().len()).find(|i| ed.paragraph(*i).is_some_and(|p| p.text == text)).unwrap()
}

#[test]
fn it_opens_and_splits_back_unchanged() {
    let p = project(&[&["One a.", "One b."], &[], &["Three."]]);
    let ed = open(&p);
    assert_eq!(stitch::dividers(&ed.session.doc), vec![id(1), id(2), id(3)]);
    let back = texts(&ed);
    assert_eq!(back, vec![(id(1), p.texts[&id(1)].body.clone()), (id(2), Text::new()), (id(3), p.texts[&id(3)].body.clone())]);
    let mut q = p.clone();
    assert!(!stitch::write_back(&ed, &mut q));
    assert_eq!(q, p);
}

#[test]
fn a_paragraph_typed_at_a_boundary_belongs_to_the_item_above() {
    let p = project(&[&["First."], &["Second."]]);
    let mut ed = open(&p);
    let i = para_of(&ed, "First.");
    ed.set_caret(i, 6, false);
    ed.new_paragraph().unwrap();
    ed.type_text("Added").unwrap();
    let back = texts(&ed);
    assert_eq!(back[0].1, Text::from_paragraphs(&["First.", "Added"]));
    assert_eq!(back[1].1, Text::from_paragraphs(&["Second."]));
}

#[test]
fn edits_across_a_divider_keep_both_items() {
    let p = project(&[&["Alpha beta."], &["Gamma delta."]]);
    let mut ed = open(&p);
    let a = para_of(&ed, "Alpha beta.");
    let b = para_of(&ed, "Gamma delta.");
    ed.session.sel = jcj_engine::Selection { anchor: pos(a, 6), focus: pos(b, 6) };
    ed.type_text("X").unwrap();
    let back = texts(&ed);
    assert_eq!(back.len(), 2);
    assert_eq!(back[0].1, Text::from_paragraphs(&["Alpha X"]));
    assert_eq!(back[1].1, Text::from_paragraphs(&["delta."]));
    // Undo puts both back as they were.
    ed.run("edit.undo", &json!({})).unwrap();
    assert_eq!(texts(&ed), vec![(id(1), Text::from_paragraphs(&["Alpha beta."])), (id(2), Text::from_paragraphs(&["Gamma delta."]))]);
}

#[test]
fn backspace_and_delete_never_merge_items() {
    let p = project(&[&["One."], &["Two."]]);
    let mut ed = open(&p);
    let b = para_of(&ed, "Two.");
    ed.set_caret(b, 0, false);
    ed.run("text.backspace", &json!({})).unwrap();
    let a = para_of(&ed, "One.");
    ed.set_caret(a, 4, false);
    ed.run("text.delete", &json!({})).unwrap();
    assert_eq!(texts(&ed), vec![(id(1), Text::from_paragraphs(&["One."])), (id(2), Text::from_paragraphs(&["Two."]))]);
}

#[test]
fn deleting_all_of_an_items_text_keeps_the_item() {
    let mut p = project(&[&["Keep."], &["Gone entirely."], &["Also keep."]]);
    let mut ed = open(&p);
    let i = para_of(&ed, "Gone entirely.");
    ed.session.sel = jcj_engine::Selection { anchor: pos(i, 0), focus: pos(i, 14) };
    ed.run("text.delete", &json!({})).unwrap();
    assert!(stitch::write_back(&ed, &mut p));
    assert!(p.binder.contains(id(2)));
    assert!(p.texts.get(&id(2)).is_none_or(|t| t.body.is_empty()), "the item stays, empty");
    assert_eq!(p.texts[&id(3)].body, Text::from_paragraphs(&["Also keep."]));
}

#[derive(Clone, Debug)]
enum Op {
    Type(f64, f64, String),
    Delete(f64, f64, f64, f64),
    Enter(f64, f64),
    Back(f64, f64),
}

fn place(ed: &Editor, pf: f64, of: f64) -> (usize, usize) {
    let n = ed.paragraphs().len();
    let i = ((pf * n as f64) as usize).min(n - 1);
    let text = &ed.paragraph(i).unwrap().text;
    let mut off = (of * text.len() as f64) as usize;
    while !text.is_char_boundary(off) {
        off -= 1;
    }
    (i, off)
}

fn words(ed: &Editor) -> Vec<String> {
    ed.paragraphs().iter().filter(|p| !stitch::is_divider(p)).flat_map(|p| wordsmith_manuscript_model::words::words(&p.text)).collect()
}

fn arb_op() -> impl Strategy<Value = Op> {
    let f = 0.0f64..1.0;
    prop_oneof![
        3 => (f.clone(), f.clone(), "[a-z é]{1,6}").prop_map(|(a, b, t)| Op::Type(a, b, t)),
        2 => (f.clone(), f.clone(), f.clone(), f.clone()).prop_map(|(a, b, c, d)| Op::Delete(a, b, c, d)),
        1 => (f.clone(), f.clone()).prop_map(|(a, b)| Op::Enter(a, b)),
        1 => (f.clone(), f).prop_map(|(a, b)| Op::Back(a, b)),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 200, ..ProptestConfig::default() })]

    #[test]
    fn random_edits_keep_every_word_and_every_item(ops in proptest::collection::vec(arb_op(), 1..25)) {
        let p = project(&[&["Once upon a time.", "There was a keeper."], &[], &["The tide rose.", "Night fell slowly."]]);
        let mut ed = open(&p);
        for op in ops {
            match op {
                Op::Type(a, b, t) => {
                    let (i, o) = place(&ed, a, b);
                    ed.set_caret(i, o, false);
                    let _ = ed.type_text(&t);
                }
                Op::Delete(a, b, c, d) => {
                    let (i, o) = place(&ed, a, b);
                    let (j, q) = place(&ed, c, d);
                    ed.session.sel = jcj_engine::Selection { anchor: pos(i, o), focus: pos(j, q) };
                    let _ = ed.run("text.delete", &json!({}));
                }
                Op::Enter(a, b) => {
                    let (i, o) = place(&ed, a, b);
                    ed.set_caret(i, o, false);
                    let _ = ed.new_paragraph();
                }
                Op::Back(a, b) => {
                    let (i, o) = place(&ed, a, b);
                    ed.set_caret(i, o, false);
                    let _ = ed.run("text.backspace", &json!({}));
                }
            }
            prop_assert_eq!(stitch::dividers(&ed.session.doc), vec![id(1), id(2), id(3)]);
        }
        // Words in = words out: the items' texts hold exactly the view's words, in order.
        let split: Vec<String> = texts(&ed).iter().flat_map(|(_, t)| t.words()).collect();
        prop_assert_eq!(split, words(&ed));
    }
}

#[test]
fn a_200000_word_view_types_within_the_frame_budget() {
    // 400 scenes of 500 words.
    let para: String = (0..50).map(|k| format!("word{k}")).collect::<Vec<_>>().join(" ");
    let paras: Vec<&str> = (0..10).map(|_| para.as_str()).collect();
    let scenes: Vec<&[&str]> = (0..400).map(|_| paras.as_slice()).collect();
    let p = project(&scenes);
    let ed = open(&p);
    assert!(ed.counts().0 >= 200_000);
    let look = wordsmith_manuscript_ui::editor::EditorLook::default();
    let mut h = egui_kittest::Harness::builder().with_size(jcj_shell::egui::vec2(1000.0, 800.0)).build_ui_state(
        |ui, st: &mut (Editor, bool)| {
            if !st.1 {
                wordsmith_manuscript_ui::brand::apply(ui.ctx(), jcj_shell::theme::Mode::Light);
                st.1 = true;
            }
            st.0.ui(ui, &look);
        },
        (ed, false),
    );
    h.run();
    // Put the caret in the middle of the manuscript and type, one character a frame.
    let mid = h.state().0.paragraphs().len() / 2;
    h.state_mut().0.set_caret(mid, 0, false);
    h.run();
    let mut times = Vec::new();
    for _ in 0..30 {
        let start = Instant::now();
        h.state_mut().0.type_text("a").unwrap();
        h.step();
        times.push(start.elapsed());
    }
    times.sort();
    let median = times[times.len() / 2];
    eprintln!("typing in a 200,000-word stitched view: median {median:?} a frame");
    // The plan's budget is 16 ms on the CI machine in a release build; debug builds get room.
    let budget = if cfg!(debug_assertions) { Duration::from_millis(120) } else { Duration::from_millis(16) };
    assert!(median < budget, "median {median:?}");
}
