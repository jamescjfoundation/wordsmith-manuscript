// SPDX-License-Identifier: MIT OR Apache-2.0
//! Series with a shared bible (BND-8, BIB-5): the commands and the Series window. See
//! `manuscript-model::series` and decision 0007.

use std::path::{Path, PathBuf};

use jcj_shell::egui::{self, RichText};
use serde_json::{Value, json};

use crate::commands::{Spec, str_param};
use crate::session::{Outcome, Session};

/// The series commands.
pub fn specs() -> Vec<Spec> {
    let spec = |id, label, params, about, mutates| Spec { id, label, key: "", params, about, mutates };
    vec![
        spec(
            "series.create",
            "Start a Series",
            r#"{"path": string, "name": string}"#,
            "Make a series bible folder at `path` from this book's Story Bible, and share it from now on.",
            true,
        ),
        spec("series.join", "Join a Series", r#"{"path": string}"#, "Share the series bible at `path`; this book's own entries join it.", true),
        spec("series.leave", "Leave the Series", "{}", "Stop sharing the series bible; this book keeps a copy of it as its own.", true),
        spec("series.info", "Series", "{}", "The series this book is in: name, folder and books.", false),
    ]
}

/// Run a series command, or `None` if `id` is not one.
pub fn run(s: &mut Session, id: &str, v: &Value) -> Option<Outcome> {
    Some(match id {
        "series.create" => (|| {
            let path = PathBuf::from(str_param(v, "path")?);
            let name = str_param(v, "name")?.trim().to_owned();
            if name.is_empty() {
                return Err("a series needs a name".into());
            }
            s.create_series(&path, &name)?;
            Ok(json!({ "series": s.project.settings.series }))
        })(),
        "series.join" => (|| {
            let path = PathBuf::from(str_param(v, "path")?);
            s.join_series(&path)?;
            Ok(json!({ "series": s.project.settings.series }))
        })(),
        "series.leave" => s.leave_series().map(|()| json!({})),
        "series.info" => Ok(match (s.series_info(), s.series_root()) {
            (Some(info), Some(root)) => json!({ "name": info.name, "folder": root.to_string_lossy(), "books": info.books }),
            _ => json!({ "name": null, "problem": s.series_problem }),
        }),
        _ => return None,
    })
}

/// The Series window's state.
#[derive(Default)]
pub struct SeriesView {
    /// Whether the window is open.
    pub open: bool,
    name: String,
    path: String,
}

/// Draw the Series window. Returns errors, and a book to open (its folder).
pub fn window(ctx: &egui::Context, s: &mut Session, v: &mut SeriesView) -> (Vec<String>, Option<PathBuf>) {
    let mut errors = Vec::new();
    let mut open_book = None;
    if !v.open {
        return (errors, open_book);
    }
    let mut open = true;
    let mut todo: Option<(&str, Value)> = None;
    egui::Window::new(jcj_shell::i18n::tr("Series")).id(egui::Id::unique("manuscript-series")).open(&mut open).default_size([520.0, 360.0]).show(
        ctx,
        |ui| {
            if let Some(p) = &s.series_problem {
                ui.label(RichText::new(p).color(egui::Color32::from_rgb(0xC0, 0x39, 0x2B)));
            }
            match (s.series_info(), s.series_root().map(Path::to_path_buf)) {
                (Some(info), Some(root)) => {
                    ui.label(RichText::new(&info.name).heading());
                    ui.label(RichText::new(format!("The Story Bible of every book in this series is kept in {}.", root.display())).weak());
                    ui.separator();
                    for b in &info.books {
                        ui.horizontal(|ui| {
                            let book = root.join(b);
                            let name = book.file_stem().map_or_else(|| b.clone(), |n| n.to_string_lossy().into_owned());
                            ui.label(&name);
                            let this = s.root().is_some_and(|r| std::fs::canonicalize(r).ok() == std::fs::canonicalize(&book).ok());
                            if this {
                                ui.label(RichText::new("this book").weak());
                            } else if ui.small_button("Open").clicked() {
                                open_book = Some(book);
                            }
                        });
                    }
                    ui.separator();
                    if ui.button("Leave the Series").on_hover_text("This book keeps a copy of the series bible as its own").clicked() {
                        todo = Some(("series.leave", json!({})));
                    }
                }
                _ => {
                    ui.label("This book keeps its own Story Bible. A series shares one bible between several books.");
                    ui.add_space(6.0);
                    let l = ui.label("Series name");
                    ui.add(egui::TextEdit::singleline(&mut v.name).desired_width(300.0)).labelled_by(l.id);
                    let l = ui.label("Folder for the series bible (to start one) or of an existing series bible (to join it)");
                    ui.add(egui::TextEdit::singleline(&mut v.path).desired_width(420.0)).labelled_by(l.id);
                    ui.horizontal(|ui| {
                        if ui.button("Start a Series").clicked() && !v.path.trim().is_empty() {
                            todo = Some(("series.create", json!({ "path": v.path.trim(), "name": v.name.trim() })));
                        }
                        if ui.button("Join a Series").clicked() && !v.path.trim().is_empty() {
                            todo = Some(("series.join", json!({ "path": v.path.trim() })));
                        }
                    });
                }
            }
        },
    );
    if let Some((cmd, args)) = todo
        && let Err(e) = s.run(cmd, &args)
    {
        errors.push(e);
    }
    if !open {
        v.open = false;
    }
    (errors, open_book)
}
