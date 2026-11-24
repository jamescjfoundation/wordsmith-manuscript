// SPDX-License-Identifier: MIT OR Apache-2.0
//! Counts and targets (PRG-1, PRG-2; SPEC 10): live words and characters for the document, the
//! selection, a folder and the manuscript; the manuscript's target with a deadline over writing
//! days; the session's target; per-item targets (shown in the outline and inspector). The
//! session and the day's words come from the core's writing statistics (`jcj_shell::writing`).

use std::time::{Instant, SystemTime};

use jcj_shell::egui::Ui;
use jcj_shell::writing::{Day, Writing, progress_bar, words_per_day};
use serde_json::{Value, json};
use wordsmith_manuscript_model::{DAYS, ItemId, Project};

use crate::commands::{Spec, opt_id};
use crate::session::{Outcome, Session};

/// Words and characters of some text.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counts {
    /// Words.
    pub words: usize,
    /// Characters (line breaks not counted).
    pub chars: usize,
    /// Characters without spaces.
    pub chars_no_spaces: usize,
}

impl Counts {
    /// The counts of some paragraphs.
    pub fn of<S: AsRef<str>>(paras: &[S]) -> Counts {
        let mut c = Counts::default();
        for p in paras {
            let p = p.as_ref();
            c.words += wordsmith_manuscript_model::words::count(p);
            c.chars += wordsmith_manuscript_model::words::chars(p);
            c.chars_no_spaces += wordsmith_manuscript_model::words::chars_no_spaces(p);
        }
        c
    }
}

/// The counts of an item and everything under it.
pub fn item_counts(p: &Project, id: ItemId) -> Counts {
    let mut paras = Vec::new();
    if let Some(item) = p.binder.get(id) {
        for i in item.walk() {
            if let Some(t) = p.texts.get(&i.id) {
                paras.extend(t.body.plain_paragraphs());
            }
        }
    }
    Counts::of(&paras)
}

/// The counts of the included manuscript.
pub fn manuscript_counts(p: &Project) -> Counts {
    let paras: Vec<String> = p.included_manuscript().into_iter().filter_map(|i| p.texts.get(&i)).flat_map(|t| t.body.plain_paragraphs()).collect();
    Counts::of(&paras)
}

/// The writing days as weekday numbers (0 Monday).
pub fn weekdays(p: &Project) -> Vec<u8> {
    p.settings.targets.writing_days.iter().filter_map(|d| DAYS.iter().position(|x| x == d).map(|i| i as u8)).collect()
}

/// The words a day needed to meet the manuscript's target by its deadline.
pub fn daily_need(p: &Project, today: Day) -> Option<usize> {
    let target = p.settings.targets.manuscript? as usize;
    let deadline = Day::parse(&p.settings.targets.deadline)?;
    let left = target.saturating_sub(p.manuscript_words());
    words_per_day(left, today, deadline, &weekdays(p))
}

/// Today in the local zone (UTC when the zone is not known).
pub fn today() -> Day {
    Day::of(SystemTime::now(), 0)
}

/// The writing session of an open project.
pub struct Progress {
    /// The core's writing statistics.
    pub writing: Writing,
    seen: u64,
}

impl Progress {
    /// A session starting now on a project.
    pub fn start(p: &Project) -> Progress {
        let target = p.settings.targets.session.map(|t| t as usize);
        Progress { writing: Writing::new(p.manuscript_words()).with_target(target), seen: u64::MAX }
    }

    /// Note the project's words when it changed.
    pub fn observe(&mut self, s: &Session) {
        if s.revision() == self.seen {
            return;
        }
        self.seen = s.revision();
        self.writing.target = s.project.settings.targets.session.map(|t| t as usize);
        self.writing.observe(s.project.manuscript_words(), today());
    }

