// SPDX-License-Identifier: MIT OR Apache-2.0
//! Thread lanes (PLN-4): the scenes in story order, one row (lane) per plot thread or per
//! point-of-view character, so the writer sees which threads carry which stretch of the book and
//! where one goes quiet.
//!
//! A scene is in a thread's lane when it is linked to that thread's bible entry (the inspector's
//! linked entries, INS-4), and in a point of view's lane when its Point of view field says so.
//! Scenes in no lane sit in the last row. Dragging a card to another lane links the scene to that
//! thread instead (or sets its point of view), as one undo step.
//!
//! core-candidate: open-source-core issue #12 (C11, story board lanes). The plan meant to promote
//! Wordsmith Script's lanes into the core; code never moves between app repositories, so this is
//! built here until the core has lanes both apps can use.

use jcj_shell::egui::{self, Color32, Rect, RichText, Sense, Stroke, Ui, pos2, vec2};
use serde_json::{Value, json};
use wordsmith_manuscript_model::{Area, ItemId, Kind, POV_FIELD, Project, Role};

use crate::commands::{Spec, id_param, opt_str};
use crate::session::{Outcome, Session};

/// What the lanes are.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum By {
    /// One lane per plot thread (bible entries of the `thread` type).
    #[default]
    Thread,
    /// One lane per point of view.
    Pov,
}

impl By {
    fn of(v: &Value) -> Result<By, String> {
        match opt_str(v, "by").unwrap_or("thread") {
            "thread" => Ok(By::Thread),
            "pov" => Ok(By::Pov),
            other => Err(format!("`{other}` is not a kind of lane (thread or pov)")),
        }
    }
}

/// One lane: its key (an entry id or a point of view), title, and which scenes (column
/// indexes) are in it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Lane {
    /// The thread's entry id, the point of view, or empty for "in no lane".
    pub key: String,
    /// What the lane is called.
    pub title: String,
    /// The columns (scenes) in it.
    pub scenes: Vec<usize>,
}

/// The scenes in story order: the included manuscript's scenes (and texts with no role).
pub fn scenes(p: &Project) -> Vec<ItemId> {
    p.included_manuscript()
        .into_iter()
        .filter(|i| p.binder.get(*i).is_some_and(|it| it.kind == Kind::Text && matches!(it.role, Role::Scene | Role::None)))
        .collect()
}

/// The lanes of a project.
pub fn lanes(p: &Project, by: By) -> (Vec<ItemId>, Vec<Lane>) {
    let cols = scenes(p);
    let mut out: Vec<Lane> = Vec::new();
    match by {
        By::Thread => {
            for r in p.binder.rows_of(Area::Bible) {
                if let Some(e) = p.entries.get(&r.item.id)
                    && e.kind == "thread"
                {
                    let scenes = cols
                        .iter()
                        .enumerate()
                        .filter(|(_, s)| p.text(**s).is_some_and(|t| t.meta.entries.contains(&r.item.id)))
                        .map(|(n, _)| n)
                        .collect();
                    out.push(Lane { key: r.item.id.as_text(), title: e.name.clone(), scenes });
                }
            }
        }
        By::Pov => {
            for (n, s) in cols.iter().enumerate() {
                let pov = p.text(*s).and_then(|t| t.meta.fields.get(POV_FIELD)).cloned().unwrap_or_default();
                if pov.is_empty() {
                    continue;
                }
                match out.iter_mut().find(|l| l.key == pov) {
                    Some(l) => l.scenes.push(n),
                    None => out.push(Lane { key: pov.clone(), title: pov, scenes: vec![n] }),
                }
            }
        }
    }
    let placed: Vec<usize> = out.iter().flat_map(|l| l.scenes.iter().copied()).collect();
    let rest: Vec<usize> = (0..cols.len()).filter(|n| !placed.contains(n)).collect();
    out.push(Lane { key: String::new(), title: if by == By::Thread { "No thread".into() } else { "No point of view".into() }, scenes: rest });
    (cols, out)
}

