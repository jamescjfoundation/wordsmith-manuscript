// SPDX-License-Identifier: MIT OR Apache-2.0
//! `cargo xtask <task>` for Wordsmith Manuscript, built on open-source-core.
//!
//! This file is the app's own, and it holds only what is Manuscript's: its data (`APP`). The
//! gate, every shared task and the list of them are in the modules under `shared`, which the
//! core owns and `cargo xtask sync` keeps identical in every app; `cargo xtask help` lists them.
//! Manuscript's layers are the table in `AGENTS.md`, which `cargo xtask app-layers` reads.

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

mod shared;

use std::process::ExitCode;

use shared::gate::{self, App};

/// What the shared tasks need to know about Manuscript.
const APP: App = App {
    product: "Wordsmith Manuscript",
    // The programs Manuscript ships (for THIRD-PARTY-LICENSES.txt).
    shipped: &["wordsmith-manuscript", "manuscript-cli"],
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
        assert!(gate::gate_names(&APP).contains(&"app-layers"));
    }
}
