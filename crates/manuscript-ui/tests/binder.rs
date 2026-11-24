// SPDX-License-Identifier: MIT OR Apache-2.0
//! The binder: every operation as a command, undo and redo, the panel in the egui test harness,
//! and a large binder staying responsive.

use std::time::Instant;

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use jcj_shell::egui;
use serde_json::{Value, json};
use wordsmith_manuscript_model::{Area, Item, ItemId, Kind, Place, Project, Role};
use wordsmith_manuscript_ui::binder::{self, BinderView, Node};
use wordsmith_manuscript_ui::session::Session;
use wordsmith_manuscript_ui::tree::Zone;

fn id(n: u64) -> ItemId {
    ItemId::from_counter(n)
}

fn sample() -> Session {
    let mut p = Project::new();
    let part = Item::with_id(id(1), Kind::Folder, "Part One").role(Role::Part);
    let ch = Item::with_id(id(2), Kind::Folder, "Chapter One").role(Role::Chapter);
    let s1 = Item::with_id(id(3), Kind::Text, "Arrival").role(Role::Scene);
    let s2 = Item::with_id(id(4), Kind::Text, "Storm").role(Role::Scene);
    p.binder.insert(Place::end_of(Area::Manuscript), part.with_children(vec![ch.with_children(vec![s1, s2])])).unwrap();
    p.binder.insert(Place::end_of(Area::Notes), Item::with_id(id(5), Kind::Text, "Plan")).unwrap();
    Session::new(p)
}

fn run(s: &mut Session, cmd: &str, v: Value) -> Value {
    s.run(cmd, &v).unwrap_or_else(|e| panic!("{cmd}: {e}"))
}

fn titles(s: &Session, area: Area) -> Vec<(usize, String)> {
    s.project.binder.rows_of(area).iter().map(|r| (r.depth, r.item.title.clone())).collect()
}

#[test]
fn new_items_take_their_role_from_the_parent() {
    let mut s = sample();
    let out = run(&mut s, "binder.new", json!({ "inside": id(2).as_text(), "title": "Dinner" }));
    let new: ItemId = out["id"].as_str().unwrap().parse().unwrap();
    assert_eq!(s.project.binder.get(new).unwrap().role, Role::Scene);
    let out = run(&mut s, "binder.newFolder", json!({ "inside": id(1).as_text() }));
    let ch: ItemId = out["id"].as_str().unwrap().parse().unwrap();
    assert_eq!(s.project.binder.get(ch).unwrap().role, Role::Chapter);
    let out = run(&mut s, "binder.new", json!({ "after": id(3).as_text() }));
    let after: ItemId = out["id"].as_str().unwrap().parse().unwrap();
    assert_eq!(s.project.binder.place_of(after).unwrap().index, 1);
    let out = run(&mut s, "binder.new", json!({ "area": "research" }));
    let r: ItemId = out["id"].as_str().unwrap().parse().unwrap();
    assert_eq!(s.project.binder.get(r).unwrap().role, Role::None);
    assert!(s.run("binder.new", &json!({ "kind": "screenplay" })).is_err());
}

#[test]
fn every_operation_is_one_undoable_step() {
    let mut s = sample();
    let start = s.project.clone();
    let steps: Vec<(&str, Value)> = vec![
        ("binder.rename", json!({ "id": id(3).as_text(), "title": "The Arrival" })),
        ("binder.moveDown", json!({ "id": id(3).as_text() })),
        ("binder.moveUp", json!({ "id": id(3).as_text() })),
        ("binder.move", json!({ "id": id(5).as_text(), "area": "manuscript", "parent": id(2).as_text(), "index": 0 })),
        ("binder.indent", json!({ "id": id(4).as_text() })),
        ("binder.outdent", json!({ "id": id(4).as_text() })),
        ("binder.setRole", json!({ "id": id(5).as_text(), "role": "scene" })),
        ("binder.setInclude", json!({ "id": id(3).as_text(), "include": false })),
        ("text.set", json!({ "id": id(3).as_text(), "text": "The keeper *arrived*.\n" })),
        ("binder.trash", json!({ "id": id(4).as_text() })),
        ("binder.restore", json!({ "id": id(4).as_text() })),
        ("binder.trash", json!({ "id": id(4).as_text() })),
        ("binder.emptyTrash", json!({ "confirm": true })),
    ];
    let mut states = vec![s.project.clone()];
    for (cmd, v) in &steps {
        run(&mut s, cmd, v.clone());
        states.push(s.project.clone());
    }
    assert!(!s.project.binder.contains(id(4)));
    assert_eq!(s.project.binder.get(id(3)).unwrap().title, "The Arrival");
    // Undo walks back through every state exactly; redo walks forward again.
    for want in states.iter().rev().skip(1) {
        assert!(s.undo().is_some());
        assert_eq!(&s.project, want);
    }
    assert_eq!(s.project, start);
    assert!(s.undo().is_none());
    for want in states.iter().skip(1) {
        assert!(s.redo().is_some());
        assert_eq!(&s.project, want);
    }
}

