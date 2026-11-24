// SPDX-License-Identifier: MIT OR Apache-2.0
//! Reading and replacing an item's text as dialect text (for agents, the command line and tests).

use serde_json::{Value, json};
use wordsmith_manuscript_format::dialect;

use crate::commands::{Spec, id_param, str_param};
use crate::session::{Outcome, Session};

/// The text commands.
pub fn specs() -> Vec<Spec> {
    vec![
        Spec {
            id: "text.get",
            label: "Get Text",
            key: "",
            params: r#"{"id": id}"#,
            about: "An item's text in the project's Markdown dialect, with its word count.",
            mutates: false,
        },
        Spec {
            id: "text.set",
            label: "Set Text",
            key: "",
            params: r#"{"id": id, "text": string}"#,
            about: "Replace an item's text with dialect text (one undo step).",
            mutates: true,
        },
    ]
}

/// Run a text command, or `None` if `id` is not one.
pub fn run(s: &mut Session, id: &str, v: &Value) -> Option<Outcome> {
    Some(match id {
        "text.get" => (|| {
            let item = id_param(v, "id")?;
            s.project.binder.get(item).ok_or_else(|| format!("no binder item {item}"))?;
            let body = s.project.text(item).map(|t| dialect::write(&t.body)).unwrap_or_default();
            Ok(json!({ "text": body, "words": s.project.item_words(item) }))
        })(),
        "text.set" => (|| {
            let item = id_param(v, "id")?;
            let text = str_param(v, "text")?;
            let kind = s.project.binder.get(item).ok_or_else(|| format!("no binder item {item}"))?.kind;
            if !kind.has_text() {
                return Err("this item has no text".into());
            }
            let body = dialect::parse(text);
            s.change("Set Text", &[item], &[], |p| {
                p.text_mut(item).body = body;
                Ok(())
            })?;
            Ok(json!({ "words": s.project.item_words(item) }))
        })(),
        _ => return None,
    })
}