/// The lane commands.
pub fn specs() -> Vec<Spec> {
    let spec = |id, label, params, about, mutates| Spec { id, label, key: "", params, about, mutates };
    vec![
        spec(
            "lanes.get",
            "Thread Lanes",
            r#"{"by"?: "thread"|"pov"}"#,
            "The scenes in story order and the lanes (plot threads or points of view) each is in.",
            false,
        ),
        spec(
            "lanes.move",
            "Move to Lane",
            r#"{"id": id, "by"?: "thread"|"pov", "from"?: string, "to": string}"#,
            "Put a scene in another lane: link it to that thread (unlinking `from`), or set its point of view (empty `to`: none).",
            true,
        ),
    ]
}

/// Run a lane command, or `None` if `id` is not one.
pub fn run(s: &mut Session, id: &str, v: &Value) -> Option<Outcome> {
    Some(match id {
        "lanes.get" => (|| {
            let (cols, lanes) = lanes(&s.project, By::of(v)?);
            Ok(json!({
                "scenes": cols.iter().map(|c| json!({ "id": c.as_text(), "title": s.project.binder.get(*c).map(|i| i.title.clone()).unwrap_or_default() })).collect::<Vec<_>>(),
                "lanes": lanes.iter().map(|l| json!({ "key": l.key, "title": l.title, "scenes": l.scenes })).collect::<Vec<_>>(),
            }))
        })(),
        "lanes.move" => (|| {
            let scene = id_param(v, "id")?;
            let by = By::of(v)?;
            let to = opt_str(v, "to").unwrap_or("").to_owned();
            let from = opt_str(v, "from").unwrap_or("").to_owned();
            match by {
                By::Pov => s.run("item.setField", &json!({ "id": scene.as_text(), "field": POV_FIELD, "value": to })).map(|_| json!({})),
                By::Thread => {
                    let parse = |k: &str| -> Result<Option<ItemId>, String> {
                        if k.is_empty() {
                            return Ok(None);
                        }
                        let id: ItemId = k.parse().map_err(|_| format!("`{k}` is not a thread"))?;
                        if s.project.entries.get(&id).is_none_or(|e| e.kind != "thread") {
                            return Err(format!("`{k}` is not a plot thread"));
                        }
                        Ok(Some(id))
                    };
                    let (from, to) = (parse(&from)?, parse(&to)?);
                    s.change("Move to Lane", &[scene], &[], |p| {
                        let m = &mut p.text_mut(scene).meta;
                        if let Some(f) = from {
                            m.entries.retain(|e| *e != f);
                        }
                        if let Some(t) = to
                            && !m.entries.contains(&t)
                        {
                            m.entries.push(t);
                        }
                        Ok(())
                    })?;
                    Ok(json!({}))
                }
            }
        })(),
        _ => return None,
    })
}

/// The lanes view's state.
#[derive(Default)]
pub struct LanesView {
    /// What the lanes are.
    pub by: By,
    drag: Option<(ItemId, String)>,
}

