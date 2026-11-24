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

use jcj_shell::egui::{self, Ui};
use serde_json::{Value, json};

use crate::commands::{Spec, id_param, opt_str, str_param};
use crate::session::Outcome;

/// The snapshot commands.
pub fn specs() -> Vec<Spec> {
    vec![
        Spec {
            id: "snapshot.take",
            label: "Take Snapshot",
            key: "Mod+Alt+S",
            params: r#"{"id": id, "name"?: string}"#,
            about: "Keep a named, dated copy of an item's text.",
            mutates: true,
        },
        Spec {
            id: "snapshot.list",
            label: "List Snapshots",
            key: "",
            params: r#"{"id": id}"#,
            about: "An item's snapshots, oldest first.",
            mutates: false,
        },
        Spec {
            id: "snapshot.restore",
            label: "Restore Snapshot",
            key: "",
            params: r#"{"id": id, "taken": string}"#,
            about: "Put a snapshot's text back (the current text is snapshotted first; one undo step).",
            mutates: true,
        },
        Spec {
            id: "snapshot.rename",
            label: "Rename Snapshot",
            key: "",
            params: r#"{"id": id, "taken": string, "name": string}"#,
            about: "Give a snapshot another name.",
            mutates: true,
        },
        Spec {
            id: "snapshot.delete",
            label: "Delete Snapshot",
            key: "",
            params: r#"{"id": id, "taken": string}"#,
            about: "Remove a snapshot for good.",
            mutates: true,
        },
        Spec {
            id: "snapshot.get",
            label: "Get Snapshot",
            key: "",
            params: r#"{"id": id, "taken": string}"#,
            about: "A snapshot's text in the Markdown dialect.",
            mutates: false,
        },
    ]
}

/// Run a snapshot command, or `None` if `id` is not one.
pub fn run(s: &mut Session, id: &str, v: &Value) -> Option<Outcome> {
    Some(match id {
        "snapshot.take" => (|| {
            let item = id_param(v, "id")?;
            if s.project.binder.get(item).is_none_or(|i| !i.kind.has_text()) {
                return Err("this item has no text to keep".into());
            }
            let name = opt_str(v, "name").map(str::trim).filter(|n| !n.is_empty()).unwrap_or("Snapshot").to_owned();
            let taken = take(s, item, &name).ok_or("the snapshot could not be taken")?;
            Ok(json!({ "taken": taken }))
        })(),
        "snapshot.list" => (|| {
            let item = id_param(v, "id")?;
            let list = s.project.snapshots.get(&item).cloned().unwrap_or_default();
            Ok(Value::Array(
                list.iter()
                    .map(|x| json!({ "taken": x.taken, "when": wordsmith_manuscript_model::snapshot::readable(&x.taken), "name": x.name, "words": x.text.word_count() }))
                    .collect(),
            ))
        })(),
        "snapshot.get" => (|| {
            let item = id_param(v, "id")?;
            let taken = str_param(v, "taken")?;
            let snap = s.project.snapshots.get(&item).and_then(|l| l.iter().find(|x| x.taken == taken)).ok_or("no such snapshot")?;
            Ok(json!({ "name": snap.name, "text": wordsmith_manuscript_format::dialect::write(&snap.text) }))
        })(),
        "snapshot.restore" => (|| {
            let item = id_param(v, "id")?;
            let taken = str_param(v, "taken")?.to_owned();
            let snap = s.project.snapshots.get(&item).and_then(|l| l.iter().find(|x| x.taken == taken)).cloned().ok_or("no such snapshot")?;
            take(s, item, &format!("Before restoring “{}”", snap.name));
            s.change("Restore Snapshot", &[item], &[], |p| {
                p.text_mut(item).body = snap.text;
                Ok(())
            })?;
            Ok(json!({}))
        })(),
        "snapshot.rename" => (|| {
            let item = id_param(v, "id")?;
            let taken = str_param(v, "taken")?.to_owned();
            let name = str_param(v, "name")?.trim().to_owned();
            if name.is_empty() {
                return Err("a snapshot needs a name".into());
            }
            let snap = s.project.snapshots.get_mut(&item).and_then(|l| l.iter_mut().find(|x| x.taken == taken)).ok_or("no such snapshot")?;
            snap.name = name;
            s.touch();
            Ok(json!({}))
        })(),
        "snapshot.delete" => (|| {
            let item = id_param(v, "id")?;
            let taken = str_param(v, "taken")?.to_owned();
            let list = s.project.snapshots.get_mut(&item).ok_or("no such snapshot")?;
            let before = list.len();
            list.retain(|x| x.taken != taken);
            if list.len() == before {
                return Err("no such snapshot".into());
            }
            s.touch();
            Ok(json!({}))
        })(),
        _ => return None,
    })
}

/// The inspector's snapshots section.
pub fn panel_section(ui: &mut Ui, s: &mut Session, id: ItemId, errors: &mut Vec<String>) {
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.label("Snapshots");
        if ui.small_button("Take Snapshot").clicked()
            && let Err(e) = s.run("snapshot.take", &json!({ "id": id.as_text() }))
        {
            errors.push(e);
        }
    });
    let list = s.project.snapshots.get(&id).cloned().unwrap_or_default();
    let mut todo: Option<(&str, String)> = None;
    for snap in list.iter().rev().take(50) {
        ui.horizontal(|ui| {
            ui.label(format!("{} · {}", snap.name, wordsmith_manuscript_model::snapshot::readable(&snap.taken)));
            if ui.small_button("Restore").clicked() {
                todo = Some(("snapshot.restore", snap.taken.clone()));
            }
            if ui.small_button("Delete").clicked() {
                todo = Some(("snapshot.delete", snap.taken.clone()));
            }
        });
    }
    if let Some((cmd, taken)) = todo
        && let Err(e) = s.run(cmd, &json!({ "id": id.as_text(), "taken": taken }))
    {
        errors.push(e);
    }
    let _ = egui::Id::NULL;
}
