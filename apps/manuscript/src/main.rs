// SPDX-License-Identifier: MIT OR Apache-2.0
//! Wordsmith Manuscript, the desktop app: the core's shell around Manuscript's views.
//!
//! `wordsmith-manuscript [project]` opens the project (a `.wsm` folder, its `project.toml` or a
//! packed `.wsmz`). `wordsmith-manuscript --version` prints the program's name and version and
//! opens no window.
//!
//! The update check installs a newer release in place where it can (the core's
//! `docs/app-releases.md`, "Updating in place"). Three steps here make that work, and each is
//! easy to drop: `main` hands its arguments to the updater first, because on Windows the
//! updater finishes through a copy of this program; the window closes when the shell asks for
//! it; and the shell is told last of all that the program ends, which starts the new version.

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

use std::process::ExitCode;
use std::time::Instant;

use jcj_shell::egui;
use jcj_shell::prefs::{Prefs, PrefsFile};
use jcj_shell::updates::desktop::ReleaseSource;
use jcj_shell::{App, Shell};
use wordsmith_manuscript_ui::view::ViewPrefs;
use wordsmith_manuscript_ui::{Manuscript, NAME, brand};

type PrefsOf = Prefs<ViewPrefs>;

/// The release id: the stem of every release file, the desktop program's name, and the name of
/// the settings folder on Linux. `xtask/src/main.rs` gives the packaging the same id.
const ID: &str = "wordsmith-manuscript";

/// The id the window is given. A Linux desktop joins the window to its menu entry and icon by
/// it, and the macOS app carries it; `xtask/src/main.rs` gives the packaging the same id.
const APP_ID: &str = "org.jcjfoundation.manuscript";

/// Where Manuscript's releases come from, for the update check.
const RELEASES: ReleaseSource = ReleaseSource {
    id: ID,
    name: NAME,
    tagline: "Write novels and other long work",
    repo: "jamescjfoundation/wordsmith-manuscript",
    developer: "The James C. J. Foundation",
    version: env!("CARGO_PKG_VERSION"),
    // Updates are installed in place on Linux and Windows; on macOS the update check offers
    // the download.
    installs_in_place: !cfg!(target_os = "macos"),
};

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
        // An installed update waits for the window to close: the project is saved on the way
        // out (`on_exit`), so nothing has to be asked first.
        if self.shell.close_requested() {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        // Save and make the closing backup (FIL-2).
        if let Err(e) = self.app.close() {
            eprintln!("{ID}: could not save on close: {e}");
        }
        self.save_prefs(true);
        // Last, once everything is saved: the helper of an installed update starts the new
        // version after this program has ended.
        self.shell.finish();
    }
}

/// What the command line asks for besides a window.
#[derive(Debug, PartialEq, Eq)]
enum Asked {
    /// `--version` or `-V`: print the version line and open no window.
    Version,
    /// Open the window, with this project if one was named.
    Window(Option<std::ffi::OsString>),
}

/// Read the arguments (without the program's name). The project's name is kept as the system
/// gave it, so a name that is not Unicode still opens.
fn asked(args: &[std::ffi::OsString]) -> Asked {
    match args.first() {
        Some(first) if first == "--version" || first == "-V" => Asked::Version,
        first => Asked::Window(first.cloned()),
    }
}

/// The line `--version` prints: the program's name and its version.
fn version_line() -> String {
    format!("{ID} {}", env!("CARGO_PKG_VERSION"))
}

fn main() -> ExitCode {
    let args: Vec<std::ffi::OsString> = std::env::args_os().skip(1).collect();
    // First of all: on Windows an update is finished by a copy of this program, started with
    // the updater's own arguments. It opens no window.
    if let Some(done) = RELEASES.finish_update(&args) {
        return match done {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("{ID}: the update could not be finished: {e}");
                ExitCode::FAILURE
            }
        };
    }
    let open = match asked(&args) {
        Asked::Version => {
            println!("{}", version_line());
            return ExitCode::SUCCESS;
        }
        Asked::Window(open) => open,
    };
    let mut shell = Shell::new(NAME, env!("CARGO_PKG_VERSION"), brand::tokens())
        .with_options(jcj_shell::ShellOptions { start_window: true, ..jcj_shell::ShellOptions::default() })
        .with_updates(RELEASES);
    let mut app = Manuscript::new();
    let mut prefs = None;
    if let Some(dir) = jcj_shell::prefs::settings_dir(ID, NAME) {
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
    if let Some(path) = open
        && let Err(e) = app.open(std::path::Path::new(&path))
    {
        app.status = e;
    }
    let window = Window { shell, app, prefs };
    let mut viewport = egui::ViewportBuilder::default().with_title(NAME).with_app_id(APP_ID);
    if let Ok(icon) = eframe::icon_data::from_png_bytes(brand::ICON_256) {
        viewport = viewport.with_icon(icon);
    }
    let options = eframe::NativeOptions { viewport, ..eframe::NativeOptions::default() };
    match eframe::run_native(NAME, options, Box::new(|_cc| Ok(Box::new(window)))) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{ID}: could not open a window: {e}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;

    use super::*;

    /// The names a release is found and installed by (the core's `docs/app-releases.md`). The
    /// packaging has the same two ids in `xtask/src/main.rs`, whose own test holds them there:
    /// the xtask cannot read this file, so each side says them and both are tested.
    #[test]
    fn the_program_carries_the_names_a_release_is_found_by() {
        assert_eq!(ID, "wordsmith-manuscript");
        assert_eq!(APP_ID, "org.jcjfoundation.manuscript");
        // The desktop program is named by the release id, and so is its package.
        assert_eq!(env!("CARGO_BIN_NAME"), ID);
        assert_eq!(env!("CARGO_PKG_NAME"), ID);
        assert_eq!(RELEASES.id, ID);
        assert_eq!(RELEASES.repo, "jamescjfoundation/wordsmith-manuscript");
        assert_eq!(RELEASES.releases_url(), "https://github.com/jamescjfoundation/wordsmith-manuscript/releases");
        assert_eq!(RELEASES.version, env!("CARGO_PKG_VERSION"));
        assert_eq!(RELEASES.developer, "The James C. J. Foundation");
    }

    #[test]
    fn macos_is_sent_to_the_release_page() {
        assert_eq!(RELEASES.installs_in_place, !cfg!(target_os = "macos"));
    }

    #[test]
    fn an_ordinary_start_is_not_an_updates_helper() {
        assert!(RELEASES.finish_update(&[]).is_none());
        assert!(RELEASES.finish_update(&[OsString::from("novel.wsm")]).is_none());
        assert!(RELEASES.finish_update(&[OsString::from("--version")]).is_none());
        // The updater's own first argument is never read as a project to open.
        assert!(RELEASES.finish_update(&[OsString::from(jcj_shell::updates::desktop::FINISH_FLAG)]).is_some());
    }

    #[test]
    fn the_command_line_is_a_project_or_the_version() {
        assert_eq!(asked(&[]), Asked::Window(None));
        assert_eq!(asked(&[OsString::from("novel.wsm")]), Asked::Window(Some(OsString::from("novel.wsm"))));
        assert_eq!(asked(&[OsString::from("--version")]), Asked::Version);
        assert_eq!(asked(&[OsString::from("-V")]), Asked::Version);
        assert_eq!(version_line(), concat!("wordsmith-manuscript ", env!("CARGO_PKG_VERSION")));
    }
}
