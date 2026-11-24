// SPDX-License-Identifier: MIT OR Apache-2.0
//! The Wordsmith Manuscript project folder: reading and writing `project.toml`, `binder.toml` and
//! the text files in the Manuscript Markdown dialect, schema versions, limits and packing.
//!
//! This crate stays free of interface and engine code: Wordsmith Book depends on it (layer 2).

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

pub mod dialect;
pub mod files;
pub mod folder;
pub mod limits;
pub mod offers;
pub mod pack;

/// The schema version this build writes and the newest it reads.
pub const SCHEMA: u32 = 3;

/// Why a project could not be read or written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The folder has no `project.toml`, or it does not say it is a project.
    NotAProject,
    /// The project was written by a newer version of the app.
    Newer {
        /// The schema the project declares.
        found: u32,
        /// The newest schema this build reads.
        supported: u32,
    },
    /// A file is over a limit.
    TooLarge {
        /// What was too large.
        what: String,
        /// The limit.
        limit: u64,
    },
    /// A file is damaged or not in the format.
    Damaged {
        /// The file, inside the project.
        file: String,
        /// What is wrong.
        why: String,
    },
    /// Reading or writing a file failed.
    Io {
        /// The file.
        file: String,
        /// The system's message.
        why: String,
    },
    /// Something could not be written (an internal limit of the writer).
    Write(String),
}

impl Error {
    /// The same error, naming the file it happened in when it names none.
    pub fn in_file(self, file: &str) -> Error {
        match self {
            Error::Damaged { file: f, why } if f.is_empty() => Error::Damaged { file: file.into(), why },
            e => e,
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::NotAProject => write!(f, "this is not a Wordsmith Manuscript project (no project.toml with a schema)"),
            Error::Newer { found, supported } => write!(
                f,
                "this project was saved by a newer version of Wordsmith Manuscript (format {found}; this version reads up to {supported}). Update the app to open it."
            ),
            Error::TooLarge { what, limit } => write!(f, "{what} is larger than the limit of {limit}"),
            Error::Damaged { file, why } => write!(f, "{file} is damaged: {why}"),
            Error::Io { file, why } => write!(f, "{file}: {why}"),
            Error::Write(why) => write!(f, "could not write the project: {why}"),
        }
    }
}

impl std::error::Error for Error {}
pub mod book;
