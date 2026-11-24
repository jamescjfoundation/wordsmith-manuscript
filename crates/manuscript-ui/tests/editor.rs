// SPDX-License-Identifier: MIT OR Apache-2.0
//! The editor: Manuscript's styles round-trip through the format, nothing else can be applied,
//! Return and smart typing behave, and the view types in the egui harness.

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use jcj_shell::egui;
use serde_json::json;
use wordsmith_manuscript_format::dialect;
use wordsmith_manuscript_model::{Block, Fence, Inline, ItemId, Marks, Text};
use wordsmith_manuscript_ui::editor::{Editor, EditorLook, pos, registry};

fn editor(paras: &[&str]) -> Editor {
    Editor::new(ItemId::from_counter(1), &Text::from_paragraphs(paras), registry())
}

/// The text survives the file format exactly.
fn through_format(t: &Text) -> Text {
    let written = dialect::write(t);
    let back = dialect::parse(&written);
    assert_eq!(dialect::write(&back), written);
    back
}

fn select(ed: &mut Editor, (pa, oa): (usize, usize), (pb, ob): (usize, usize)) {
    ed.session.sel = jcj_engine::Selection { anchor: pos(pa, oa), focus: pos(pb, ob) };
}

#[test]
fn every_style_round_trips_through_the_format() {
    let mut ed = editor(&["Heading", "Quoted one", "Quoted two", "Verse line", "A letter", "Body with italic bold caps gone français"]);
    select(&mut ed, (0, 0), (0, 0));
    ed.command("write.heading", &json!({ "level": 2 })).unwrap();
    select(&mut ed, (1, 0), (2, 3));
    ed.command("write.quote", &json!({})).unwrap();
    select(&mut ed, (3, 0), (3, 0));
    ed.command("write.verse", &json!({})).unwrap();
    select(&mut ed, (4, 0), (4, 0));
    ed.command("write.aside", &json!({ "kind": "letter" })).unwrap();
    let body = 5;
    for (word, cmd) in [("italic", "write.italic"), ("bold", "write.bold"), ("caps", "write.smallCaps"), ("gone", "write.strike")] {
        let text = ed.paragraph(body).unwrap().text.clone();
        let at = text.find(word).unwrap();
        select(&mut ed, (body, at), (body, at + word.len()));
        ed.command(cmd, &json!({})).unwrap();
    }
    let text = ed.paragraph(body).unwrap().text.clone();
    let at = text.find("français").unwrap();
    select(&mut ed, (body, at), (body, text.len()));
    ed.command("write.language", &json!({ "lang": "fr" })).unwrap();
    // A scene break after the verse.
    ed.set_caret(3, "Verse line".len(), false);
    ed.command("write.sceneBreak", &json!({})).unwrap();
    let t = ed.text();
    assert_eq!(through_format(&t), t);
    let kinds: Vec<&str> = t
        .blocks
        .iter()
        .map(|b| match b {
            Block::Heading { .. } => "heading",
            Block::Quote(ps) if ps.len() == 2 => "quote of two",
            Block::Fenced { kind: Fence::Verse, .. } => "verse",
            Block::Fenced { kind: Fence::Aside(Some(k)), .. } if k == "letter" => "letter",
            Block::SceneBreak => "break",
            Block::Para(_) => "body",
            _ => "other",
        })
        .collect();
    assert_eq!(kinds, ["heading", "quote of two", "verse", "break", "letter", "body"]);
    let Some(Block::Para(last)) = t.blocks.last() else { panic!("a body paragraph") };
    let marked: Vec<(String, Marks)> =
        last.iter().filter_map(|i| if let Inline::Span { text, marks } = i { Some((text.clone(), marks.clone())) } else { None }).collect();
    assert!(marked.iter().any(|(t, m)| t == "italic" && m.italic));
    assert!(marked.iter().any(|(t, m)| t == "bold" && m.bold));
    assert!(marked.iter().any(|(t, m)| t == "caps" && m.small_caps));
    assert!(marked.iter().any(|(t, m)| t == "gone" && m.strike));
    assert!(marked.iter().any(|(t, m)| t == "français" && m.lang.as_deref() == Some("fr")));
    // Undo walks every style change back.
    for _ in 0..30 {
        let _ = ed.run("edit.undo", &json!({}));
    }
    assert_eq!(
        ed.text(),
        Text::from_paragraphs(&["Heading", "Quoted one", "Quoted two", "Verse line", "A letter", "Body with italic bold caps gone français"])
    );
}

