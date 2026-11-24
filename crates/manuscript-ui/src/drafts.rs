// SPDX-License-Identifier: MIT OR Apache-2.0
//! Drafts (DRF-1): start a new draft from the working one, switch drafts, compare two drafts
//! scene by scene, and choose which draft exports. See `manuscript-model::drafts`.

use std::time::SystemTime;

use jcj_shell::egui::{self, RichText};
use serde_json::{Value, json};
use wordsmith_manuscript_model::diff::Change;
use wordsmith_manuscript_model::drafts::{self, ItemChange};
use wordsmith_manuscript_model::{ItemId, Project};

use crate::commands::{Spec, opt_str, str_param};
use crate::session::{Outcome, Session};

/// The draft commands.
pub fn specs() -> Vec<Spec> {
    let spec = |id, label, params, about, mutates| Spec { id, label, key: "", params, about, mutates };
    vec![
        spec(
            "draft.new",
            "New Draft",
            r#"{"name": string}"#,
            "Keep the working draft as it is now and carry on writing it as a new draft with this name.",
            true,
        ),
        spec("draft.list", "List Drafts", "{}", "The working draft and the stored ones: id, name, when kept, words, and which exports.", false),
        spec(
            "draft.switch",
            "Switch to Draft",
            r#"{"id": string}"#,
            "Make a stored draft the working one (the working draft is stored in its place).",
            true,
        ),
        spec("draft.rename", "Rename Draft", r#"{"id"?: string, "name": string}"#, "Rename a stored draft, or the working one (no id).", true),
        spec("draft.delete", "Delete Draft", r#"{"id": string}"#, "Remove a stored draft for good (one undo step).", true),
        spec("draft.setExport", "Export This Draft", r#"{"id"?: string}"#, "Choose the draft that exports (no id: the working draft).", true),
        spec(
            "draft.compare",
            "Compare Drafts",
            r#"{"old": string, "new"?: string}"#,
            "Compare two drafts scene by scene (no `new`: the working draft): each item, whether it was added, cut or changed, with the words added and removed.",
            false,
        ),
    ]
}

fn name_taken(p: &Project, name: &str, except: Option<&str>) -> bool {
    (drafts::working_name(p) == name && except.is_some()) || p.settings.drafts.iter().any(|d| d.name == name && Some(d.id.as_str()) != except)
}

/// The changes between two drafts as JSON.
pub fn changes_json(changes: &[ItemChange]) -> Value {
    json!(
        changes
            .iter()
            .map(|c| json!({
                "id": c.id.as_text(),
                "title": c.title,
                "state": match (c.in_old, c.in_new) {
                    (false, _) => "added",
                    (_, false) => "cut",
                    _ if c.diff.is_same() => "same",
                    _ => "changed",
                },
                "added": c.diff.added,
                "removed": c.diff.removed,
            }))
            .collect::<Vec<_>>()
    )
}

/// Run a draft command, or `None` if `id` is not one.
pub fn run(s: &mut Session, id: &str, v: &Value) -> Option<Outcome> {
    Some(match id {
        "draft.list" => {
            let p = &s.project;
            let mut rows = vec![
                json!({ "id": "", "name": drafts::working_name(p), "working": true, "words": p.manuscript_words(), "exports": p.settings.export_draft.is_empty() }),
            ];
            for d in &p.settings.drafts {
                rows.push(json!({
                    "id": d.id, "name": d.name, "created": d.created, "working": false,
                    "words": p.drafts.get(&d.id).map_or(0, drafts::DraftBody::words),
                    "exports": p.settings.export_draft == d.id,
                }));
            }
            Ok(json!({ "drafts": rows }))
        }
        "draft.new" => (|| {
            let name = str_param(v, "name")?.to_owned();
            let did = ItemId::new().as_text();
            let created = wordsmith_manuscript_model::snapshot::stamp(SystemTime::now());
            s.change_drafts("New Draft", |p| drafts::start_new(p, &name, &did, &created))?;
            Ok(json!({ "kept": did }))
        })(),
        "draft.switch" => (|| {
            let did = str_param(v, "id")?.to_owned();
            s.change_drafts("Switch Draft", |p| drafts::switch(p, &did))?;
            Ok(json!({}))
        })(),
        "draft.delete" => (|| {
            let did = str_param(v, "id")?.to_owned();
            s.change_drafts("Delete Draft", |p| drafts::delete(p, &did))?;
            Ok(json!({}))
        })(),
        "draft.rename" => (|| {
            let name = str_param(v, "name")?.trim().to_owned();
            if name.is_empty() {
                return Err("a draft needs a name".into());
            }
            let did = opt_str(v, "id").filter(|d| !d.is_empty()).map(str::to_owned);
            if name_taken(&s.project, &name, Some(did.as_deref().unwrap_or(""))) && drafts::working_name(&s.project) != name {
                return Err(format!("there is already a draft named “{name}”"));
            }
            s.change("Rename Draft", &[], &[], |p| {
                match &did {
                    None => p.settings.draft_name = name,
                    Some(d) => p.settings.drafts.iter_mut().find(|x| &x.id == d).ok_or_else(|| format!("no draft {d}"))?.name = name,
                }
                Ok(())
            })?;
            Ok(json!({}))
        })(),
        "draft.setExport" => (|| {
            let did = opt_str(v, "id").unwrap_or("").to_owned();
            if !did.is_empty() && !s.project.settings.drafts.iter().any(|d| d.id == did) {
                return Err(format!("no draft {did}"));
            }
            s.change("Export Draft", &[], &[], |p| {
                p.settings.export_draft = did;
                Ok(())
            })?;
            Ok(json!({}))
        })(),
        "draft.compare" => (|| {
            let old = drafts::body(&s.project, str_param(v, "old")?).ok_or("no such draft")?;
            let new = drafts::body(&s.project, opt_str(v, "new").unwrap_or("")).ok_or("no such draft")?;
            Ok(json!({ "items": changes_json(&drafts::compare(&old, &new)) }))
        })(),
        _ => return None,
    })
}

/// The drafts window's state.
#[derive(Default)]
pub struct DraftsView {
    /// Whether the window is open.
    pub open: bool,
    new_name: String,
    compare: Option<String>,
    expanded: Option<ItemId>,
}

/// Draw the drafts window. Returns errors for the status bar.
pub fn window(ctx: &egui::Context, s: &mut Session, v: &mut DraftsView) -> Vec<String> {
    let mut errors = Vec::new();
    if !v.open {
        return errors;
    }
    let mut open = true;
    let mut todo: Option<(&str, Value)> = None;
    egui::Window::new("Drafts").id(egui::Id::unique("manuscript-drafts")).open(&mut open).default_size([620.0, 480.0]).show(ctx, |ui| {
        let list = run(s, "draft.list", &json!({})).and_then(Result::ok).unwrap_or_default();
        for d in list["drafts"].as_array().cloned().unwrap_or_default() {
            let did = d["id"].as_str().unwrap_or("").to_owned();
            ui.horizontal(|ui| {
                let name = d["name"].as_str().unwrap_or("");
                let working = d["working"].as_bool() == Some(true);
                ui.label(RichText::new(name).strong());
                ui.label(
                    RichText::new(format!(
                        "{} words{}",
                        crate::binder::format_words(d["words"].as_u64().unwrap_or(0) as usize),
                        if working { ", being written" } else { "" }
                    ))
                    .weak(),
                );
                if d["exports"].as_bool() == Some(true) {
                    ui.label(RichText::new("exports").weak());
                } else if ui.small_button("Export This").on_hover_text("Export this draft").clicked() {
                    todo = Some(("draft.setExport", json!({ "id": did })));
                }
                if !working {
                    if ui.small_button("Compare").on_hover_text("Compare with the draft being written, scene by scene").clicked() {
                        v.compare = Some(did.clone());
                    }
                    if ui.small_button("Write This").on_hover_text("Make it the draft being written").clicked() {
                        todo = Some(("draft.switch", json!({ "id": did })));
                    }
                    if ui.small_button("Delete").clicked() {
                        todo = Some(("draft.delete", json!({ "id": did })));
                    }
                }
            });
        }
        ui.separator();
        ui.horizontal(|ui| {
            let r = ui.add(egui::TextEdit::singleline(&mut v.new_name).hint_text("Second Draft").desired_width(180.0));
            r.widget_info(|| egui::WidgetInfo::labeled(egui::Role::TextInput, true, "New draft's name"));
            if ui.button("New Draft").on_hover_text("Keep this draft as it is, and carry on writing as the new one").clicked()
                && !v.new_name.trim().is_empty()
            {
                todo = Some(("draft.new", json!({ "name": v.new_name.trim() })));
                v.new_name.clear();
            }
        });
        if let Some(old) = v.compare.clone() {
            ui.separator();
            let name = s.project.settings.drafts.iter().find(|d| d.id == old).map(|d| d.name.clone()).unwrap_or_default();
            ui.label(RichText::new(format!("“{name}” and “{}”, scene by scene", drafts::working_name(&s.project))).strong());
            let (Some(a), Some(b)) = (drafts::body(&s.project, &old), drafts::body(&s.project, "")) else { return };
            let changes = drafts::compare(&a, &b);
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                for c in &changes {
                    let state = match (c.in_old, c.in_new) {
                        (false, _) => "added".to_owned(),
                        (_, false) => "cut".to_owned(),
                        _ if c.diff.is_same() => "same".to_owned(),
                        _ => format!("+{} −{} words", c.diff.added, c.diff.removed),
                    };
                    let open_this = v.expanded == Some(c.id);
                    if ui.selectable_label(open_this, format!("{}  ·  {state}", c.title)).clicked() {
                        v.expanded = if open_this { None } else { Some(c.id) };
                    }
                    if open_this && !c.diff.is_same() {
                        let t = jcj_theme::Theme::get(ui.ctx());
                        for p in &c.diff.paras {
                            if p.iter().all(|seg| seg.change == Change::Same) {
                                continue;
                            }
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
                            let plain: String = p.iter().map(|x| x.text.as_str()).collect();
                            ui.label(job).widget_info(|| egui::WidgetInfo::labeled(egui::Role::Label, true, &plain));
                        }
                    }
                }
            });
        }
    });
    if let Some((cmd, args)) = todo
        && let Err(e) = s.run(cmd, &args)
    {
        errors.push(e);
    }
    if !open {
        v.open = false;
    }
    errors
}
