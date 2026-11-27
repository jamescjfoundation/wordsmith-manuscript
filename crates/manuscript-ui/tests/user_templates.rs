// SPDX-License-Identifier: MIT OR Apache-2.0
//! Project templates the writer makes (BND-4): kept on this computer, offered on the New page,
//! and each project made from one is its own (fresh ids, none of the first book's words).

use jcj_shell::{App, Cmd, Shell};
use serde_json::json;
use wordsmith_manuscript_model::{Area, ItemId, Role, Text, templates};
use wordsmith_manuscript_ui::Manuscript;
use wordsmith_manuscript_ui::session::Session;

#[test]
fn save_as_template_and_start_a_new_project_from_it() {
    let base = std::env::temp_dir().join(format!("wsm-user-templates-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    let mut p = templates::build("novel", "The River", &mut ItemId::new).unwrap();
    let scene = p.binder.rows_of(Area::Manuscript).iter().find(|r| r.item.role == Role::Scene).map(|r| r.item.id).unwrap();
    p.text_mut(scene).body = Text::from_paragraphs(&["Words of the first book."]);
    p.text_mut(scene).meta.synopsis = "Inciting incident here.".into();
    p.settings.labels.push(wordsmith_manuscript_model::Named::new("Subplot", "#336699"));
    let mut app = Manuscript::with_session(Session::new(p));
    app.templates_dir = Some(base.join("settings/templates"));
    let mut sh = Shell::new("Test", "0", wordsmith_manuscript_ui::brand::BRAND.tokens());
    app.run(&Cmd::with("file.saveTemplate", json!({ "name": "Three Acts", "about": "My usual shape" })), &mut sh).unwrap();
    let offered = app.templates();
    let mine = offered.iter().find(|t| t.group == "Your Templates").expect("offered on the New page");
    assert_eq!((mine.name.as_str(), mine.about.as_str()), ("Three Acts", "My usual shape"));
    let id = mine.id.clone();
    let root = base.join("Second Book");
    app.new_from_template(&id, &root).unwrap();
    let s = app.session.as_ref().unwrap();
    assert_eq!(s.project.settings.book.title, "Second Book");
    assert!(s.project.settings.labels.iter().any(|l| l.name == "Subplot"));
    let new_scene = s.project.binder.rows_of(Area::Manuscript).iter().find(|r| r.item.role == Role::Scene).map(|r| r.item.id).unwrap();
    assert_ne!(new_scene, scene, "fresh ids");
    let t = s.project.text(new_scene).unwrap();
    assert!(t.body.is_empty(), "none of the first book's words");
    assert_eq!(t.meta.synopsis, "Inciting incident here.");
    assert!(root.join("project.toml").exists());
    app.run(&Cmd::with("file.deleteTemplate", json!({ "template": id })), &mut sh).unwrap();
    assert!(!app.templates().iter().any(|t| t.group == "Your Templates"));
    let _ = std::fs::remove_dir_all(&base);
}
