// SPDX-License-Identifier: MIT OR Apache-2.0
//! Wordsmith Manuscript's project model: the binder tree and its items, roles, metadata, project
//! settings, targets, the story bible and the link index. No files and no interface (layer 1).

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

mod binder;
pub mod doc;
pub mod entries;
mod id;
mod meta;
mod project;
mod settings;
pub mod snapshot;
pub mod templates;
pub mod text;
pub mod words;

pub use binder::{Area, Binder, BinderError, Item, Kind, MAX_DEPTH, Place, Role, Row};
pub use entries::Entry;
pub use id::{BadId, ItemId};
pub use meta::{Comment, ItemMeta, ItemText};
pub use project::Project;
pub use settings::{BookInfo, DAYS, FieldDef, FieldKind, Named, POV_FIELD, Settings, Targets};
pub use snapshot::Snapshot;
pub use text::{Block, Fence, ImageRole, Inline, Inlines, Marks, Note, Text};
