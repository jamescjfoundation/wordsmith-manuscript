// SPDX-License-Identifier: MIT OR Apache-2.0
//! Wordsmith Manuscript, the desktop app: the core's shell around Manuscript's views.
//!
//! `wordsmith-manuscript [project]` opens the project (a `.wsm` folder, its `project.toml` or a
//! packed `.wsmz`). `wordsmith-manuscript --version` prints the program's name and version and
//! opens no window.
//!
//! The start-up is the core's (`jcj_shell::desktop`, core decision 0014): this file gives
//! Manuscript's data ([`DESKTOP`]), and `wordsmith_manuscript_ui::desktop` what only Manuscript
//! does. The window's title, app id and icon, the settings folder, the preferences, the update
//! check, the close that starts an installed update and its helper, the file dialogs and the
//! steps of every frame and of the end are not written here, so none of them can be left out.

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

use std::ffi::OsString;
use std::process::ExitCode;

use jcj_shell::desktop::{self, Desktop, Releases};
use wordsmith_manuscript_ui::{NAME, brand};

/// The release id: the stem of every release file, the desktop program's name, and the name of
/// the settings folder on Linux. `xtask/src/main.rs` gives the packaging the same id.
const ID: &str = wordsmith_manuscript_ui::ID;

/// The id the window is given. A Linux desktop joins the window to its menu entry and icon by
/// it, and the macOS app carries it; `xtask/src/main.rs` gives the packaging the same id.
const APP_ID: &str = "org.jcjfoundation.manuscript";

/// Manuscript's data for the core's start-up: its names, its brand, and where its releases come
/// from, for the update check. Updates are installed in place on Linux and Windows; on macOS the
/// update check offers the download.
const DESKTOP: Desktop = Desktop {
    brand: brand::BRAND,
    releases: Some(Releases {
        repo: "jamescjfoundation/wordsmith-manuscript",
        tagline: "Write novels and other long work",
        developer: "The James C. J. Foundation",
    }),
    ..Desktop::new(NAME, ID, APP_ID, env!("CARGO_PKG_VERSION"))
};

/// Whether the command line asks for the version line (`--version` or `-V` first) and no window.
fn asks_version(args: &[OsString]) -> bool {
    args.first().is_some_and(|first| first == "--version" || first == "-V")
}

/// The line `--version` prints: the program's name and its version.
fn version_line() -> String {
    format!("{ID} {}", env!("CARGO_PKG_VERSION"))
}

fn main() -> ExitCode {
    // First of all: on Windows an update is finished by a copy of this program, started with
    // the updater's own arguments. It opens no window.
    if let Some(code) = DESKTOP.finish_update() {
        return code;
    }
    if asks_version(&std::env::args_os().skip(1).collect::<Vec<_>>()) {
        println!("{}", version_line());
        return ExitCode::SUCCESS;
    }
    // The project named on the command line, kept as the system gave it, so a name that is not
    // Unicode still opens.
    match DESKTOP.run(desktop::arguments(), wordsmith_manuscript_ui::desktop::build) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{ID}: could not open a window: {e}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use jcj_shell::desktop::DesktopApp;
    use jcj_shell::egui;

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
        let releases = DESKTOP.release_source().unwrap();
        assert_eq!(releases.id, ID);
        assert_eq!(releases.name, NAME);
        assert_eq!(releases.tagline, "Write novels and other long work");
        assert_eq!(releases.repo, "jamescjfoundation/wordsmith-manuscript");
        assert_eq!(releases.releases_url(), "https://github.com/jamescjfoundation/wordsmith-manuscript/releases");
        assert_eq!(releases.version, env!("CARGO_PKG_VERSION"));
        assert_eq!(releases.developer, "The James C. J. Foundation");
    }

    #[test]
    fn macos_is_sent_to_the_release_page() {
        assert_eq!(DESKTOP.release_source().unwrap().installs_in_place, !cfg!(target_os = "macos"));
    }

    #[test]
    fn an_ordinary_start_is_not_an_updates_helper() {
        assert!(DESKTOP.finish_update_with(&[]).is_none());
        assert!(DESKTOP.finish_update_with(&[OsString::from("novel.wsm")]).is_none());
        assert!(DESKTOP.finish_update_with(&[OsString::from("--version")]).is_none());
        // The updater's own first argument is never read as a project to open.
        assert!(DESKTOP.finish_update_with(&[OsString::from(jcj_shell::updates::desktop::FINISH_FLAG)]).is_some());
    }

    #[test]
    fn the_command_line_is_a_project_or_the_version() {
        assert!(!asks_version(&[]));
        assert!(!asks_version(&[OsString::from("novel.wsm")]));
        assert!(asks_version(&[OsString::from("--version")]));
        assert!(asks_version(&[OsString::from("-V")]));
        assert_eq!(desktop::files_named([OsString::from("novel.wsm")]), [PathBuf::from("novel.wsm")]);
        assert_eq!(version_line(), concat!("wordsmith-manuscript ", env!("CARGO_PKG_VERSION")));
    }

    /// The window is the app's name, with its app id and icon, as before the start-up was the
    /// core's.
    #[test]
    fn the_window_carries_the_app_s_name_id_and_icon() {
        let viewport = DESKTOP.viewport();
        assert_eq!(viewport.title.as_deref(), Some(NAME));
        assert_eq!(viewport.app_id.as_deref(), Some(APP_ID));
        assert!(viewport.icon.is_some(), "the brand's icon decodes");
    }

    /// The start-up without a window: Manuscript is built with its folders beside the
    /// preferences, opens the project it was started with, and keeps its preferences there.
    #[test]
    fn manuscript_starts_from_its_data() {
        let dir = std::env::temp_dir().join(format!("wordsmith-manuscript-start-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let settings = dir.join("settings");
        let ctx = egui::Context::default();
        let missing = dir.join("Nowhere.wsm");
        let mut window =
            jcj_shell::desktop::Window::start(&DESKTOP, &ctx, Some(settings.clone()), vec![missing], wordsmith_manuscript_ui::desktop::build);
        assert_eq!(window.app.history_dir.as_deref(), Some(settings.join("history").as_path()));
        assert_eq!(window.app.templates_dir.as_deref(), Some(settings.join("templates").as_path()));
        assert!(!window.app.status.is_empty(), "a project that cannot be opened is said on the status line");
        assert!(window.app.session.is_none());
        assert_eq!(window.app.window_title(), None, "the window keeps the app's name");
        assert_eq!(window.prefs_path(), Some(settings.join("preferences.json").as_path()));
        // A preference changed just before the end is written at the end.
        window.app.view.typewriter = !window.app.view.typewriter;
        window.exit();
        let written = std::fs::read_to_string(settings.join("preferences.json")).unwrap();
        assert!(written.contains("typewriter"), "{written}");
        drop(window);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
