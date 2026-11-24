// SPDX-License-Identifier: MIT OR Apache-2.0
//! Wordsmith Manuscript, the desktop app: the core's shell around Manuscript's views.

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

use std::process::ExitCode;

use jcj_shell::egui;
use jcj_shell::{Shell, theme::Tokens};
use wordsmith_manuscript_ui::{Manuscript, NAME};

struct Window {
    shell: Shell,
    app: Manuscript,
}

impl eframe::App for Window {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.shell.ui(ui, &mut self.app);
    }
}

fn main() -> ExitCode {
    let shell = Shell::new(NAME, env!("CARGO_PKG_VERSION"), Tokens::default());
    let window = Window { shell, app: Manuscript::new() };
    match eframe::run_native(NAME, eframe::NativeOptions::default(), Box::new(|_cc| Ok(Box::new(window)))) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("could not open a window: {e}");
            ExitCode::FAILURE
        }
    }
}
