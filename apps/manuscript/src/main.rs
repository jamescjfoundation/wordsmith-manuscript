// SPDX-License-Identifier: MIT OR Apache-2.0
//! Wordsmith Manuscript, the desktop app: the core's shell around Manuscript's views.

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

use std::process::ExitCode;

use jcj_shell::Shell;
use jcj_shell::egui;
use wordsmith_manuscript_ui::{Manuscript, NAME, brand};

struct Window {
    shell: Shell,
    app: Manuscript,
}

impl eframe::App for Window {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.shell.ui(ui, &mut self.app);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        // Save and make the closing backup (FIL-2).
        if let Err(e) = self.app.close() {
            eprintln!("could not save on close: {e}");
        }
    }
}

fn main() -> ExitCode {
    let shell = Shell::new(NAME, env!("CARGO_PKG_VERSION"), brand::tokens())
        .with_options(jcj_shell::ShellOptions { start_window: true, ..jcj_shell::ShellOptions::default() });
    let mut app = Manuscript::new();
    if let Some(path) = std::env::args_os().nth(1) {
        if let Err(e) = app.open(std::path::Path::new(&path)) {
            app.status = e;
        }
    }
    let window = Window { shell, app };
    let mut viewport = egui::ViewportBuilder::default().with_title(NAME).with_app_id("org.jcjfoundation.manuscript");
    if let Ok(icon) = eframe::icon_data::from_png_bytes(brand::ICON_256) {
        viewport = viewport.with_icon(icon);
    }
    let options = eframe::NativeOptions { viewport, ..eframe::NativeOptions::default() };
    match eframe::run_native(NAME, options, Box::new(|_cc| Ok(Box::new(window)))) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("could not open a window: {e}");
            ExitCode::FAILURE
        }
    }
}
