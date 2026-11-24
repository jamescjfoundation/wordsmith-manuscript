// SPDX-License-Identifier: MIT OR Apache-2.0
//! Item metadata commands and the inspector panel (M3.1).

use serde_json::Value;

use crate::commands::Spec;
use crate::session::{Outcome, Session};

/// The inspector's commands.
pub fn specs() -> Vec<Spec> {
    Vec::new()
}

/// Run an inspector command, or `None` if `id` is not one.
pub fn run(_s: &mut Session, _id: &str, _v: &Value) -> Option<Outcome> {
    None
}
