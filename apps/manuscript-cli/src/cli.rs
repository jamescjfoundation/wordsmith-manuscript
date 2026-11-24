// SPDX-License-Identifier: MIT OR Apache-2.0
//! The command line's commands, writing their results to a writer (so tests can read them).

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use serde_json::{Value, json};
use wordsmith_manuscript_format::book;
use wordsmith_manuscript_io::verify::{self, Check};
use wordsmith_manuscript_model::{Area, Project, Role};
use wordsmith_manuscript_ui::session::Session;

const USAGE: &str = "usage:
  manuscript-cli convert <input> <output>
  manuscript-cli export <project> <file> [--format F] [--preset NAME]
  manuscript-cli counts <project> [--item ID]
  manuscript-cli verify <source> <result>
  manuscript-cli report <project>
  manuscript-cli run <project> <command> [JSON]
  manuscript-cli commands
  manuscript-cli mcp <project>";

fn flag(args: &[String], name: &str) -> Option<String> {
    args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).cloned()
}

fn positional(args: &[String]) -> Vec<&String> {
    let mut out = Vec::new();
    let mut skip = false;
    for a in args.iter().skip(1) {
        if skip {
            skip = false;
            continue;
        }
        if a.starts_with("--") {
            skip = true;
            continue;
        }
        out.push(a);
    }
    out
}

fn is_project(p: &Path) -> bool {
    p.join("project.toml").exists() || p.file_name().is_some_and(|n| n == "project.toml") || p.extension().is_some_and(|e| e == "wsmz")
}

/// Open a project folder, its `project.toml`, or a packed `.wsmz` (read in memory).
fn open(path: &Path) -> Result<Session, String> {
    if path.extension().is_some_and(|e| e == "wsmz") {
        let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let p = wordsmith_manuscript_format::pack::read_packed(&bytes).map_err(|e| e.to_string())?;
        return Ok(Session::new(p));
    }
    Session::open(path)
}

