// SPDX-License-Identifier: MIT OR Apache-2.0
//! Focus mode, typewriter scrolling, the sepia page and the text column (ED-3, ACC-1).

use jcj_shell::{App, Cmd, Shell};
use serde_json::json;
use wordsmith_manuscript_model::Project;
use wordsmith_manuscript_ui::Manuscript;
use wordsmith_manuscript_ui::brand::EditorSurface;
use wordsmith_manuscript_ui::session::Session;
use wordsmith_manuscript_ui::view::ViewPrefs;

fn shell() -> Shell {
    Shell::new("Test", "0", wordsmith_manuscript_ui::brand::tokens())
}

#[test]
fn focus_mode_hides_the_chrome_and_panels_and_puts_them_back() {
    let mut app = Manuscript::with_session(Session::new(Project::new()));
    let mut sh = shell();
    assert!(!app.panels().is_empty());
    app.run(&Cmd::new("view.focus"), &mut sh).unwrap();
    assert!(app.focus.is_some());
    assert!(!sh.options.ribbon && !sh.options.status_bar && !sh.options.quick_row);
    assert!(app.panels().is_empty());
    app.run(&Cmd::new("view.focus"), &mut sh).unwrap();
    assert!(app.focus.is_none());
    assert!(sh.options.ribbon && sh.options.status_bar && sh.options.quick_row);
    assert!(!app.panels().is_empty());
}

#[test]
fn the_page_and_column_follow_the_view_preferences() {
    let mut app = Manuscript::new();
    let mut sh = shell();
    app.run(&Cmd::new("view.sepia"), &mut sh).unwrap();
    assert_eq!(app.view.surface, EditorSurface::Sepia);
    assert_eq!(app.view.look(true, false).surface, EditorSurface::Sepia);
    app.run(&Cmd::with("view.textSize", json!({ "size": 400 })), &mut sh).unwrap();
    assert_eq!(app.view.size, wordsmith_manuscript_ui::view::SIZE.1);
    app.run(&Cmd::with("view.width", json!({ "by": -5000 })), &mut sh).unwrap();
    assert_eq!(app.view.width, wordsmith_manuscript_ui::view::WIDTH.0);
    app.run(&Cmd::new("view.typewriter"), &mut sh).unwrap();
    assert!(app.view.typewriter);
    // A damaged preferences file cannot ask for absurd values.
    let p: ViewPrefs = serde_json::from_str(r#"{"size": 1e9, "width": -3}"#).unwrap();
    let p = p.bounded();
    assert!(p.size <= 36.0 && p.width >= 360.0);
    // Dimming applies only in focus mode.
    app.view.dim_in_focus = true;
    assert!(!app.view.look(false, false).dim_others);
    assert!(app.view.look(false, true).dim_others);
}

#[test]
fn sepia_is_its_own_page_in_either_theme() {
    use wordsmith_manuscript_ui::brand::page_colors;
    assert_eq!(page_colors(EditorSurface::Sepia, false), page_colors(EditorSurface::Sepia, true));
    assert_ne!(page_colors(EditorSurface::Theme, false).page, page_colors(EditorSurface::Theme, true).page);
}
