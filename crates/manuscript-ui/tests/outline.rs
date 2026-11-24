// SPDX-License-Identifier: MIT OR Apache-2.0
//! The outline: rows, sorting as a view, applying a sort, and the table in the harness.

use std::collections::HashSet;

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use jcj_shell::egui;
use serde_json::json;
use wordsmith_manuscript_model::{Area, Item, ItemId, ItemText, Kind, Place, Project, Role, Text};
use wordsmith_manuscript_ui::outline::{self, OutlineView};
use wordsmith_manuscript_ui::session::Session;

fn id(n: u64) -> ItemId {
    ItemId::from_counter(n)
}

fn project() -> Project {
    let mut p = Project::new();
    let mut ch = Item::with_id(id(1), Kind::Folder, "Chapter").role(Role::Chapter);
    for (n, t, words) in [(2, "Cove", "one two three"), (3, "Attic", "one"), (4, "Barn", "one two")] {
        ch.children.push(Item::with_id(id(n), Kind::Text, t).role(Role::Scene));
        p.texts.insert(id(n), ItemText::with_body(Text::from_paragraphs(&[words])));
    }
    p.binder.insert(Place::end_of(Area::Manuscript), ch).unwrap();
    p
}

fn titles(s: &Session) -> Vec<String> {
    s.project.binder.get(id(1)).unwrap().children.iter().map(|c| c.title.clone()).collect()
}

#[test]
fn rows_list_descendants_with_their_columns() {
    let mut s = Session::new(project());
    s.run("item.setTarget", &json!({ "id": id(2).as_text(), "words": 6 })).unwrap();
    s.run("item.setStatus", &json!({ "id": id(2).as_text(), "status": "Done" })).unwrap();
    let rows = s.run("outline.list", &json!({ "root": id(1).as_text() })).unwrap();
    let rows = rows.as_array().unwrap();
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0]["Title"], "Cove");
    assert_eq!(rows[0]["Words"], "3");
    assert_eq!(rows[0]["Target"], "6");
    assert_eq!(rows[0]["Status"], "Done");
    let by_words = s.run("outline.list", &json!({ "root": id(1).as_text(), "sort": 3 })).unwrap();
    let order: Vec<&str> = by_words.as_array().unwrap().iter().map(|r| r["Title"].as_str().unwrap()).collect();
    assert_eq!(order, ["Attic", "Barn", "Cove"]);
    assert_eq!(titles(&s), ["Cove", "Attic", "Barn"], "sorting is only a view");
    assert_eq!(outline::items(&s.project, None, &HashSet::new()).len(), 4);
}

#[test]
fn applying_a_sort_reorders_the_binder_in_one_step() {
    let mut s = Session::new(project());
    s.run("outline.applySort", &json!({ "root": id(1).as_text(), "column": 0 })).unwrap();
    assert_eq!(titles(&s), ["Attic", "Barn", "Cove"]);
    s.run("outline.applySort", &json!({ "root": id(1).as_text(), "column": 3, "descending": true })).unwrap();
    assert_eq!(titles(&s), ["Cove", "Barn", "Attic"]);
    s.undo();
    assert_eq!(titles(&s), ["Attic", "Barn", "Cove"]);
    s.undo();
    assert_eq!(titles(&s), ["Cove", "Attic", "Barn"]);
}

#[test]
fn the_table_draws_rows_and_sorts_by_its_headings() {
    struct St {
        s: Session,
        v: OutlineView,
        styled: bool,
    }
    let mut h = Harness::builder().with_size(egui::vec2(1100.0, 500.0)).build_ui_state(
        |ui, st: &mut St| {
            if !st.styled {
                wordsmith_manuscript_ui::brand::apply(ui.ctx(), jcj_shell::theme::Mode::Light);
                st.styled = true;
            }
            let _ = outline::view(ui, &mut st.s, &mut st.v, Some(id(1)), &HashSet::new(), None);
        },
        St { s: Session::new(project()), v: OutlineView::default(), styled: false },
    );
    h.run();
    assert!(h.query_by_label("Attic").is_some());
    h.get_by_label("Words").click();
    h.run();
    assert_eq!(h.state().v.table.sort.map(|s| s.column), Some(3));
    assert!(h.query_by_label("Apply This Order to the Binder").is_some());
}