fn print(out: &mut dyn Write, v: &Value) -> Result<(), String> {
    writeln!(out, "{}", serde_json::to_string_pretty(v).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}

/// The words of a file or project, as the check reads them: a project's manuscript (part and
/// chapter titles, then text), or a document's main text.
pub fn words_of(path: &Path) -> Result<Vec<String>, String> {
    if is_project(path) {
        let s = open(path)?;
        return Ok(manuscript_words(&s.project));
    }
    let doc = wordsmith_manuscript_io::import::document(path, &Default::default())?;
    Ok(verify::document_words(&doc))
}

/// A project's manuscript words in reading order.
pub fn manuscript_words(p: &Project) -> Vec<String> {
    let mut out = Vec::new();
    for id in p.included_manuscript() {
        let Some(item) = p.binder.get(id) else { continue };
        if matches!(item.role, Role::Part | Role::Chapter) {
            out.extend(verify::words(&item.title));
        }
        if let Some(t) = p.texts.get(&id) {
            let body = wordsmith_manuscript_model::Text { blocks: t.body.blocks.clone(), notes: vec![] };
            out.extend(body.plain_paragraphs().iter().flat_map(|x| verify::words(x)));
        }
    }
    out
}

/// Run the command line.
pub fn run(args: &[String], out: &mut dyn Write) -> Result<ExitCode, String> {
    let pos = positional(args);
    let arg = |n: usize| pos.get(n).map(|s| PathBuf::from(s.as_str())).ok_or_else(|| USAGE.to_owned());
    match args.first().map(String::as_str) {
        Some("convert") => {
            let (input, output) = (arg(0)?, arg(1)?);
            if is_project(&input) {
                let mut s = open(&input)?;
                let r = s.run("export.file", &json!({ "path": output.to_string_lossy() }))?;
                print(out, &r)?;
                return Ok(ExitCode::SUCCESS);
            }
            if output.exists() {
                return Err(format!("{} already exists", output.display()));
            }
            let title = input.file_stem().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            let mut p = Project::new();
            p.settings.book.title = title;
            let mut s = Session::new(p);
            let r = if input.is_dir() {
                s.run("import.set", &json!({ "folder": input.to_string_lossy() }))?
            } else {
                s.run("import.document", &json!({ "path": input.to_string_lossy() }))?
            };
            if output.extension().is_some_and(|e| e == "wsmz") {
                let bytes = wordsmith_manuscript_format::pack::pack_project(&s.project).map_err(|e| e.to_string())?;
                std::fs::write(&output, bytes).map_err(|e| format!("{}: {e}", output.display()))?;
            } else {
                s.save_as(&output)?;
            }
            print(out, &r)?;
            Ok(ExitCode::SUCCESS)
        }
        Some("export") => {
            let (project, file) = (arg(0)?, arg(1)?);
            let mut s = open(&project)?;
            let mut params = json!({ "path": file.to_string_lossy() });
            if let Some(f) = flag(args, "--format") {
                params["format"] = json!(f);
            }
            if let Some(p) = flag(args, "--preset") {
                params["preset"] = json!(p);
            }
            let r = s.run("export.file", &params)?;
            print(out, &r)?;
            Ok(ExitCode::SUCCESS)
        }
        Some("counts") => {
            let mut s = open(&arg(0)?)?;
            let mut params = json!({});
            if let Some(i) = flag(args, "--item") {
                params["id"] = json!(i);
            }
            let r = s.run("counts.get", &params)?;
            print(out, &r)?;
            Ok(ExitCode::SUCCESS)
        }
        Some("verify") => {
            let (a, b) = (arg(0)?, arg(1)?);
            let check = Check::of(&words_of(&a)?, &words_of(&b)?);
            print(out, &json!({ "same": check.passed(), "check": check.to_string() }))?;
            Ok(if check.passed() { ExitCode::SUCCESS } else { ExitCode::from(2) })
        }
        Some("report") => {
            let s = open(&arg(0)?)?;
            let p = &s.project;
            let b = book::from_project(p);
            let mut labels = std::collections::BTreeMap::<String, usize>::new();
            let mut statuses = std::collections::BTreeMap::<String, usize>::new();
            for r in p.binder.rows_of(Area::Manuscript) {
                if let Some(l) = &r.item.label {
                    *labels.entry(l.clone()).or_default() += 1;
                }
                if let Some(st) = &r.item.status {
                    *statuses.entry(st.clone()).or_default() += 1;
                }
            }
            print(
                out,
                &json!({
                    "title": p.settings.book.title,
                    "byline": p.settings.book.byline(),
                    "words": p.manuscript_words(),
                    "target": p.settings.targets.manuscript,
                    "deadline": p.settings.targets.deadline,
                    "sections": book::counts(&b),
                    "labels": labels,
                    "statuses": statuses,
                    "bible_entries": p.entries.len(),
                    "items": p.binder.len(),
                    "in_trash": p.binder.rows_of(Area::Trash).len(),
                    "missing_book_metadata": book::missing_metadata(&b),
                }),
            )?;
            Ok(ExitCode::SUCCESS)
        }
        Some("run") => {
            let project = arg(0)?;
            let cmd = pos.get(1).ok_or_else(|| USAGE.to_owned())?.to_string();
            let params: Value = match pos.get(2) {
                Some(j) => serde_json::from_str(j).map_err(|e| format!("parameters: {e}"))?,
                None => json!({}),
            };
            let mut s = open(&project)?;
            let r = s.run(&cmd, &params)?;
            if s.is_dirty() && s.root().is_some() {
                s.save()?;
            }
            print(out, &r)?;
            Ok(ExitCode::SUCCESS)
        }
        Some("commands") => {
            let list: Vec<Value> = wordsmith_manuscript_ui::commands::specs()
                .iter()
                .map(|c| json!({ "id": c.id, "label": c.label, "params": c.params, "about": c.about }))
                .collect();
            print(out, &Value::Array(list))?;
            Ok(ExitCode::SUCCESS)
        }
        Some("mcp") => {
            let s = open(&arg(0)?)?;
            wordsmith_manuscript_ui::agents::serve(s)?;
            Ok(ExitCode::SUCCESS)
        }
        Some("help" | "--help" | "-h") | None => {
            writeln!(out, "{USAGE}").map_err(|e| e.to_string())?;
            Ok(ExitCode::SUCCESS)
        }
        Some("--version" | "version") => {
            writeln!(out, "manuscript-cli {}", env!("CARGO_PKG_VERSION")).map_err(|e| e.to_string())?;
            Ok(ExitCode::SUCCESS)
        }
        Some(other) => Err(format!("unknown command `{other}`\n{USAGE}")),
    }
}
