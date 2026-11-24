// SPDX-License-Identifier: MIT OR Apache-2.0
//! Revision passes (PRG-4): while a pass is on, what the writer types is marked with the pass,
//! and shown in its color, so each round of revision stays visible until the writer clears it.
//!
//! The passes are the core's revision sets (`jcj-features` revisions: names and colors from its
//! color cycle), kept for the whole project in its settings (`[extra] revisions`); text carries
//! the pass as a mark (`{rev=N}` in the text files). The core's typing rules decide what is
//! marked: in revision mode typed and pasted text takes the current pass, out of it text typed
//! next to revised text is not revised. Deleting is not marked (nothing is left to mark).

use jcj_features::revisions::{MAX_NAME_CHARS, Revisions};
use serde_json::{Value, json};
use wordsmith_manuscript_model::{Inline, ItemId, Project, Text};

use crate::commands::{Spec, opt_str};
use crate::session::{Outcome, Session};

/// The settings key.
pub const KEY: &str = "revisions";

/// The project's revision passes.
pub fn of(p: &Project) -> Revisions {
    let mut doc = jcj_doc::new_document();
    if let Some(v) = p.settings.extra.get(KEY)
        && let Ok(r) = serde_json::from_value::<Revisions>(v.clone())
    {
        let _ = r.store(&mut doc);
    }
    // Read back through the core, which brings bad values within limits.
    Revisions::of_doc(&doc)
}

/// Put the project's passes on an editor's document, so typing follows them.
pub fn apply(p: &Project, doc: &mut jcj_doc::Document) {
    let _ = of(p).store(doc);
}

fn save(s: &mut Session, label: &str, r: &Revisions, texts: &[ItemId], f: impl FnOnce(&mut Project)) -> Outcome {
    let v = serde_json::to_value(r).map_err(|e| e.to_string())?;
    s.change(label, texts, &[], |p| {
        p.settings.extra.insert(KEY.into(), v);
        f(p);
        Ok(())
    })?;
    Ok(json!({}))
}

fn marks(t: &Text) -> std::collections::BTreeSet<u32> {
    t.inline_runs()
        .into_iter()
        .flatten()
        .filter_map(|i| match i {
            Inline::Span { marks, .. } | Inline::NoteRef { marks, .. } => marks.rev,
        })
        .collect()
}

fn clear(t: &mut Text, pass: Option<u32>) -> bool {
    let mut changed = false;
    for run in t.inline_runs_mut() {
        for i in run.iter_mut() {
            let m = match i {
                Inline::Span { marks, .. } | Inline::NoteRef { marks, .. } => marks,
            };
            if m.rev.is_some() && (pass.is_none() || m.rev == pass) {
                m.rev = None;
                changed = true;
            }
        }
    }
    if changed {
        t.normalize();
    }
    changed
}

/// The revision commands.
pub fn specs() -> Vec<Spec> {
    let spec = |id, label, key, params, about, mutates| Spec { id, label, key, params, about, mutates };
    vec![
        spec(
            "revision.start",
            "Start Revision Pass",
            "",
            r#"{"name"?: string}"#,
            "Start the next revision pass (the next color) and turn revision mode on: what is typed from now is marked with it.",
            true,
        ),
        spec(
            "revision.mode",
            "Revision Mode",
            "Mod+Shift+E",
            r#"{"on"?: bool}"#,
            "Turn revision mode on or off (switches when `on` is left out).",
            true,
        ),
        spec(
            "revision.list",
            "List Revision Passes",
            "",
            "{}",
            "The passes: number, name, color, whether current, and how many items carry each.",
            false,
        ),
        spec(
            "revision.clear",
            "Clear Revision Marks",
            "",
            r#"{"pass"?: n}"#,
            "Take one pass's marks (or every pass's) off all the text; the words stay. One undo step.",
            true,
        ),
    ]
}

/// Run a revision command, or `None` if `id` is not one.
pub fn run(s: &mut Session, id: &str, v: &Value) -> Option<Outcome> {
    Some(match id {
        "revision.start" => (|| {
            let mut r = of(&s.project);
            let n = r.start_next().ok_or("the project has as many revision passes as it can hold")?;
            if let Some(name) = opt_str(v, "name").map(str::trim).filter(|n| !n.is_empty()) {
                let name: String = name.chars().take(MAX_NAME_CHARS).collect();
                if r.sets.iter().any(|x| x.name == name) {
                    return Err(format!("there is already a pass named `{name}`"));
                }
                if let Some(set) = r.sets.get_mut(n as usize) {
                    set.name.clone_from(&name);
                }
                r.current = name;
            }
            r.mode = true;
            let name = r.current.clone();
            save(s, "Start Revision Pass", &r, &[], |_| {})?;
            Ok(json!({ "pass": n, "name": name }))
        })(),
        "revision.mode" => (|| {
            let mut r = of(&s.project);
            let on = v.get("on").and_then(Value::as_bool).unwrap_or(!r.mode);
            if on && r.current_index().is_none() {
                return Err("start a revision pass first".into());
            }
            r.mode = on;
            save(s, if on { "Revision Mode On" } else { "Revision Mode Off" }, &r, &[], |_| {})?;
            Ok(json!({ "on": on }))
        })(),
        "revision.list" => {
            let r = of(&s.project);
            let used: Vec<std::collections::BTreeSet<u32>> = s.project.texts.values().map(|t| marks(&t.body)).collect();
            Ok(json!({
                "mode": r.mode,
                "passes": r.sets.iter().enumerate().map(|(n, set)| json!({
                    "pass": n,
                    "name": set.name,
                    "color": set.color,
                    "current": r.current == set.name,
                    "items": used.iter().filter(|u| u.contains(&(n as u32))).count(),
                })).collect::<Vec<_>>(),
            }))
        }
        "revision.clear" => (|| {
            let pass = v.get("pass").and_then(Value::as_u64).map(|n| u32::try_from(n).map_err(|_| "no such pass".to_owned())).transpose()?;
            let r = of(&s.project);
            if let Some(n) = pass
                && n as usize >= r.sets.len()
            {
                return Err(format!("there is no pass {n}"));
            }
            let touched: Vec<ItemId> =
                s.project.texts.iter().filter(|(_, t)| marks(&t.body).iter().any(|m| pass.is_none_or(|p| p == *m))).map(|(i, _)| *i).collect();
            save(s, "Clear Revision Marks", &r, &touched, |p| {
                for i in &touched {
                    if let Some(t) = p.texts.get_mut(i) {
                        clear(&mut t.body, pass);
                    }
                }
            })?;
            Ok(json!({ "items": touched.len() }))
        })(),
        _ => return None,
    })
}
