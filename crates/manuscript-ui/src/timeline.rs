// SPDX-License-Identifier: MIT OR Apache-2.0
//! The timeline (PLN-5): the scenes on the story's own calendar, from each scene's date.
//!
//! A scene's date is its value of the project's first Date field (INS-6; the view offers to add
//! a "Scene date" field when there is none). Scenes are placed along a line from the earliest
//! date to the latest, those on the same day stacked; each card shows its place in story order,
//! so a flashback stands out. Undated scenes are listed beside it.

use jcj_shell::egui::{self, Rect, RichText, Sense, Stroke, Ui, pos2, vec2};
use jcj_shell::writing::{Day, days_from_civil};
use serde_json::{Value, json};
use wordsmith_manuscript_model::{FieldKind, ItemId, Project};

use crate::commands::Spec;
use crate::session::{Outcome, Session};

/// The name of the field the view adds when the project has no date field.
pub const SCENE_DATE: &str = "Scene date";

/// The project's date field, if it has one.
pub fn date_field(p: &Project) -> Option<String> {
    p.settings.fields.iter().find(|f| f.kind == FieldKind::Date).map(|f| f.name.clone())
}

/// One scene on the timeline.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Placed {
    /// The scene.
    pub id: ItemId,
    /// Its place in story order (from 1).
    pub order: usize,
    /// Its date.
    pub day: Day,
}

/// The dated scenes in date order (story order within a day), and the undated ones in story order.
pub fn placed(p: &Project) -> (Vec<Placed>, Vec<ItemId>) {
    let Some(field) = date_field(p) else { return (Vec::new(), crate::lanes::scenes(p)) };
    let mut dated = Vec::new();
    let mut undated = Vec::new();
    for (n, id) in crate::lanes::scenes(p).into_iter().enumerate() {
        match p.text(id).and_then(|t| t.meta.fields.get(&field)).and_then(|d| Day::parse(d)) {
            Some(day) => dated.push(Placed { id, order: n + 1, day }),
            None => undated.push(id),
        }
    }
    dated.sort_by_key(|x| (x.day, x.order));
    (dated, undated)
}

/// The timeline commands.
pub fn specs() -> Vec<Spec> {
    vec![Spec {
        id: "timeline.get",
        label: "Timeline",
        key: "",
        params: "{}",
        about: "The scenes in the story's own time: each dated scene with its date and its place in story order, then the undated ones.",
        mutates: false,
    }]
}

/// Run a timeline command, or `None` if `id` is not one.
pub fn run(s: &mut Session, id: &str, _v: &Value) -> Option<Outcome> {
    if id != "timeline.get" {
        return None;
    }
    let (dated, undated) = placed(&s.project);
    let title = |i: ItemId| s.project.binder.get(i).map(|x| x.title.clone()).unwrap_or_default();
    Some(Ok(json!({
        "field": date_field(&s.project),
        "dated": dated.iter().map(|d| json!({ "id": d.id.as_text(), "title": title(d.id), "date": d.day.iso(), "story_order": d.order })).collect::<Vec<_>>(),
        "undated": undated.iter().map(|i| json!({ "id": i.as_text(), "title": title(*i) })).collect::<Vec<_>>(),
    })))
}

