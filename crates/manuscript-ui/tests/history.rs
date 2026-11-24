// SPDX-License-Identifier: MIT OR Apache-2.0
//! Writing history (PRG-3): words per day kept on this computer, outside the project, and shown
//! as a calendar.

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use jcj_shell::egui;
use jcj_shell::writing::{Day, History};
use wordsmith_manuscript_model::{Area, Item, ItemId, ItemText, Kind, Place, Project, Text};
use wordsmith_manuscript_ui::Manuscript;
use wordsmith_manuscript_ui::history::{self, HistoryView};
use wordsmith_manuscript_ui::session::Session;

fn scratch(name: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("wsm-history-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

#[test]
fn words_written_are_kept_beside_the_preferences_not_in_the_project() {
    let base = scratch("keep");
    let root = base.join("Novel");
    let mut p = Project::new();
    let id = ItemId::from_counter(1);
    p.binder.insert(Place::end_of(Area::Manuscript), Item::with_id(id, Kind::Text, "Scene")).unwrap();
    p.texts.insert(id, ItemText::with_body(Text::from_paragraphs(&["One two three."])));
    let s = Session::create(&root, p).unwrap();
    let mut app = Manuscript::with_session(s);
    app.history_dir = Some(base.join("settings/history"));
    app.start_progress();
    app.session.as_mut().unwrap().run("text.set", &serde_json::json!({ "id": id.as_text(), "text": "One two three four five.\n" })).unwrap();
    let s = app.session.as_ref().unwrap();
    app.progress.as_mut().unwrap().observe(s);
    app.close().unwrap();
    let today = wordsmith_manuscript_ui::progress::today();
    let kept = history::load(&base.join("settings/history"), &root);
    assert_eq!(kept.on(today), 2);
    // Nothing of it is in the project folder, and the file name says nothing about the project.
    let files: Vec<String> = std::fs::read_dir(&root).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
    assert!(!files.iter().any(|f| f.contains("history")), "{files:?}");
    let name = history::file_for(&base.join("settings/history"), &root);
    assert!(!name.to_string_lossy().contains("Novel"));
    // A new session on the project carries on from it.
    let mut again = Manuscript::with_session(Session::open(&root).unwrap());
    again.history_dir = Some(base.join("settings/history"));
    again.start_progress();
    assert_eq!(again.progress.as_ref().unwrap().writing.history.on(today), 2);
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn the_calendar_shows_a_month() {
    let mut h = History::default();
    let d = |day| Day { year: 2026, month: 3, day };
    h.add(d(2), 500);
    h.add(d(3), 1200);
    h.add(d(4), -40);
    let (total, days, best) = history::month_totals(&h, 2026, 3);
    assert_eq!((total, days, best), (1660, 2, Some((d(3), 1200))));
    let mut harness = Harness::builder().with_size(egui::vec2(520.0, 420.0)).build_ui_state(
        |ui, st: &mut (History, HistoryView, bool)| {
            if !st.2 {
                wordsmith_manuscript_ui::brand::apply(ui.ctx(), jcj_shell::theme::Mode::Light);
                st.2 = true;
            }
            history::calendar(ui, &st.0, &mut st.1, 2026, 3, Day { year: 2026, month: 3, day: 3 });
        },
        (h, HistoryView::default(), false),
    );
    harness.run();
    assert!(harness.query_by_label("March 3: 1200 words").is_some());
    assert!(harness.query_by_label("March 4: -40 words").is_some());
    assert!(harness.query_by_label_contains("1,660 words on 2 days").is_some());
}
