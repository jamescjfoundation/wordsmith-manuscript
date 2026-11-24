// SPDX-License-Identifier: MIT OR Apache-2.0
//! Snapshots (INS-5): taking them, listing them and restoring one.

use std::time::SystemTime;

use wordsmith_manuscript_model::ItemId;

use crate::session::Session;

/// Take a snapshot of an item's text now (not an undo step: a snapshot is kept whatever happens
/// next). Returns its stamp.
pub fn take(s: &mut Session, id: ItemId, name: &str) -> Option<String> {
    let taken = s.project.take_snapshot(id, name, SystemTime::now());
    s.touch();
    taken
}
