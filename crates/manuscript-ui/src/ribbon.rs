// SPDX-License-Identifier: MIT OR Apache-2.0
//! Manuscript's ribbon tabs: Write (styles, scene breaks), and the tabs later milestones add.

use jcj_shell::egui::{self, Ui};
use jcj_shell::widgets::{self, Control};
use jcj_shell::{Commands, Shell};
use serde_json::json;
use wordsmith_manuscript_model::doc::BlockTag;

use crate::Manuscript;
use crate::editor::styles::current_block;

/// The Write tab: paragraph styles, character styles, scene breaks.
pub fn write_tab(app: &mut Manuscript, ui: &mut Ui, shell: &mut Shell) {
    let block = app.editor.as_ref().map(current_block).unwrap_or(BlockTag::Body);
    let targets: Vec<(String, String)> = app
        .session
        .as_ref()
        .map(|s| {
            let mut v: Vec<(String, String)> = s
                .project
                .binder
                .rows_of(wordsmith_manuscript_model::Area::Bible)
                .iter()
                .filter(|r| r.item.kind == wordsmith_manuscript_model::Kind::Entry)
                .map(|r| (r.item.title.clone(), format!("wsm:{}", r.item.id)))
                .collect();
            v.extend(
                s.project
                    .binder
                    .rows_of(wordsmith_manuscript_model::Area::Manuscript)
                    .iter()
                    .filter(|r| r.item.kind.has_text())
                    .map(|r| (r.item.title.clone(), format!("wsm:{}", r.item.id))),
            );
            v
        })
        .unwrap_or_default();
    let cmds: &dyn Commands = &*app;
    widgets::group(ui, shell, "Paragraph", |ui, shell| {
        let on = |b: bool| b;
        widgets::small(ui, shell, cmds, &Control::new("write.body").label("Body").on(on(block == BlockTag::Body)));
        widgets::small(ui, shell, cmds, &Control::new("write.quote").label("Quote").on(matches!(block, BlockTag::Quote { .. })));
        widgets::small(ui, shell, cmds, &Control::new("write.verse").label("Verse").on(matches!(block, BlockTag::Verse { .. })));
        widgets::small(ui, shell, cmds, &Control::new("write.aside").label("Aside").on(matches!(block, BlockTag::Aside { .. })));
        for level in 1..=3u8 {
            let c =
                Control::with("write.heading", json!({ "level": level })).label(format!("Heading {level}")).on(block == BlockTag::Heading { level });
            widgets::small(ui, shell, cmds, &c);
        }
    });
    widgets::group(ui, shell, "Characters", |ui, shell| {
        widgets::small(ui, shell, cmds, &Control::new("write.italic").label("Italic"));
        widgets::small(ui, shell, cmds, &Control::new("write.bold").label("Bold"));
        widgets::small(ui, shell, cmds, &Control::new("write.smallCaps").label("Small Caps"));
        widgets::small(ui, shell, cmds, &Control::new("write.strike").label("Strikethrough"));
        widgets::small(ui, shell, cmds, &Control::new("write.plain").label("Plain"));
    });
    widgets::group(ui, shell, "Link", |ui, shell| {
        widgets::drop(ui, shell, "Link to…", "Link the selected words to a bible entry or a document.", |ui, shell| {
            egui::ScrollArea::vertical().max_height(320.0).show(ui, |ui| {
                for (title, target) in &targets {
                    if ui.button(title).clicked() {
                        shell.run(jcj_shell::Cmd::with("write.link", json!({ "target": target })));
                        ui.close();
                    }
                }
            });
        });
        widgets::small(ui, shell, cmds, &Control::new("write.unlink").label("Remove Link"));
    });
    widgets::group(ui, shell, "Read", |ui, shell| {
        widgets::small(ui, shell, cmds, &Control::new("view.stitch").label("As One Text"));
        widgets::small(ui, shell, cmds, &Control::new("view.stitchManuscript").label("Whole Manuscript"));
    });
    widgets::group(ui, shell, "Insert", |ui, shell| {
        widgets::big(ui, shell, cmds, &Control::new("write.sceneBreak").label("Scene Break"));
        widgets::small(ui, shell, cmds, &Control::new("write.lineBreak").label("Line Break"));
    });
}

/// The View tab: focus mode, themes, the page and the text column.
pub fn view_tab(app: &mut Manuscript, ui: &mut Ui, shell: &mut Shell) {
    let focus = app.focus.is_some();
    let sepia = app.view.surface == crate::brand::EditorSurface::Sepia;
    let typewriter = app.view.typewriter;
    let dim = app.view.dim_in_focus;
    let cmds: &dyn Commands = &*app;
    let mode = app.mode;
    let cmds: &dyn Commands = &*app;
    widgets::group(ui, shell, "Show", |ui, shell| {
        widgets::small(ui, shell, cmds, &Control::new("view.editor").label("Editor").on(mode == crate::ViewMode::Editor));
        widgets::small(ui, shell, cmds, &Control::new("view.corkboard").label("Corkboard").on(mode == crate::ViewMode::Corkboard));
        widgets::small(ui, shell, cmds, &Control::new("view.outline").label("Outline").on(mode == crate::ViewMode::Outline));
    });
    widgets::group(ui, shell, "Focus", |ui, shell| {
        widgets::big(ui, shell, cmds, &Control::new("view.focus").label("Focus Mode").on(focus));
        widgets::small(ui, shell, cmds, &Control::new("view.typewriter").label("Typewriter Scrolling").on(typewriter));
        widgets::small(ui, shell, cmds, &Control::new("view.dimAround").label("Dim Around the Caret").on(dim));
    });
    widgets::group(ui, shell, "Theme", |ui, shell| {
        let theme = jcj_shell::commands::shell_ids::THEME;
        widgets::small(ui, shell, cmds, &Control::with(theme, json!({ "mode": "light" })).label("Light"));
        widgets::small(ui, shell, cmds, &Control::with(theme, json!({ "mode": "dark" })).label("Dark"));
        widgets::small(ui, shell, cmds, &Control::new("view.sepia").label("Sepia Page").on(sepia));
    });
    widgets::group(ui, shell, "Text", |ui, shell| {
        widgets::small(ui, shell, cmds, &Control::with("view.textSize", json!({ "by": 1 })).label("Larger"));
        widgets::small(ui, shell, cmds, &Control::with("view.textSize", json!({ "by": -1 })).label("Smaller"));
        widgets::small(ui, shell, cmds, &Control::with("view.width", json!({ "by": 60 })).label("Wider"));
        widgets::small(ui, shell, cmds, &Control::with("view.width", json!({ "by": -60 })).label("Narrower"));
    });
}

/// The Bible tab: a new entry of each type.
pub fn bible_tab(app: &mut Manuscript, ui: &mut Ui, shell: &mut Shell) {
    let templates = app.session.as_ref().map(|s| wordsmith_manuscript_model::bible::templates(&s.project.settings)).unwrap_or_default();
    let cmds: &dyn Commands = &*app;
    widgets::group(ui, shell, "New Entry", |ui, shell| {
        for t in templates {
            let c = Control::with("bible.new", json!({ "kind": t.kind, "name": format!("New {}", t.title) })).label(t.title.clone());
            widgets::small(ui, shell, cmds, &c);
        }
    });
}
