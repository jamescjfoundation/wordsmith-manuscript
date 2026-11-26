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
//! manuscript-cli mcp <project> [--read-root <folder>] [--write-root <folder>]
//!                                                the commands as agent tools over standard input and output
//! manuscript-cli --help | --version
//! ```
//!
//! It follows the core's convention for command lines
//! (`docs/command-lines-and-agent-tools.md` in open-source-core): results on standard output,
//! what went wrong on standard error, and the exit codes 0 done, 1 failed, 2 a wrong command
//! line, 3 a word check found a difference.

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

mod cli;

use std::process::ExitCode;

use jcj_agents::cli::Exit;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match cli::run(&args, &mut std::io::stdout()) {
        Ok(exit) => exit.into(),
        Err(stop) => {
            eprintln!("{}: {}", cli::PROGRAM, stop.why);
            if stop.exit == Exit::Usage {
                eprintln!("{}", cli::USAGE);
            }
            stop.exit.into()
        }
    }
}
