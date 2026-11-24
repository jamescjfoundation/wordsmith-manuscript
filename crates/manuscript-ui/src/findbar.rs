// SPDX-License-Identifier: MIT OR Apache-2.0
//! The find bar above the editor: find and replace in the document, the selection or the project.

use jcj_shell::egui::{self, Ui};
use serde_json::json;
use wordsmith_manuscript_model::ItemId;

use crate::Manuscript;
use crate::find::{Found, Query, find_in_project};

/// Where the find bar looks.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Scope {
    /// The open document.
    #[default]
    Document,
    /// The selection in the open document.
    Selection,
    /// Every text in the project.
    Project,
}

/// The find bar's state.
#[derive(Default)]
pub struct FindBar {
    /// Shown.
    pub open: bool,
    /// What to find.
    pub query: Query,
    /// What to replace it with.
    pub with: String,
    /// Where.
    pub scope: Scope,
    /// Project matches from the last search.
    pub results: Vec<Found>,
    /// What the last action did.
    pub message: String,
    focus: bool,
}

impl FindBar {
    /// Show the bar with the keyboard in it.
    pub fn show(&mut self) {
        self.open = true;
        self.focus = true;
    }
}

/// Draw the find bar (when open).
pub fn ui(app: &mut Manuscript, ui: &mut Ui) {
    if !app.find.open {
        return;
    }
    let mut action: Option<&str> = None;
    ui.horizontal_wrapped(|ui| {
        let bar = &mut app.find;
        let r = ui.add(egui::TextEdit::singleline(&mut bar.query.text).hint_text("Find").desired_width(220.0));
        if bar.focus {
            r.request_focus();
            bar.focus = false;
        }
        if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            action = Some("next");
        }
        ui.add(egui::TextEdit::singleline(&mut bar.with).hint_text("Replace with").desired_width(180.0));
        ui.checkbox(&mut bar.query.case, "Match case");
        ui.checkbox(&mut bar.query.word, "Whole word");
        ui.checkbox(&mut bar.query.regex, "Pattern");
        egui::ComboBox::from_id_salt("manuscript-find-scope")
            .selected_text(match bar.scope {
                Scope::Document => "Document",
                Scope::Selection => "Selection",
                Scope::Project => "Project",
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut bar.scope, Scope::Document, "Document");
                ui.selectable_value(&mut bar.scope, Scope::Selection, "Selection");
                ui.selectable_value(&mut bar.scope, Scope::Project, "Project");
            });
        if ui.button("Previous").clicked() {
            action = Some("previous");
        }
        if ui.button("Next").clicked() {
            action = Some("next");
        }
        if ui.button("Replace").clicked() {
            action = Some("replace");
        }
        if ui.button("Replace All").clicked() {
            action = Some("all");
        }
        if ui.button("Close").clicked() || ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            bar.open = false;
        }
        if !bar.message.is_empty() {
            ui.label(&bar.message);
        }
    });
    if let Some(a) = action {
        let r = act(app, a);
        app.find.message = match r {
            Ok(m) => m,
            Err(e) => e,
        };
    }
    results(app, ui);
}

fn act(app: &mut Manuscript, a: &str) -> Result<String, String> {
    let q = app.find.query.clone();
    let with = app.find.with.clone();
    match (app.find.scope, a) {
        (Scope::Project, "all") => {
            app.write_back();
            let s = app.session.as_mut().ok_or("no project is open")?;
            let out = s.run("find.replaceProject", &json!({ "query": q.text, "with": with, "case": q.case, "word": q.word, "regex": q.regex }))?;
            app.reload_editor();
            Ok(format!("Replaced {} in {} documents (a snapshot of each was taken first).", out["replaced"], out["items"]))
        }
        (Scope::Project, _) => {
            app.write_back();
            let s = app.session.as_ref().ok_or("no project is open")?;
            let re = q.matcher()?;
            app.find.results = find_in_project(&s.project, &re);
            Ok(format!("{} matches in the project.", app.find.results.len()))
        }
        (scope, "all") => {
            let ed = app.editor.as_mut().ok_or("open a document first")?;
            let n = ed.replace_all(&q, &with, scope == Scope::Selection)?;
            Ok(format!("Replaced {n}."))
        }
        (_, "replace") => {
            let ed = app.editor.as_mut().ok_or("open a document first")?;
            ed.replace_one(&q, &with)?;
            Ok(String::new())
        }
        (_, dir) => {
            let ed = app.editor.as_mut().ok_or("open a document first")?;
            let found = ed.find_next(&q, dir == "previous")?;
            Ok(if found { String::new() } else { "Not found.".into() })
        }
    }
}

fn results(app: &mut Manuscript, ui: &mut Ui) {
    if app.find.scope != Scope::Project || app.find.results.is_empty() {
        return;
    }
    let mut open: Option<ItemId> = None;
    egui::ScrollArea::vertical().id_salt("manuscript-find-results").max_height(160.0).show(ui, |ui| {
        let Some(s) = app.session.as_ref() else { return };
        for f in app.find.results.iter().take(500) {
            let title = s.project.binder.get(f.item).map(|i| i.title.clone()).unwrap_or_default();
            if ui.link(format!("{title} ({}): {}", f.field, f.context)).clicked() {
                open = Some(f.item);
            }
        }
    });
    if let Some(id) = open {
        app.binder.selected = Some(id);
        app.binder.tree.reveal = true;
        app.open_item(id);
        let q = app.find.query.clone();
        if let Some(ed) = app.editor.as_mut() {
            ed.set_caret(0, 0, false);
            let _ = ed.find_next(&q, false);
        }
    }
}