/// Draw the lanes. Returns an item to open and errors.
pub fn view(ui: &mut Ui, s: &mut Session, v: &mut LanesView) -> (Option<ItemId>, Vec<String>) {
    let mut open = None;
    let mut errors = Vec::new();
    ui.horizontal(|ui| {
        ui.heading(crate::i18n::tr("Thread Lanes"));
        ui.selectable_value(&mut v.by, By::Thread, "Plot threads");
        ui.selectable_value(&mut v.by, By::Pov, "Points of view");
    });
    let (cols, lanes) = lanes(&s.project, v.by);
    if cols.is_empty() {
        ui.label("The manuscript has no scenes yet.");
        return (open, errors);
    }
    if v.by == By::Thread && lanes.len() == 1 {
        ui.label(RichText::new("No plot threads yet: add Plot Thread entries to the Story Bible and link scenes to them in the inspector.").weak());
    }
    let t = jcj_theme::Theme::get(ui.ctx());
    let (head_w, cell_w, row_h) = (150.0, 118.0, 56.0);
    let mut drop: Option<(ItemId, String, String)> = None;
    egui::ScrollArea::both().id_salt("manuscript-lanes").auto_shrink([false, false]).show(ui, |ui| {
        let size = vec2(head_w + cols.len() as f32 * cell_w + 8.0, lanes.len() as f32 * row_h + 28.0);
        let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
        let painter = ui.painter_at(rect);
        let pointer = ui.input(|i| i.pointer.interact_pos());
        for (n, c) in cols.iter().enumerate() {
            let title = s.project.binder.get(*c).map(|i| i.title.clone()).unwrap_or_default();
            let x = rect.min.x + head_w + n as f32 * cell_w;
            painter.text(
                pos2(x + 4.0, rect.min.y + 4.0),
                egui::Align2::LEFT_TOP,
                format!("{}. {}", n + 1, title.chars().take(14).collect::<String>()),
                jcj_theme::regular(t.type_sizes.caption),
                t.text_dim,
            );
        }
        for (r, lane) in lanes.iter().enumerate() {
            let y = rect.min.y + 24.0 + r as f32 * row_h;
            let row = Rect::from_min_size(pos2(rect.min.x, y), vec2(size.x, row_h - 4.0));
            painter.rect_filled(row, 4.0, if r % 2 == 0 { t.surface } else { t.canvas });
            painter.text(
                pos2(rect.min.x + 6.0, y + row_h / 2.0 - 2.0),
                egui::Align2::LEFT_CENTER,
                &lane.title,
                jcj_theme::regular(t.type_sizes.body),
                t.text,
            );
            if let (Some((dragged, from)), Some(p)) = (&v.drag, pointer)
                && row.contains(p)
                && ui.input(|i| i.pointer.any_released())
                && *from != lane.key
            {
                drop = Some((*dragged, from.clone(), lane.key.clone()));
            }
            for &n in &lane.scenes {
                let Some(id) = cols.get(n).copied() else { continue };
                let card = Rect::from_min_size(pos2(rect.min.x + head_w + n as f32 * cell_w + 4.0, y + 6.0), vec2(cell_w - 8.0, row_h - 16.0));
                let resp = ui.interact(card, ui.scope_id().with(("lane-card", r, n)), Sense::click_and_drag());
                let title = s.project.binder.get(id).map(|i| i.title.clone()).unwrap_or_default();
                let fill = if resp.hovered() { t.accent_soft } else { t.surface_high };
                painter.rect_filled(card, 4.0, fill);
                painter.rect_stroke(card, 4.0, Stroke::new(1.0, t.border), egui::StrokeKind::Inside);
                painter.text(
                    card.left_top() + vec2(6.0, 6.0),
                    egui::Align2::LEFT_TOP,
                    title.chars().take(16).collect::<String>(),
                    jcj_theme::regular(t.type_sizes.caption),
                    t.text,
                );
                let label = format!("{title}, in {}", lane.title);
                resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &label));
                if resp.drag_started() {
                    v.drag = Some((id, lane.key.clone()));
                }
                if resp.double_clicked() {
                    open = Some(id);
                }
            }
        }
        if let (Some(_), Some(p)) = (&v.drag, pointer) {
            painter.circle_filled(p, 5.0, Color32::from_rgba_unmultiplied(t.accent.r(), t.accent.g(), t.accent.b(), 160));
        }
    });
    if ui.input(|i| i.pointer.any_released()) {
        v.drag = None;
    }
    if let Some((id, from, to)) = drop {
        let by = if v.by == By::Thread { "thread" } else { "pov" };
        if let Err(e) = s.run("lanes.move", &json!({ "id": id.as_text(), "by": by, "from": from, "to": to })) {
            errors.push(e);
        }
    }
    (open, errors)
}
