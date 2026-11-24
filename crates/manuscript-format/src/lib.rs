// SPDX-License-Identifier: MIT OR Apache-2.0
//! The Wordsmith Manuscript project folder: reading and writing `project.toml`, `binder.toml` and
//! the text files in the Manuscript Markdown dialect, schema versions, limits and packing.
//!
//! This crate stays free of interface and engine code: Wordsmith Book depends on it (layer 2).

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]
