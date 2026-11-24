// SPDX-License-Identifier: MIT OR Apache-2.0
//! Limits on what a project may hold. Reading refuses anything beyond them with an error, so a
//! damaged or hostile project cannot exhaust memory. Tests feed files past each one.

/// The largest `project.toml`.
pub const MAX_PROJECT_FILE: u64 = 4 * 1024 * 1024;
/// The largest `binder.toml`.
pub const MAX_BINDER_FILE: u64 = 32 * 1024 * 1024;
/// The largest text or entry file.
pub const MAX_TEXT_FILE: u64 = 64 * 1024 * 1024;
/// The largest front matter of one file.
pub const MAX_FRONT_MATTER: usize = 1024 * 1024;
/// The most items a binder may hold.
pub const MAX_ITEMS: usize = 100_000;
/// The most bytes of text and entry files read for one project.
pub const MAX_PROJECT_TEXT: u64 = 1024 * 1024 * 1024;
