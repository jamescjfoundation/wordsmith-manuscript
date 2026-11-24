// SPDX-License-Identifier: MIT OR Apache-2.0
//! The corkboard, and the milestone's test: a reorder in any of binder, corkboard or outline shows
//! at once in the other two, as one undoable command.

use std::collections::HashSet;

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use jcj_shell::egui;
use serde_json::json;
use wordsmith_manuscript_model::{Area, Item, ItemId, ItemText, Kind, Place, Project, Role, Text};
use wordsmith_manuscript_ui::cork::{self, CorkView};
use wordsmith_manuscript_ui::session::Session;
use wordsmith_manuscript_ui::{binder, outline};

fn id(n: u64) -> ItemId {
    ItemId::from_counter(n)
}

fn project() -> Project {
    let mut p = Project::new();
    let mut ch = Item::with_id(id(1), Kind::Folder, "Chapter").role(Role::Chapter);
    for (n, t) in [(2, "Arrival"), (3, "Storm"), (4, "Morning")] {
        let mut it = Item::with_id(id(n), Kind::Text, t).role(Role::Scene);
        if n == 3 {
            it.label = Some("Action".into());
            it.status = Some("First draft".into());
        }
        ch.children.push(it);
    }
    p.binder.insert(Place::end_of(Area::Manuscript), ch).unwrap();
    p.texts.insert(
        id(2),
        ItemText { meta: wordsmith_manuscript_model::ItemMeta { synopsis: "The keeper arrives.".into(), ..Default::default() }, body: Text::new() },
    );
    p
}

#[test]
fn cards_show_title_synopsis_label_color_and_status() {
    let p = project();
    let items = cork::items(&p, Some(id(1)), &HashSet::new());
    let b = cork::board(&p, &items);
    assert_eq!(b.cards.len(), 3);
    assert_eq!(b.cards[0].title, "Arrival");
    assert_eq!(b.cards[0].body, "The keeper arrives.");
    assert_eq!(b.cards[0].color, cork::PLAIN);
    assert_eq!(b.cards[1].color, "#C0504D");
    assert_eq!(b.cards[1].label, "First draft");
}

fn orders(s: &Session) -> (Vec<String>, Vec<String>, Vec<String>) {
    let p = &s.project;
    let binder: Vec<String> = binder::rows(p, &binder::BinderView::default()).iter().skip(2).take(3).map(|r| r.label.clone()).collect();
    let outline: Vec<String> = outline::rows(p, &outline::items(p, Some(id(1)), &HashSet::new())).iter().map(|r| r.cells[0].text.clone()).collect();
    let cork: Vec<String> = cork::board(p, &cork::items(p, Some(id(1)), &HashSet::new())).cards.iter().map(|c| c.title.clone()).collect();
    (binder, outline, cork)
}

#[test]
fn a_reorder_anywhere_shows_everywhere_and_undoes_in_one_step() {
    let mut s = Session::new(project());
    // The corkboard and the outline both reorder through binder.move, as the binder does.
    s.run("binder.move", &json!({ "id": id(4).as_text(), "area": "manuscript", "parent": id(1).as_text(), "index": 0 })).unwrap();
    let (b, o, c) = orders(&s);
    assert_eq!(b, ["Morning", "Arrival", "Storm"]);
    assert_eq!(o, b);
    assert_eq!(c, b);
    s.undo();
    let (b, o, c) = orders(&s);
    assert_eq!(b, ["Arrival", "Storm", "Morning"]);
    assert_eq!((o, c), (b.clone(), b));
}

#[test]
fn the_corkboard_draws_and_adds_cards() {
    struct St {
        s: Session,
        v: CorkView,
        styled: bool,
    }
    let mut h = Harness::builder().with_size(egui::vec2(1000.0, 700.0)).build_ui_state(
        |ui, st: &mut St| {
            if !st.styled {
                wordsmith_manuscript_ui::brand::apply(ui.ctx(), jcj_shell::theme::Mode::Light);
                st.styled = true;
            }
            let _ = cork::view(ui, &mut st.s, &mut st.v, Some(id(1)), &HashSet::new(), None);
        },
        St { s: Session::new(project()), v: CorkView::default(), styled: false },
    );
    h.run();
    h.get_by_label("New Card").click();
    h.run();
    let kids = &h.state().s.project.binder.get(id(1)).unwrap().children;
    assert_eq!(kids.len(), 4);
    assert_eq!(kids[3].role, Role::Scene, "a new card is a new item with the role the binder would give it");
}

#[test]
fn a_filter_applies_to_binder_corkboard_and_outline_together() {
    let mut app = wordsmith_manuscript_ui::Manuscript::with_session(Session::new(project()));
    let mut sh = jcj_shell::Shell::new("Test", "0", wordsmith_manuscript_ui::brand::tokens());
    jcj_shell::App::run(&mut app, &jcj_shell::Cmd::with("filter.set", json!({ "labels": ["Action"] })), &mut sh).unwrap();
    let s = app.session.as_ref().unwrap();
    let hidden = &app.binder.hidden;
    let binder_titles: Vec<String> = binder::rows(&s.project, &app.binder).iter().map(|r| r.label.clone()).collect();
    assert!(binder_titles.contains(&"Storm".to_owned()) && !binder_titles.contains(&"Arrival".to_owned()));
    assert!(binder_titles.contains(&"Chapter".to_owned()), "the ancestor stays as context");
    assert_eq!(cork::items(&s.project, Some(id(1)), hidden), [id(3)]);
    assert_eq!(outline::items(&s.project, Some(id(1)), hidden), [(id(3), 0)]);
    jcj_shell::App::run(&mut app, &jcj_shell::Cmd::new("filter.clear"), &mut sh).unwrap();
    assert!(app.binder.hidden.is_empty());
}
