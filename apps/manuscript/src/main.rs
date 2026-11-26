// SPDX-License-Identifier: MIT OR Apache-2.0
//! Wordsmith Manuscript, the desktop app: the core's shell around Manuscript's views.

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

use std::process::ExitCode;
use std::time::Instant;

use jcj_shell::egui;
use jcj_shell::prefs::{Prefs, PrefsFile};
use jcj_shell::{App, Shell};
use wordsmith_manuscript_ui::view::ViewPrefs;
use wordsmith_manuscript_ui::{Manuscript, NAME, brand};

type PrefsOf = Prefs<ViewPrefs>;

struct Window {
    shell: Shell,
    app: Manuscript,
    prefs: Option<PrefsFile<PrefsOf>>,
}

impl Window {
    fn save_prefs(&mut self, force: bool) {
        if let Some(f) = self.prefs.as_mut() {
            let p = Prefs { shell: self.shell.prefs(), app: self.app.view.clone() };
            if let Err(e) = f.save(&p, Instant::now(), force) {
                self.app.status = format!("Preferences could not be saved: {e}");
            }
        }
    }
}

impl eframe::App for Window {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.shell.ui(ui, &mut self.app);
        self.save_prefs(false);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        // Save and make the closing backup (FIL-2).
        if let Err(e) = self.app.close() {
            eprintln!("could not save on close: {e}");
        }
        self.save_prefs(true);
    }
}

fn main() -> ExitCode {
    let mut shell = Shell::new(NAME, env!("CARGO_PKG_VERSION"), brand::tokens())
        .with_options(jcj_shell::ShellOptions { start_window: true, ..jcj_shell::ShellOptions::default() })
        .with_updates(jcj_shell::updates::desktop::ReleaseSource {
            id: "wordsmith-manuscript",
            name: NAME,
            tagline: "Write novels and other long work",
            repo: "jamescjfoundation/wordsmith-manuscript",
            developer: "The James C. J. Foundation",
            version: env!("CARGO_PKG_VERSION"),
            // Updates are installed in place on Linux and Windows; on macOS the update check
            // offers the download.
            installs_in_place: !cfg!(target_os = "macos"),
        });
    let mut app = Manuscript::new();
    let mut prefs = None;
    if let Some(dir) = jcj_shell::prefs::settings_dir("wordsmith-manuscript", NAME) {
        let (file, p, problem) = PrefsFile::<PrefsOf>::open(&dir);
        shell.apply_prefs(&p.shell, &app.panels());
        app.view = p.app.bounded();
        wordsmith_manuscript_ui::languages::set_language(&app.view.language);
        if let Some(m) = problem {
            app.status = m;
        }
        prefs = Some(file);
        // The writing history is kept on this computer, beside the preferences.
        app.history_dir = Some(dir.join("history"));
        app.templates_dir = Some(dir.join("templates"));
    }
    if let Some(path) = std::env::args_os().nth(1)
        && let Err(e) = app.open(std::path::Path::new(&path))
    {
        app.status = e;
    }
    let window = Window { shell, app, prefs };
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
