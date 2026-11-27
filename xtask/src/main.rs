// SPDX-License-Identifier: MIT OR Apache-2.0
//! `cargo xtask <task>` for Wordsmith Manuscript, built on open-source-core.
//!
//! This file is the app's own, and it holds only what is Manuscript's: its data (`APP`), which
//! includes what a release carries (`package`: the core's `docs/app-releases.md` says what each
//! field is). The gate, the packaging, every shared task and the list of them are in the modules
//! under `shared`, which the core owns and `cargo xtask sync` keeps identical in every app;
//! `cargo xtask help` lists them. Manuscript's layers are the table in `AGENTS.md`, which
//! `cargo xtask app-layers` reads.

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

mod shared;

use std::process::ExitCode;

use shared::gate::{self, App};
use shared::package::{FileType, Package};

/// What the shared tasks need to know about Manuscript.
const APP: App = App {
    product: "Wordsmith Manuscript",
    // The packages that build the programs Manuscript ships, the desktop program's first. Each
    // program has its package's name.
    shipped: &["wordsmith-manuscript", "wordsmith-manuscript-cli"],
    package: Some(Package {
        // Also the `id` of the update check's source and the settings folder's name
        // (`apps/manuscript/src/main.rs`, whose tests hold the two places together).
        id: "wordsmith-manuscript",
        // Also the app id the window is given.
        app_id: "org.jcjfoundation.manuscript",
        summary: "Write novels and other long work",
        categories: "Office;WordProcessor;",
        icons: &[
            (32, "assets/brand/icon-32.png"),
            (256, "assets/brand/icon-256.png"),
            (512, "assets/brand/icon-512.png"),
            (1024, "assets/brand/icon-1024.png"),
        ],
        // `apps/manuscript/build.rs` embeds it in the Windows program.
        windows_icon: Some("assets/brand/icon.ico"),
        // The project types (decision 0002): a project is a folder, a packed project a zip.
        file_types: &[
            FileType {
                name: "Wordsmith Manuscript project",
                extensions: &["wsm"],
                mime: "application/x-wordsmith-manuscript",
                folder: true,
                ..FileType::DEFAULT
            },
            FileType {
                name: "Wordsmith Manuscript packed project",
                extensions: &["wsmz"],
                mime: "application/x-wordsmith-manuscript-packed",
                parent: "application/zip",
                ..FileType::DEFAULT
            },
        ],
        ..Package::DEFAULT
    }),
    // `cargo xtask screenshots` redraws docs/images from a public-domain sample
    // (`crates/manuscript-ui/examples/screenshots.rs`).
    screenshots: Some("wordsmith-manuscript-ui"),
    ..App::DEFAULT
};

fn main() -> ExitCode {
    gate::main(&APP)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_task_table_is_sound() {
        assert_eq!(gate::table_problems(&APP), Vec::<String>::new());
        let in_gate = gate::gate_names(&APP);
        for check in ["app-layers", "assets", "release-data"] {
            assert!(in_gate.contains(&check), "{check} is not in ci");
        }
    }

    /// The release names every reader of a release relies on (the core's `docs/app-releases.md`).
    /// `apps/manuscript/src/main.rs` has the other half: the program gives the same two ids.
    #[test]
    fn the_release_carries_manuscripts_names() {
        assert_eq!(APP.shipped, ["wordsmith-manuscript", "wordsmith-manuscript-cli"]);
        let package = APP.package.as_ref().map(|p| (p.id, p.app_id));
        assert_eq!(package, Some(("wordsmith-manuscript", "org.jcjfoundation.manuscript")));
        assert_eq!(shared::package::data_problems(&APP), Vec::<String>::new());
    }
}
