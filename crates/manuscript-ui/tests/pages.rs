// SPDX-License-Identifier: MIT OR Apache-2.0
//! The page view (ED-8): the same pages as the standard manuscript PDF, counted and drawn.

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use jcj_shell::egui;
use serde_json::json;
use wordsmith_manuscript_model::{Area, Item, ItemId, ItemText, Kind, Place, Project, Role, Text};
use wordsmith_manuscript_ui::pages::{self, PageView};
use wordsmith_manuscript_ui::session::Session;

fn id(n: u64) -> ItemId {
    ItemId::from_counter(n)
}

fn session() -> Session {
    let mut p = Project::new();
    let mut ch = Item::with_id(id(100), Kind::Folder, "One").role(Role::Chapter);
    for n in 1..=2 {
        ch.children.push(Item::with_id(id(n), Kind::Text, format!("Scene {n}")).role(Role::Scene));
        let para = "The keeper walked the length of the breakwater and back again, counting the lamps. ".repeat(6);
        let paras: Vec<String> = (0..40).map(|_| para.clone()).collect();
        p.texts.insert(id(n), ItemText::with_body(Text::from_paragraphs(&paras)));
    }
    p.binder.insert(Place::end_of(Area::Manuscript), ch).unwrap();
    Session::new(p)
}

#[test]
fn page_counts_match_the_pdf() {
    let mut s = session();
    let whole = s.run("pages.count", &json!({})).unwrap()["pages"].as_u64().unwrap();
    let pdf = wordsmith_manuscript_io::standard::pdf(&s.project, &Default::default(), &Default::default()).unwrap();
    assert_eq!(whole as usize, pdf.pages, "the page view lays out exactly as the PDF");
    let one = s.run("pages.count", &json!({ "id": id(1).as_text() })).unwrap()["pages"].as_u64().unwrap();
    assert!(one > 2 && one < whole, "{one} of {whole}");
}

#[test]
fn the_page_view_draws_pages() {
    struct St {
        s: Session,
        v: PageView,
        styled: bool,
    }
    let mut v = PageView::default();
    v.whole = true;
    let s = session();
    v.build_now(&s, None);
    let n = v.pages().unwrap();
    let mut h = Harness::builder().with_size(egui::vec2(900.0, 700.0)).build_ui_state(
        |ui, st: &mut St| {
            if !st.styled {
                wordsmith_manuscript_ui::brand::apply(ui.ctx(), jcj_shell::theme::Mode::Light);
                st.styled = true;
            }
            pages::view(ui, &st.s, &mut st.v, None);
        },
        St { s, v, styled: false },
    );
    h.run();
    assert!(h.query_by_label(&format!("Page 1 of {n}")).is_some());
    assert!(h.query_by_label_contains(&format!("{n} pages of the whole manuscript")).is_some());
}
