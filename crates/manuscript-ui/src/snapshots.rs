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
use wordsmith_manuscript_model::diff::{self, Change, Diff};

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
            id: "snapshot.compare",
            label: "Compare Snapshot",
            key: "",
            params: r#"{"id": id, "taken": string, "with"?: string}"#,
            about: "Compare a snapshot word by word with the current text (or with another snapshot): each paragraph as same, added and removed stretches, and the words added and removed.",
            mutates: false,
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
        "snapshot.compare" => (|| {
            let item = id_param(v, "id")?;
            let d = compare(s, item, str_param(v, "taken")?, opt_str(v, "with"))?;
            let key = |c: Change| match c {
                Change::Same => "same",
                Change::Added => "added",
                Change::Removed => "removed",
            };
            Ok(json!({
                "added": d.added,
                "removed": d.removed,
                "same": d.is_same(),
                "paragraphs": d.paras.iter().map(|p| p.iter().map(|s| json!({ "change": key(s.change), "text": s.text })).collect::<Vec<_>>()).collect::<Vec<_>>(),
            }))
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

/// Compare a snapshot (older) with the current text, or with another snapshot.
pub fn compare(s: &Session, item: ItemId, taken: &str, with: Option<&str>) -> Result<Diff, String> {
    let list = s.project.snapshots.get(&item).cloned().unwrap_or_default();
    let find = |t: &str| list.iter().find(|x| x.taken == t).map(|x| x.text.clone()).ok_or_else(|| format!("no snapshot {t}"));
    let old = find(taken)?;
    let new = match with {
        Some(w) => find(w)?,
        None => s.project.text(item).map(|t| t.body.clone()).unwrap_or_default(),
    };
    Ok(diff::texts(&old, &new))
}

/// A comparison shown in a window: which item and snapshot.
#[derive(Clone, Debug, Default)]
pub struct CompareView {
    /// The item and snapshot compared with the current text, while the window is open.
    pub open: Option<(ItemId, String)>,
}

/// Draw the comparison window, if one is open.
pub fn compare_window(ctx: &egui::Context, s: &Session, view: &mut CompareView) {
    let Some((item, taken)) = view.open.clone() else { return };
    let Ok(d) = compare(s, item, &taken, None) else {
        view.open = None;
        return;
    };
    let name = s.project.snapshots.get(&item).and_then(|l| l.iter().find(|x| x.taken == taken)).map(|x| x.name.clone()).unwrap_or_default();
    let title = s.project.binder.get(item).map(|i| i.title.clone()).unwrap_or_default();
    let t = jcj_theme::Theme::get(ctx);
    let mut open = true;
    egui::Window::new(format!("{title}: “{name}” and now"))
        .id(egui::Id::unique("manuscript-compare"))
        .open(&mut open)
        .default_size([560.0, 480.0])
        .show(ctx, |ui| {
            ui.label(if d.is_same() { "No changes.".to_owned() } else { format!("{} words added, {} removed.", d.added, d.removed) });
            ui.separator();
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                for p in &d.paras {
                    let mut job = egui::text::LayoutJob::default();
                    job.wrap.max_width = ui.available_width();
                    for seg in p {
                        let mut f = egui::TextFormat { font_id: jcj_theme::regular(t.type_sizes.body), color: t.text, ..Default::default() };
                        match seg.change {
                            Change::Same => {}
                            Change::Added => {
                                f.underline = egui::Stroke::new(1.5, t.accent);
                                f.background = t.accent_soft;
                            }
                            Change::Removed => {
                                f.strikethrough = egui::Stroke::new(1.5, t.danger);
                                f.color = t.danger;
                            }
                        }
                        job.append(&seg.text, 0.0, f);
                    }
                    let plain: String = p.iter().map(|s| s.text.as_str()).collect();
                    let r = ui.label(job);
                    r.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Label, true, &plain));
                    ui.add_space(4.0);
                }
            });
        });
    if !open {
        view.open = None;
    }
}

/// The inspector's snapshots section.
pub fn panel_section(ui: &mut Ui, s: &mut Session, id: ItemId, errors: &mut Vec<String>) {
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.label(crate::i18n::tr("Snapshots"));
        if ui.small_button(crate::i18n::tr("Take Snapshot")).clicked()
            && let Err(e) = s.run("snapshot.take", &json!({ "id": id.as_text() }))
        {
            errors.push(e);
        }
    });
    let list = s.project.snapshots.get(&id).cloned().unwrap_or_default();
    let mut todo: Option<(&str, String)> = None;
    let mut compare_with: Option<String> = None;
    for snap in list.iter().rev().take(50) {
        ui.horizontal(|ui| {
            ui.label(format!("{} · {}", snap.name, wordsmith_manuscript_model::snapshot::readable(&snap.taken)));
            if ui.small_button("Compare").on_hover_text("Compare with the current text, word by word").clicked() {
                compare_with = Some(snap.taken.clone());
            }
            if ui.small_button("Restore").clicked() {
                todo = Some(("snapshot.restore", snap.taken.clone()));
            }
            if ui.small_button("Delete").clicked() {
                todo = Some(("snapshot.delete", snap.taken.clone()));
            }
        });
    }
    if let Some(taken) = compare_with {
        ui.ctx().data_mut(|d| d.insert_temp(egui::Id::unique("manuscript-compare-ask"), (id, taken)));
    }
    if let Some((cmd, taken)) = todo
        && let Err(e) = s.run(cmd, &json!({ "id": id.as_text(), "taken": taken }))
    {
        errors.push(e);
    }
    let _ = egui::Id::NULL;
}