/// Draw the timeline. Returns a scene to open and errors.
pub fn view(ui: &mut Ui, s: &mut Session) -> (Option<ItemId>, Vec<String>) {
    let mut open = None;
    let mut errors = Vec::new();
    ui.heading(jcj_shell::i18n::tr("Timeline"));
    let Some(field) = date_field(&s.project) else {
        ui.label("The timeline places scenes by a date field, and this project has none yet.");
        if ui.button("Add a Scene Date Field").clicked() {
            let mut defs = crate::fields::defs_json(&s.project);
            if let Some(a) = defs.as_array_mut() {
                a.push(json!({ "name": SCENE_DATE, "kind": "date" }));
            }
            if let Err(e) = s.run("project.setFields", &json!({ "fields": defs })) {
                errors.push(e);
            }
        }
        return (open, errors);
    };
    let (dated, undated) = placed(&s.project);
    ui.label(
        RichText::new(format!(
            "Scenes by their “{field}” (YYYY-MM-DD, set in the inspector or the outline). The number on each card is its place in the book."
        ))
        .weak(),
    );
    let t = jcj_theme::Theme::get(ui.ctx());
    if let (Some(first), Some(last)) = (dated.first(), dated.last()) {
        let span = (days_from_civil(last.day) - days_from_civil(first.day)).max(1) as f32;
        let mut stacks: std::collections::BTreeMap<Day, usize> = std::collections::BTreeMap::new();
        let (card_w, card_h) = (130.0, 40.0);
        let tallest = dated.iter().fold(std::collections::BTreeMap::<Day, usize>::new(), |mut m, d| {
            *m.entry(d.day).or_default() += 1;
            m
        });
        let rows = tallest.values().copied().max().unwrap_or(1);
        egui::ScrollArea::horizontal().id_salt("manuscript-timeline").show(ui, |ui| {
            let width = (ui.available_width() - 20.0).max(dated.len() as f32 * 40.0 + card_w).max(600.0);
            let height = 60.0 + rows as f32 * (card_h + 6.0);
            let (rect, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
            let painter = ui.painter_at(rect);
            let line_y = rect.min.y + 24.0;
            let x_of = |d: Day| rect.min.x + 10.0 + (days_from_civil(d) - days_from_civil(first.day)) as f32 / span * (width - card_w - 20.0);
            painter.line_segment([pos2(rect.min.x + 10.0, line_y), pos2(rect.max.x - 10.0, line_y)], Stroke::new(2.0, t.border));
            for d in tallest.keys() {
                let x = x_of(*d);
                painter.circle_filled(pos2(x, line_y), 4.0, t.accent);
                painter.text(pos2(x, line_y - 8.0), egui::Align2::CENTER_BOTTOM, d.iso(), jcj_theme::regular(t.type_sizes.caption), t.text_dim);
            }
            for d in &dated {
                let k = stacks.entry(d.day).or_default();
                let card = Rect::from_min_size(pos2(x_of(d.day) - 6.0, line_y + 14.0 + *k as f32 * (card_h + 6.0)), vec2(card_w, card_h));
                *k += 1;
                let title = s.project.binder.get(d.id).map(|i| i.title.clone()).unwrap_or_default();
                let resp = ui.interact(card, ui.scope_id().with(("timeline-card", d.id.as_text())), Sense::click());
                painter.rect_filled(card, 4.0, if resp.hovered() { t.accent_soft } else { t.surface_high });
                painter.rect_stroke(card, 4.0, Stroke::new(1.0, t.border), egui::StrokeKind::Inside);
                painter.text(
                    card.left_top() + vec2(6.0, 4.0),
                    egui::Align2::LEFT_TOP,
                    format!("{}. {}", d.order, title.chars().take(14).collect::<String>()),
                    jcj_theme::regular(t.type_sizes.caption),
                    t.text,
                );
                painter.text(
                    card.left_bottom() + vec2(6.0, -4.0),
                    egui::Align2::LEFT_BOTTOM,
                    d.day.iso(),
                    jcj_theme::regular(t.type_sizes.caption),
                    t.text_dim,
                );
                let label = format!("{title}, {}, scene {} of the book", d.day.iso(), d.order);
                resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &label));
                if resp.double_clicked() {
                    open = Some(d.id);
                }
            }
        });
    } else {
        ui.label("No scene has a date yet.");
    }
    if !undated.is_empty() {
        ui.separator();
        ui.label(RichText::new(format!("Without a date ({})", undated.len())).strong());
        ui.horizontal_wrapped(|ui| {
            for i in &undated {
                let title = s.project.binder.get(*i).map(|x| x.title.clone()).unwrap_or_default();
                if ui.small_button(title).clicked() {
                    open = Some(*i);
                }
            }
        });
    }
    (open, errors)
}
