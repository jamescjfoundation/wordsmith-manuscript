// SPDX-License-Identifier: MIT OR Apache-2.0
//! `manuscript-cli`: Wordsmith Manuscript on the command line (AUT-1).
//!
//! ```text
//! manuscript-cli convert <input> <output>        a document or set into a project, or a project to a file
//! manuscript-cli export <project> <file> [--format F] [--preset NAME]
//! manuscript-cli counts <project> [--item ID]
//! manuscript-cli verify <source> <result>        word-for-word check of two files or projects
//! manuscript-cli report <project>
//! manuscript-cli run <project> <command> [JSON]  any Manuscript command (saved when it changes the project)
//! manuscript-cli commands                        every command, with its parameters
//! manuscript-cli mcp <project>                   the commands as agent tools over standard input and output
//! ```

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

mod cli;

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match cli::run(&args, &mut std::io::stdout()) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("manuscript-cli: {e}");
            ExitCode::FAILURE
        }
    }
}
