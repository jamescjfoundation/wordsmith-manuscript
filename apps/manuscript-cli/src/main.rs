// SPDX-License-Identifier: MIT OR Apache-2.0
//! `manuscript-cli`: Wordsmith Manuscript on the command line.

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

fn main() -> std::process::ExitCode {
    println!("manuscript-cli {}", env!("CARGO_PKG_VERSION"));
    std::process::ExitCode::SUCCESS
}
