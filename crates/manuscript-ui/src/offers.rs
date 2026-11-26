// SPDX-License-Identifier: MIT OR Apache-2.0
//! The "Offered fixes" inbox (plan section 9.2 item 4): fixes Wordsmith Book offers back, each
//! shown with the words it changes, for the writer to accept or reject.
//!
//! An offer is read from Book's offers file beside the project folder
//! (`manuscript-format::offers`). Accepting one replaces exactly those words in that item's
//! paragraph (the new words take the formatting of the old), as one undo step; an offer whose
//! words are no longer in the text exactly once is "stale" and cannot be accepted. Decisions are
//! kept in the project for Book to read.

use std::path::{Path, PathBuf};

use jcj_shell::egui::{self, RichText};
use serde_json::{Value, json};
use wordsmith_manuscript_format::offers::{self, Decision, Offer, OffersFile};
use wordsmith_manuscript_model::diff::{self, Change};
use wordsmith_manuscript_model::doc as mdoc;
use wordsmith_manuscript_model::{Project, Text};

use crate::commands::{Spec, opt_str, str_param};
use crate::session::{Outcome, Session};

/// The offer commands.
pub fn specs() -> Vec<Spec> {
    let spec = |id, label, params, about, mutates| Spec { id, label, key: "", params, about, mutates };
    vec![
        spec(
            "offers.list",
            "Offered Fixes",
            r#"{"path"?: string}"#,
            "The fixes offered back (from the offers file beside the project, or `path`): id, item, words before and after, note, and state (pending, accepted, rejected, stale).",
            false,
        ),
        spec("offers.accept", "Accept Offered Fix", r#"{"id": string, "path"?: string}"#, "Take an offered fix into the text (one undo step).", true),
        spec("offers.reject", "Reject Offered Fix", r#"{"id": string, "path"?: string}"#, "Turn an offered fix down.", true),
    ]
}

/// The offers file to read: given, else the one beside the project folder.
fn path_of(s: &Session, v: &Value) -> Result<PathBuf, String> {
    match opt_str(v, "path") {
        Some(p) => Ok(PathBuf::from(p)),
        None => s.root().map(offers::offers_path).ok_or_else(|| "save the project first: offers come from a file beside its folder".to_owned()),
    }
}

/// Read the offers file (none yet: no offers).
pub fn load(path: &Path) -> Result<OffersFile, String> {
    match std::fs::metadata(path) {
        Err(_) => Ok(OffersFile { version: offers::VERSION, offers: Vec::new() }),
        Ok(m) if m.len() > offers::MAX_FILE => Err(format!("{} is too large", path.display())),
        Ok(_) => offers::read(&std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?),
    }
}

/// Where an offer's words are in its item's text: the paragraph index (of the editable
/// document) and the byte range, when they occur exactly once.
fn locate(p: &Project, o: &Offer) -> Option<(usize, std::ops::Range<usize>)> {
    let t = p.text(o.item)?;
    let doc = mdoc::to_document(&t.body);
    let mut found = None;
    for (i, b) in doc.body.iter().enumerate() {
        let Some(para) = b.as_para() else { continue };
        let mut from = 0;
        while let Some(at) = para.text[from..].find(&o.find) {
            if found.is_some() {
                return None;
            }
            let start = from + at;
            found = Some((i, start..start + o.find.len()));
            from = start + o.find.len().max(1);
        }
    }
    found
}

/// An offer's state for the project now.
pub fn state(p: &Project, o: &Offer) -> &'static str {
    match offers::decisions(p).get(&o.id) {
        Some(Decision::Accepted) => "accepted",
        Some(Decision::Rejected) => "rejected",
        None if locate(p, o).is_some() => "pending",
        None => "stale",
    }
}

fn apply(t: &mut Text, o: &Offer, at: (usize, std::ops::Range<usize>)) -> Result<(), String> {
    let mut doc = mdoc::to_document(t);
    let (i, r) = at;
    let block = doc.body.get_mut(i).ok_or("the paragraph is gone")?;
    let jcj_doc::Block::Para(para) = std::sync::Arc::make_mut(block) else { return Err("the paragraph is gone".into()) };
    let props = para.props_of_char(r.start).clone();
    para.delete(r.start, r.end).map_err(|e| format!("{e:?}"))?;
    para.insert_text(r.start, &o.replace, &props).map_err(|e| format!("{e:?}"))?;
    *t = mdoc::from_document(&doc);
    Ok(())
}

/// Run an offer command, or `None` if `id` is not one.
pub fn run(s: &mut Session, id: &str, v: &Value) -> Option<Outcome> {
    Some(match id {
        "offers.list" => (|| {
            let file = load(&path_of(s, v)?)?;
            Ok(json!({
                "offers": file.offers.iter().map(|o| json!({
                    "id": o.id, "item": o.item.as_text(),
                    "title": s.project.binder.get(o.item).map(|i| i.title.clone()).unwrap_or_default(),
                    "find": o.find, "replace": o.replace, "note": o.note, "made": o.made,
                    "state": state(&s.project, o),
                })).collect::<Vec<_>>(),
            }))
        })(),
        "offers.accept" | "offers.reject" => (|| {
            let file = load(&path_of(s, v)?)?;
            let oid = str_param(v, "id")?;
            let o = file.offers.iter().find(|o| o.id == oid).cloned().ok_or_else(|| format!("no offered fix `{oid}`"))?;
            if offers::decisions(&s.project).contains_key(&o.id) {
                return Err("this fix was already decided".into());
            }
            if id == "offers.reject" {
                s.change("Reject Offered Fix", &[], &[], |p| offers::decide(p, &o.id, Decision::Rejected))?;
                return Ok(json!({}));
            }
            let at = locate(&s.project, &o).ok_or("the text has changed since this fix was offered: its words are not there exactly once")?;
            s.change("Accept Offered Fix", &[o.item], &[], |p| {
                apply(&mut p.text_mut(o.item).body, &o, at)?;
                offers::decide(p, &o.id, Decision::Accepted)
            })?;
            Ok(json!({}))
        })(),
        _ => return None,
    })
}

/// The inbox window's state.
#[derive(Default)]
pub struct InboxView {
    /// Whether the window is open.
    pub open: bool,
    /// Show decided offers too.
    pub all: bool,
}

/// How many offers wait for the writer (for the status bar).
pub fn pending(s: &Session) -> usize {
    let Some(root) = s.root() else { return 0 };
    load(&offers::offers_path(root)).map(|f| f.offers.iter().filter(|o| state(&s.project, o) == "pending").count()).unwrap_or(0)
}

/// Draw the inbox. Returns errors and an item to open.
pub fn window(ctx: &egui::Context, s: &mut Session, v: &mut InboxView) -> (Vec<String>, Option<wordsmith_manuscript_model::ItemId>) {
    let mut errors = Vec::new();
    let mut open_item = None;
    if !v.open {
        return (errors, open_item);
    }
    let file = match s.root().map(offers::offers_path) {
        Some(p) => load(&p),
        None => Err("Save the project first: offers come from a file beside its folder.".into()),
    };
    let mut open = true;
    egui::Window::new(jcj_shell::i18n::tr("Offered Fixes"))
        .id(egui::Id::unique("manuscript-offers"))
        .open(&mut open)
        .default_size([560.0, 440.0])
        .show(ctx, |ui| {
            let file = match file {
                Ok(f) => f,
                Err(e) => {
                    ui.label(e);
                    return;
                }
            };
            ui.label(RichText::new("Fixes Wordsmith Book offers back. Nothing changes until you accept one.").weak());
            ui.checkbox(&mut v.all, "Show decided fixes too");
            let mut todo: Option<(&str, String)> = None;
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                let shown: Vec<&Offer> = file.offers.iter().filter(|o| v.all || matches!(state(&s.project, o), "pending" | "stale")).collect();
                if shown.is_empty() {
                    ui.label(jcj_shell::i18n::tr("No fixes are waiting."));
                }
                for o in shown {
                    let st = state(&s.project, o);
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            let title =
                                s.project.binder.get(o.item).map_or_else(|| "(an item no longer in the project)".to_owned(), |i| i.title.clone());
                            ui.label(RichText::new(title).strong());
                            ui.label(RichText::new(st).weak());
                        });
                        if !o.note.is_empty() {
                            ui.label(&o.note);
                        }
                        let t = jcj_theme::Theme::get(ui.ctx());
                        let mut job = egui::text::LayoutJob::default();
                        for seg in diff::paragraph(&o.find, &o.replace) {
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
                        let shown = format!("{} → {}", o.find, o.replace);
                        ui.label(job).widget_info(|| egui::WidgetInfo::labeled(egui::Role::Label, true, &shown));
                        ui.horizontal(|ui| {
                            if st == "pending" && ui.button("Accept").clicked() {
                                todo = Some(("offers.accept", o.id.clone()));
                            }
                            if matches!(st, "pending" | "stale") && ui.button("Reject").clicked() {
                                todo = Some(("offers.reject", o.id.clone()));
                            }
                            if s.project.binder.contains(o.item) && ui.button("Open").clicked() {
                                open_item = Some(o.item);
                            }
                        });
                    });
                }
            });
            if let Some((cmd, oid)) = todo
                && let Err(e) = s.run(cmd, &json!({ "id": oid }))
            {
                errors.push(e);
            }
        });
    if !open {
        v.open = false;
    }
    (errors, open_item)
}