#[test]
fn refused_operations_change_nothing() {
    let mut s = sample();
    let before = s.project.clone();
    assert!(s.run("binder.move", &json!({ "id": id(1).as_text(), "area": "manuscript", "parent": id(3).as_text() })).is_err());
    assert!(s.run("binder.emptyTrash", &json!({})).is_err());
    assert!(s.run("binder.rename", &json!({ "id": "nope", "title": "x" })).is_err());
    assert!(s.run("binder.indent", &json!({ "id": id(1).as_text() })).is_err());
    assert!(s.run("binder.outdent", &json!({ "id": id(1).as_text() })).is_err());
    assert_eq!(s.project, before);
    assert!(s.undo().is_none());
}

#[test]
fn emptying_the_trash_reports_what_went() {
    let mut s = sample();
    run(&mut s, "text.set", json!({ "id": id(4).as_text(), "text": "Three words here.\n" }));
    run(&mut s, "binder.trash", json!({ "id": id(2).as_text() }));
    assert_eq!(titles(&s, Area::Trash), vec![(0, "Chapter One".into()), (1, "Arrival".into()), (1, "Storm".into())]);
    let out = run(&mut s, "binder.emptyTrash", json!({ "confirm": true }));
    assert_eq!(out, json!({ "removed": 3, "words": 3 }));
    assert!(!s.project.texts.contains_key(&id(4)));
}

#[test]
fn drops_become_moves() {
    let s = sample();
    let p = &s.project;
    let v = binder::drop_params(p, id(5), Node::Item(id(3)), Zone::Before).unwrap();
    assert_eq!(v, json!({ "id": id(5).as_text(), "area": "manuscript", "parent": id(2).as_text(), "index": 0 }));
    let v = binder::drop_params(p, id(5), Node::Item(id(3)), Zone::After).unwrap();
    assert_eq!(v["index"], 1);
    let v = binder::drop_params(p, id(5), Node::Item(id(2)), Zone::Inside).unwrap();
    assert_eq!(v, json!({ "id": id(5).as_text(), "area": "manuscript", "parent": id(2).as_text() }));
    let v = binder::drop_params(p, id(3), Node::Area(Area::Research), Zone::Inside).unwrap();
    assert_eq!(v, json!({ "id": id(3).as_text(), "area": "research" }));
}

struct State {
    session: Session,
    view: BinderView,
    styled: bool,
}

fn harness(state: State) -> Harness<'static, State> {
    Harness::builder().with_size(egui::vec2(320.0, 600.0)).build_ui_state(
        |ui, st: &mut State| {
            if !st.styled {
                wordsmith_manuscript_ui::brand::apply(ui.ctx(), jcj_shell::theme::Mode::Light);
                st.styled = true;
            }
            let _ = binder::panel(ui, &mut st.session, &mut st.view);
        },
        state,
    )
}

#[test]
fn the_panel_shows_areas_and_items_and_selects_on_click() {
    let mut h = harness(State { session: sample(), view: BinderView::default(), styled: false });
    h.run();
    for label in ["Manuscript", "Story Bible", "Research", "Notes", "Trash", "Part One", "Chapter One", "Arrival", "Storm", "Plan"] {
        assert!(h.query_by_label(label).is_some(), "{label} is shown");
    }
    h.get_by_label("Storm").click();
    h.run();
    assert_eq!(h.state().view.selected, Some(id(4)));
    // Collapsing a folder hides its children.
    h.state_mut().view.closed.insert(Node::Item(id(2)));
    h.run();
    assert!(h.query_by_label("Storm").is_none());
}

#[test]
fn a_thousand_items_stay_responsive() {
    let mut p = Project::new();
    let mut n = 10;
    for c in 0..50 {
        let mut ch = Item::with_id(id(n), Kind::Folder, format!("Chapter {c}")).role(Role::Chapter);
        n += 1;
        for k in 0..19 {
            ch.children.push(Item::with_id(id(n), Kind::Text, format!("Scene {c}.{k}")).role(Role::Scene));
            n += 1;
        }
        p.binder.insert(Place::end_of(Area::Manuscript), ch).unwrap();
    }
    assert_eq!(p.binder.len(), 1000);
    let mut h = harness(State { session: Session::new(p), view: BinderView::default(), styled: false });
    h.run();
    let start = Instant::now();
    for _ in 0..20 {
        h.step();
    }
    let per_frame = start.elapsed() / 20;
    assert!(per_frame.as_millis() < 50, "a frame of the binder with 1,000 items took {per_frame:?}");
    // Moving the last scene to the top is one command and shows at once.
    let last = id(n - 1);
    h.state_mut().session.run("binder.move", &json!({ "id": last.as_text(), "area": "manuscript", "index": 0 })).unwrap();
    h.run();
    assert_eq!(h.state().session.project.binder.area(Area::Manuscript)[0].id, last);
}
