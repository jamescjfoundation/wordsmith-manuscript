// SPDX-License-Identifier: MIT OR Apache-2.0
//! Import and export for Wordsmith Manuscript: mapping documents to the binder, document-set
//! rules, the word-for-word verifier, export presets and the standard manuscript layout (layer 3).

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

pub mod compile;
pub mod docset;
pub mod export;
pub mod external;
pub mod import;
pub mod mapping;
pub mod notes;
pub mod opml;
pub mod screenplay;
pub mod standard;

/// The word-for-word check (plan section 8, own rule 2): the core's, under the name this crate
/// has always given it. Every import and export here proves its words with it.
pub use jcj_features::verify;