#[test]
fn styles_outside_the_dialect_cannot_be_applied() {
    let mut ed = editor(&["Some words"]);
    select(&mut ed, (0, 0), (0, 4));
    // Formatting the core's engine knows but the dialect does not hold is dropped when read back.
    ed.run("format.underline", &json!({})).unwrap();
    ed.run("format.font", &json!({ "font": "Some Face" })).ok();
    assert_eq!(ed.text(), Text::from_paragraphs(&["Some words"]));
    // The app offers only Manuscript's styles.
    let app = wordsmith_manuscript_ui::Manuscript::new();
    let ids: Vec<String> = jcj_shell::Commands::all_commands(&app).into_iter().map(|c| c.id).collect();
    assert!(ids.iter().all(|id| !id.starts_with("format.")), "no raw formatting commands are offered");
    // Pasted text comes in plain, line breaks as paragraphs.
    ed.set_caret(0, 10, false);
    ed.paste(" and\r\nmore").unwrap();
    assert_eq!(ed.text(), Text::from_paragraphs(&["Some words and", "more"]));
}

#[test]
fn return_after_headings_and_breaks_starts_body_text() {
    let mut ed = editor(&["Title"]);
    ed.command("write.heading", &json!({ "level": 1 })).unwrap();
    ed.set_caret(0, 5, false);
    ed.new_paragraph().unwrap();
    ed.type_text("Body").unwrap();
    let t = ed.text();
    assert_eq!(t.blocks[1], Block::Para(vec![Inline::plain("Body")]));
    // Return on an empty quote paragraph ends the quote.
    ed.command("write.quote", &json!({})).unwrap();
    ed.new_paragraph().unwrap();
    ed.new_paragraph().unwrap();
    let t = ed.text();
    assert_eq!(t.blocks.last(), Some(&Block::Para(vec![])));
}

#[test]
fn smart_typing_and_backspace_undoing_it() {
    let mut ed = editor(&[""]);
    for c in "\"Wait--what's that...\"".chars() {
        ed.type_text(&c.to_string()).unwrap();
    }
    assert_eq!(ed.paragraph(0).unwrap().text, "“Wait—what’s that…”");
    // Backspace right after a replacement puts back what was typed.
    let mut ed = editor(&[""]);
    for c in "a--".chars() {
        ed.type_text(&c.to_string()).unwrap();
    }
    assert_eq!(ed.paragraph(0).unwrap().text, "a—");
    let r = ed.last_smart.clone().unwrap();
    wordsmith_manuscript_ui::typing::undo_smart(&mut ed, &r).unwrap();
    assert_eq!(ed.paragraph(0).unwrap().text, "a--");
}

#[test]
fn counts_for_document_and_selection() {
    let mut ed = editor(&["One two three", "four five"]);
    assert_eq!(ed.counts(), (5, 0));
    select(&mut ed, (0, 4), (1, 4));
    assert_eq!(ed.counts(), (5, 3));
}

struct State {
    ed: Editor,
    styled: bool,
}

#[test]
fn the_view_shows_text_and_takes_typing() {
    let mut h = Harness::builder().with_size(egui::vec2(900.0, 600.0)).build_ui_state(
        |ui, st: &mut State| {
            if !st.styled {
                wordsmith_manuscript_ui::brand::apply(ui.ctx(), jcj_shell::theme::Mode::Light);
                st.styled = true;
            }
            let r = st.ed.ui(ui, &EditorLook::default());
            r.widget_info(|| egui::WidgetInfo::labeled(egui::Role::TextInput, true, "Manuscript text"));
        },
        State { ed: editor(&["It was a dark night."]), styled: false },
    );
    h.run();
    h.get_by_label("Manuscript text").click();
    h.run();
    h.state_mut().ed.set_caret(0, 0, false);
    h.get_by_label("Manuscript text").type_text("Truly, ");
    h.run();
    assert_eq!(h.state().ed.paragraph(0).unwrap().text, "Truly, It was a dark night.");
}

#[test]
fn smart_typing_follows_the_project_settings() {
    let mut s = wordsmith_manuscript_ui::session::Session::new(wordsmith_manuscript_model::Project::new());
    s.run("settings.typing", &json!({ "quotes": false, "language": "de" })).unwrap();
    let smart = wordsmith_manuscript_ui::typing::Smart::of(&s.project.settings);
    assert!(!smart.quotes && smart.dashes && smart.lang == "de");
    assert!(s.run("settings.typing", &json!({ "language": "not a tag" })).is_err());
    s.undo();
    assert!(wordsmith_manuscript_ui::typing::Smart::of(&s.project.settings).quotes);
}
