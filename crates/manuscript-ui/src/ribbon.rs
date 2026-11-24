// SPDX-License-Identifier: MIT OR Apache-2.0
//! Manuscript's ribbon tabs: Write (styles, scene breaks), and the tabs later milestones add.

use jcj_shell::egui::Ui;
use jcj_shell::widgets::{self, Control};
use jcj_shell::{Commands, Shell};
use serde_json::json;
use wordsmith_manuscript_model::doc::BlockTag;

use crate::Manuscript;
use crate::editor::styles::current_block;

/// The Write tab: paragraph styles, character styles, scene breaks.
pub fn write_tab(app: &mut Manuscript, ui: &mut Ui, shell: &mut Shell) {
    let block = app.editor.as_ref().map(current_block).unwrap_or(BlockTag::Body);
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
    widgets::group(ui, shell, "Insert", |ui, shell| {
        widgets::big(ui, shell, cmds, &Control::new("write.sceneBreak").label("Scene Break"));
        widgets::small(ui, shell, cmds, &Control::new("write.lineBreak").label("Line Break"));
    });
}
