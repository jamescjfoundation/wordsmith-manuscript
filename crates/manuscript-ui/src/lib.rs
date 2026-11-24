// SPDX-License-Identifier: MIT OR Apache-2.0
//! Wordsmith Manuscript's views: the binder, the editor and stitched view, the corkboard, the
//! outline, the inspector, the bible, search, the export dialog, ribbon tabs and panels (layer 4).

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

use jcj_shell::egui;
use jcj_shell::{About, App, Cmd, CommandInfo, Commands, Shell};

/// The product name shown in the window and the About page.
pub const NAME: &str = "Wordsmith Manuscript";

/// The desktop app's state: what the shell draws around.
#[derive(Default)]
pub struct Manuscript {}

impl Manuscript {
    /// An app with no project open.
    pub fn new() -> Manuscript {
        Manuscript {}
    }
}

impl Commands for Manuscript {
    fn command(&self, id: &str) -> Option<CommandInfo> {
        self.all_commands().into_iter().find(|c| c.id == id)
    }

    fn all_commands(&self) -> Vec<CommandInfo> {
        Vec::new()
    }
}

impl App for Manuscript {
    fn document_name(&self) -> String {
        "Untitled".into()
    }

    fn run(&mut self, cmd: &Cmd, _shell: &mut Shell) -> Result<(), String> {
        Err(format!("unknown command {}", cmd.id))
    }

    fn document_ui(&mut self, ui: &mut egui::Ui, _shell: &mut Shell) {
        ui.centered_and_justified(|ui| ui.label("No project open"));
    }

    fn about(&self) -> About {
        About {
            summary: "An open-source app for writing novels and other long work.".into(),
            credit: "Based on WordCraft by the ArtCraft team.".into(),
            ..About::default()
        }
    }
}
