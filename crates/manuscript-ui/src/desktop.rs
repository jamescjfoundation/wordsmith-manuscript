// SPDX-License-Identifier: MIT OR Apache-2.0
//! What only Manuscript does at start-up, while it runs and at the end. The start-up itself is
//! the core's (`jcj_shell::desktop`, core decision 0014): the window, the settings folder, the
//! preferences file, the update check and its restart, the file dialogs. `apps/manuscript` gives
//! it Manuscript's data and builds the app; the steps here are the ones it calls back.

use std::path::PathBuf;

use jcj_shell::Shell;
use jcj_shell::desktop::{DesktopApp, Start};
use jcj_shell::prefs::Prefs;

use crate::view::ViewPrefs;
use crate::{ID, Manuscript, languages};

/// Build the app once its window exists: the start window on, and the writing history and the
/// writer's own templates kept on this computer, beside the preferences.
pub fn build(start: &mut Start<'_>) -> Manuscript {
    start.shell_with(|s| s.with_options(jcj_shell::ShellOptions { start_window: true, ..jcj_shell::ShellOptions::default() }));
    let mut app = Manuscript::new();
    if let Some(dir) = start.settings {
        app.history_dir = Some(dir.join("history"));
        app.templates_dir = Some(dir.join("templates"));
    }
    app
}

impl DesktopApp for Manuscript {
    type Prefs = Prefs<ViewPrefs>;

    /// The view's preferences, held to their bounds, then the interface language they name.
    fn apply_prefs(&mut self, prefs: &ViewPrefs, _shell: &mut Shell) {
        self.view = prefs.clone().bounded();
        languages::set_language(&self.view.language);
    }

    fn prefs(&self, _last: &ViewPrefs) -> ViewPrefs {
        self.view.clone()
    }

    /// The project named on the command line, if one was; a problem goes to the status line.
    fn open(&mut self, files: &[PathBuf], _shell: &mut Shell) {
        if let Some(path) = files.first()
            && let Err(e) = Manuscript::open(self, path)
        {
            self.status = e;
        }
    }

    fn report(&mut self, problem: String, _shell: &mut Shell) {
        self.status = problem;
    }

    /// Save and make the closing backup (FIL-2).
    fn exit(&mut self, _shell: &mut Shell) {
        if let Err(e) = self.close() {
            eprintln!("{ID}: could not save on close: {e}");
        }
    }
}
