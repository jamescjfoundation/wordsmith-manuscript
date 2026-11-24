// SPDX-License-Identifier: MIT OR Apache-2.0
//! Writing history (PRG-3): the words written each day, shown as a calendar.
//!
//! The core's writing statistics count net words per day ([`jcj_shell::writing::History`]). The
//! history is the writer's own record, not part of the book: it is kept on this computer, in the
//! app's settings folder, one file per project (named by a hash of the project folder's path,
//! so no title or path is written), and never in the project folder.

use std::path::{Path, PathBuf};

use jcj_shell::egui::{self, RichText, Ui};
use jcj_shell::writing::{Day, History};
use sha2::{Digest, Sha256};

/// The file that keeps a project's history.
pub fn file_for(dir: &Path, root: &Path) -> PathBuf {
    let key = std::fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
    let hash = Sha256::digest(key.to_string_lossy().as_bytes());
    let hex: String = hash.iter().take(12).map(|b| format!("{b:02x}")).collect();
    dir.join(format!("{hex}.json"))
}

/// A project's history (empty when there is none yet, or it cannot be read).
pub fn load(dir: &Path, root: &Path) -> History {
    let path = file_for(dir, root);
    match std::fs::metadata(&path) {
        Ok(m) if m.len() <= 4 << 20 => std::fs::read(&path).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default(),
        _ => History::default(),
    }
}

/// Keep a project's history.
pub fn save(dir: &Path, root: &Path, h: &History) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("the writing history could not be kept: {e}"))?;
    let bytes = serde_json::to_vec_pretty(h).map_err(|e| e.to_string())?;
    let path = file_for(dir, root);
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    wordsmith_manuscript_format::folder::Sink::write(&mut wordsmith_manuscript_format::folder::Disk::new(dir), &name, &bytes)
        .map_err(|e| format!("the writing history could not be kept: {e}"))
}

/// Today, in the computer's time zone.
pub fn local_today() -> Day {
    let offset = i64::from(chrono::Local::now().offset().local_minus_utc());
    Day::of(std::time::SystemTime::now(), offset)
}

/// The calendar window's state.
#[derive(Clone, Debug, Default)]
pub struct HistoryView {
    /// Whether the window is open.
    pub open: bool,
    /// The month shown (year, month); `None`: this month.
    pub month: Option<(i32, u8)>,
}

const MONTHS: [&str; 12] = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];

fn days_in(year: i32, month: u8) -> u8 {
    let first = Day { year, month, day: 1 };
    let next = if month == 12 { Day { year: year + 1, month: 1, day: 1 } } else { Day { year, month: month + 1, day: 1 } };
    (jcj_shell::writing::days_from_civil(next) - jcj_shell::writing::days_from_civil(first)) as u8
}

/// What one month of a history adds up to: words, days written, the best day.
pub fn month_totals(h: &History, year: i32, month: u8) -> (i64, u32, Option<(Day, i64)>) {
    let mut total = 0;
    let mut days = 0;
    let mut best: Option<(Day, i64)> = None;
    for d in 1..=days_in(year, month) {
        let day = Day { year, month, day: d };
        let w = h.on(day);
        total += w;
        if w > 0 {
            days += 1;
            if best.is_none_or(|(_, b)| w > b) {
                best = Some((day, w));
            }
        }
    }
    (total, days, best)
}

/// Draw the calendar window.
pub fn window(ctx: &egui::Context, h: &History, v: &mut HistoryView, today: Day) {
    if !v.open {
        return;
    }
    let (year, month) = v.month.unwrap_or((today.year, today.month));
    let mut open = true;
    egui::Window::new("Writing History").id(egui::Id::unique("manuscript-history")).open(&mut open).resizable(false).show(ctx, |ui| {
        calendar(ui, h, v, year, month, today);
    });
    if !open {
        v.open = false;
    }
}

/// One month as a grid of days, each with its words; darker days had more.
pub fn calendar(ui: &mut Ui, h: &History, v: &mut HistoryView, year: i32, month: u8, today: Day) {
    let t = jcj_theme::Theme::get(ui.ctx());
    ui.horizontal(|ui| {
        if ui.small_button("◀").on_hover_text("Previous month").clicked() {
            v.month = Some(if month == 1 { (year - 1, 12) } else { (year, month - 1) });
        }
        ui.label(RichText::new(format!("{} {year}", MONTHS[(month as usize).saturating_sub(1).min(11)])).strong());
        if ui.small_button("▶").on_hover_text("Next month").clicked() {
            v.month = Some(if month == 12 { (year + 1, 1) } else { (year, month + 1) });
        }
        if v.month.is_some() && ui.small_button("This Month").clicked() {
            v.month = None;
        }
    });
    let (total, days, best) = month_totals(h, year, month);
    ui.label(format!(
        "{} words on {days} day{}{}. Streak: {} day{}.",
        crate::binder::format_words(total.max(0) as usize),
        if days == 1 { "" } else { "s" },
        best.map(|(d, w)| format!(", most on the {} ({})", d.day, crate::binder::format_words(w.max(0) as usize))).unwrap_or_default(),
        h.streak(today),
        if h.streak(today) == 1 { "" } else { "s" },
    ));
    let most = (1..=days_in(year, month)).map(|d| h.on(Day { year, month, day: d })).max().unwrap_or(0).max(1);
    let cell = egui::vec2(56.0, 44.0);
    egui::Grid::new("manuscript-history-grid").spacing(egui::vec2(4.0, 4.0)).show(ui, |ui| {
        for name in ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"] {
            ui.label(RichText::new(name).weak());
        }
        ui.end_row();
        let first = Day { year, month, day: 1 };
        for _ in 0..first.weekday() {
            ui.label("");
        }
        for d in 1..=days_in(year, month) {
            let day = Day { year, month, day: d };
            let w = h.on(day);
            let (rect, resp) = ui.allocate_exact_size(cell, egui::Sense::hover());
            let share = if w > 0 { 0.2 + 0.8 * (w as f32 / most as f32) } else { 0.0 };
            let fill = if w > 0 { t.accent.gamma_multiply(share) } else { t.surface };
            ui.painter().rect_filled(rect, 4.0, fill);
            if day == today {
                ui.painter().rect_stroke(rect, 4.0, egui::Stroke::new(1.5, t.accent), egui::StrokeKind::Inside);
            }
            let text_color = if share > 0.6 { t.on_brand } else { t.text };
            ui.painter().text(
                rect.left_top() + egui::vec2(5.0, 3.0),
                egui::Align2::LEFT_TOP,
                d.to_string(),
                jcj_theme::regular(t.type_sizes.caption),
                text_color,
            );
            if w != 0 {
                ui.painter().text(
                    rect.right_bottom() - egui::vec2(5.0, 3.0),
                    egui::Align2::RIGHT_BOTTOM,
                    crate::binder::format_words(w.max(0) as usize),
                    jcj_theme::regular(t.type_sizes.caption),
                    text_color,
                );
            }
            let label = format!("{} {d}: {} words", MONTHS[(month as usize - 1).min(11)], w);
            resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Label, true, &label));
            if (u32::from(first.weekday()) + u32::from(d)) % 7 == 0 {
                ui.end_row();
            }
        }
    });
}