    /// The status bar's part: the manuscript's and the session's progress, a running sprint.
    pub fn ui(&self, ui: &mut Ui, p: &Project) {
        let words = p.manuscript_words();
        match p.settings.targets.manuscript.filter(|t| *t > 0) {
            Some(t) => {
                let need = daily_need(p, today()).map(|n| format!(", {} a day to the deadline", crate::binder::format_words(n))).unwrap_or_default();
                progress_bar(
                    ui,
                    &format!("{} of {} words{need}", crate::binder::format_words(words), crate::binder::format_words(t as usize)),
                    words as f32 / t as f32,
                    70.0,
                );
            }
            None => {
                ui.label(format!("Manuscript: {} words", crate::binder::format_words(words)));
            }
        }
        let session = self.writing.session_words();
        match self.writing.target {
            Some(t) if t > 0 => {
                progress_bar(ui, &format!("{session} of {t} this session"), self.writing.progress().unwrap_or(0.0), 50.0);
            }
            _ => {
                ui.label(format!("{session} this session"));
            }
        }
        if let Some(sp) = self.writing.sprint {
            let left = sp.remaining(Instant::now()).as_secs();
            let words = self.writing.sprint_words();
            if left > 0 {
                ui.label(format!("Sprint {}:{:02}, {words} words", left / 60, left % 60));
                ui.ctx().request_repaint_after(std::time::Duration::from_secs(1));
            } else {
                ui.label(format!("Sprint done: {words} words"));
            }
        }
    }
}

/// The progress commands.
pub fn specs() -> Vec<Spec> {
    vec![
        Spec {
            id: "targets.set",
            label: "Set Targets",
            key: "",
            params: r#"{"manuscript"?: n|null, "deadline"?: "YYYY-MM-DD"|"", "writing_days"?: ["mon",...], "session"?: n|null}"#,
            about: "The manuscript's word target, its deadline and writing days, and the session's target.",
            mutates: true,
        },
        Spec {
            id: "counts.get",
            label: "Counts",
            key: "",
            params: r#"{"id"?: id}"#,
            about: "Words and characters of an item and what is under it, or of the included manuscript; with the targets and the words a day the deadline needs.",
            mutates: false,
        },
    ]
}

/// Run a progress command, or `None` if `id` is not one.
pub fn run(s: &mut Session, id: &str, v: &Value) -> Option<Outcome> {
    Some(match id {
        "targets.set" => (|| {
            let mut t = s.project.settings.targets.clone();
            let num = |k: &str| -> Result<Option<Option<u64>>, String> {
                match v.get(k) {
                    None => Ok(None),
                    Some(Value::Null) => Ok(Some(None)),
                    Some(x) => x
                        .as_u64()
                        .filter(|n| *n <= 100_000_000)
                        .map(|n| Some(Some(n).filter(|n| *n > 0)))
                        .ok_or_else(|| format!("`{k}` is a number of words")),
                }
            };
            if let Some(m) = num("manuscript")? {
                t.manuscript = m;
            }
            if let Some(m) = num("session")? {
                t.session = m;
            }
            if let Some(d) = v.get("deadline").and_then(Value::as_str) {
                if !d.is_empty() && Day::parse(d).is_none() {
                    return Err(format!("`{d}` is not a date (YYYY-MM-DD)"));
                }
                t.deadline = d.to_owned();
            }
            if let Some(days) = v.get("writing_days").and_then(Value::as_array) {
                let days: Vec<String> = days.iter().filter_map(Value::as_str).map(str::to_lowercase).collect();
                if let Some(bad) = days.iter().find(|d| !DAYS.contains(&d.as_str())) {
                    return Err(format!("`{bad}` is not a day (mon, tue, wed, thu, fri, sat, sun)"));
                }
                t.writing_days = DAYS.iter().filter(|d| days.iter().any(|x| x == *d)).map(|d| (*d).to_owned()).collect();
            }
            s.change("Targets", &[], &[], |p| {
                p.settings.targets = t;
                Ok(())
            })?;
            Ok(json!({}))
        })(),
        "counts.get" => (|| {
            let item = opt_id(v, "id")?;
            let c = match item {
                Some(i) => {
                    s.project.binder.get(i).ok_or_else(|| format!("no binder item {i}"))?;
                    item_counts(&s.project, i)
                }
                None => manuscript_counts(&s.project),
            };
            let target = match item {
                Some(i) => s.project.text(i).and_then(|t| t.meta.target),
                None => s.project.settings.targets.manuscript,
            };
            Ok(json!({
                "words": c.words,
                "characters": c.chars,
                "characters_no_spaces": c.chars_no_spaces,
                "target": target,
                "progress": target.filter(|t| *t > 0).map(|t| c.words as f64 / t as f64),
                "words_a_day": if item.is_none() { daily_need(&s.project, today()) } else { None },
            }))
        })(),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_of_paragraphs() {
        let c = Counts::of(&["One two", "three\nfour"]);
        assert_eq!(c, Counts { words: 4, chars: 16, chars_no_spaces: 15 });
    }
}
