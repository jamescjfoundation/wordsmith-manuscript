// SPDX-License-Identifier: MIT OR Apache-2.0
//! Screen readers (ACC-3): the whole window, in every view, has no widget without a name, and the
//! editor reads as a text field named after its document, holding the paragraph at the caret.

use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use jcj_shell::{App, Cmd, Shell, egui};
use wordsmith_manuscript_model::{Area, Item, ItemId, ItemText, Kind, Place, Project, Role, Text};
use wordsmith_manuscript_ui::Manuscript;
use wordsmith_manuscript_ui::session::Session;

fn id(n: u64) -> ItemId {
    ItemId::from_counter(n)
}

struct St {
    shell: Shell,
    app: Manuscript,
    styled: bool,
}

fn window() -> Harness<'static, St> {
    let mut p = Project::new();
    let mut ch = Item::with_id(id(100), Kind::Folder, "Chapter One").role(Role::Chapter);
    for n in 1..=2 {
        ch.children.push(Item::with_id(id(n), Kind::Text, format!("Scene {n}")).role(Role::Scene));
        p.texts.insert(id(n), ItemText::with_body(Text::from_paragraphs(&[&format!("The keeper of scene {n} walked."), "Rain fell."])));
    }
    p.binder.insert(Place::end_of(Area::Manuscript), ch).unwrap();
    let mut app = Manuscript::with_session(Session::new(p));
    app.binder.selected = Some(id(1));
    app.open_item(id(1));
    let shell = Shell::new("Test", "0", wordsmith_manuscript_ui::brand::BRAND.tokens());
    Harness::builder().with_size(egui::vec2(1400.0, 900.0)).build_ui_state(
        |ui, st: &mut St| {
            if !st.styled {
                wordsmith_manuscript_ui::brand::BRAND.apply(ui.ctx(), jcj_shell::theme::Mode::Light);
                st.styled = true;
            }
            st.shell.ui(ui, &mut st.app);
        },
        St { shell, app, styled: false },
    )
}

#[test]
fn the_editor_reads_as_a_named_text_field() {
    let mut h = window();
    h.run();
    h.get_by_label("Binder").click();
    h.run();
    let editor = h.get_by_label("Text of “Scene 1”");
    assert_eq!(editor.accesskit_node().role(), egui::accesskit::Role::MultilineTextInput);
    assert_eq!(editor.accesskit_node().value().as_deref(), Some("The keeper of scene 1 walked."));
    let labels: Vec<String> = h.root().children_recursive().filter_map(|n| n.accesskit_node().label()).collect();
    assert!(labels.iter().any(|l| l.contains("Chapter One")), "binder rows are named: {labels:?}");
}

#[test]
fn every_view_names_its_widgets() {
    let mut h = window();
    h.run();
    // With the binder and the inspector open.
    h.get_by_label("Binder").click();
    h.run();
    h.get_by_label("Inspector").click();
    h.run();
    for view in ["view.corkboard", "view.outline", "view.board", "view.pages", "view.lanes", "view.timeline", "view.split", "view.editor"] {
        {
            let st = h.state_mut();
            let shell = &mut st.shell;
            st.app.run(&Cmd::new(view), shell).unwrap();
        }
        // The harness fails on any widget a screen reader could not name.
        h.run();
    }
    let st = h.state_mut();
    st.app.run(&Cmd::with("view.stitch", serde_json::json!({ "id": id(100).as_text() })), &mut st.shell).unwrap();
    h.run();
    assert!(h.query_by_label("Text of “Scene 1” and the documents after it").is_some());
}
