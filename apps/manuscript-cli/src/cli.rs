// SPDX-License-Identifier: MIT OR Apache-2.0
//! The command line's commands, writing their results to a writer (so tests can read them).

use std::io::Write;
use std::ops::RangeInclusive;
use std::path::{Path, PathBuf};

use jcj_agents::cli::{Exit, McpArgs, Standard, standard, version_line};
use serde_json::{Value, json};
use wordsmith_manuscript_format::book;
use wordsmith_manuscript_io::verify::{self, Check};
use wordsmith_manuscript_model::{Area, Project, Role};
use wordsmith_manuscript_ui::session::Session;

/// The program's name, as the usage, the version line and every message give it.
pub const PROGRAM: &str = "wordsmith-manuscript-cli";

/// The usage: on standard output for `--help`, on standard error after a wrong command line.
pub const USAGE: &str = "usage:
  wordsmith-manuscript-cli convert <input> <output>
  wordsmith-manuscript-cli export <project> <file> [--format F] [--preset NAME]
  wordsmith-manuscript-cli counts <project> [--item ID]
  wordsmith-manuscript-cli verify <source> <result>
  wordsmith-manuscript-cli report <project>
  wordsmith-manuscript-cli run <project> <command> [JSON]
  wordsmith-manuscript-cli commands
  wordsmith-manuscript-cli mcp <project> [--read-root <folder>] [--write-root <folder>]
  wordsmith-manuscript-cli --help | --version";

/// Why the command line stopped without doing what was asked: the exit code, and the reason
/// for standard error.
#[derive(Debug, PartialEq, Eq)]
pub struct Stop {
    /// [`Exit::Usage`] for a wrong command line, [`Exit::Failed`] for work that could not be done.
    pub exit: Exit,
    /// The reason, without the program's name.
    pub why: String,
}

/// A wrong command line: nothing was done.
fn usage(why: impl Into<String>) -> Stop {
    Stop { exit: Exit::Usage, why: why.into() }
}

/// An error from the work itself is a failure.
impl From<String> for Stop {
    fn from(why: String) -> Stop {
        Stop { exit: Exit::Failed, why }
    }
}

/// The arguments after a command's name: the plain ones in order, and its options.
struct Arguments<'a> {
    plain: Vec<&'a str>,
    options: Vec<(&'a str, &'a str)>,
}

impl<'a> Arguments<'a> {
    /// Read `rest`, the arguments after `command`. `takes` is how many plain arguments the
    /// command needs and allows, `shape` names them for the message, and `allowed` are its
    /// options, each of which takes a value. Anything else is a usage error: an argument
    /// starting with `-` that is no option of the command, an option without its value or
    /// given twice, a missing or an extra argument.
    fn read(command: &str, rest: &'a [String], takes: RangeInclusive<usize>, shape: &str, allowed: &[&str]) -> Result<Self, Stop> {
        let mut out = Arguments { plain: Vec::new(), options: Vec::new() };
        let mut rest = rest.iter().map(String::as_str);
        while let Some(arg) = rest.next() {
            if !arg.starts_with('-') || arg.len() == 1 {
                out.plain.push(arg);
                continue;
            }
            if !allowed.contains(&arg) {
                return Err(usage(format!("{command} has no option {arg}")));
            }
            if out.option(arg).is_some() {
                return Err(usage(format!("{arg} was given twice")));
            }
            let value = rest.next().filter(|v| !v.starts_with("--")).ok_or_else(|| usage(format!("{arg} needs a value")))?;
            out.options.push((arg, value));
        }
        if !takes.contains(&out.plain.len()) {
            let shape = if shape.is_empty() { "no arguments".to_owned() } else { shape.to_owned() };
            return Err(usage(format!("{command} takes {shape}")));
        }
        Ok(out)
    }

    fn option(&self, name: &str) -> Option<&'a str> {
        self.options.iter().find(|(n, _)| *n == name).map(|(_, v)| *v)
    }

    /// The plain argument at `n`, as a path. [`Arguments::read`] has checked that it is there.
    fn path(&self, n: usize) -> PathBuf {
        PathBuf::from(self.plain.get(n).copied().unwrap_or_default())
    }
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

/// Run the command line: `args` are the arguments after the program's name, and results are
/// written to `out`. `Ok` is how it ended when it did what was asked ([`Exit::Done`], or
/// [`Exit::Differs`] for a word check that found a difference); `Err` says why it did not.
pub fn run(args: &[String], out: &mut dyn Write) -> Result<Exit, Stop> {
    match standard(args) {
        Some(Standard::Help) => {
            writeln!(out, "{USAGE}").map_err(|e| e.to_string())?;
            return Ok(Exit::Done);
        }
        Some(Standard::Version) => {
            writeln!(out, "{}", version_line(PROGRAM, env!("CARGO_PKG_VERSION"))).map_err(|e| e.to_string())?;
            return Ok(Exit::Done);
        }
        // Nothing was asked, so nothing was done: a caller that lost its arguments must not
        // read success.
        Some(Standard::NoCommand) => return Err(usage("no command was given")),
        None => {}
    }
    let Some((command, rest)) = args.split_first() else { return Err(usage("no command was given")) };
    let command = command.as_str();
    match command {
        "convert" => {
            let a = Arguments::read(command, rest, 2..=2, "<input> <output>", &[])?;
            let (input, output) = (a.path(0), a.path(1));
            if is_project(&input) {
                let mut s = open(&input)?;
                let r = s.run("export.file", &json!({ "path": output.to_string_lossy() }))?;
                print(out, &r)?;
                return Ok(Exit::Done);
            }
            if output.exists() {
                return Err(format!("{} already exists", output.display()).into());
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
            Ok(Exit::Done)
        }
        "export" => {
            let a = Arguments::read(command, rest, 2..=2, "<project> <file>", &["--format", "--preset"])?;
            let mut s = open(&a.path(0))?;
            let mut params = json!({ "path": a.path(1).to_string_lossy() });
            if let Some(f) = a.option("--format") {
                params["format"] = json!(f);
            }
            if let Some(p) = a.option("--preset") {
                params["preset"] = json!(p);
            }
            let r = s.run("export.file", &params)?;
            print(out, &r)?;
            Ok(Exit::Done)
        }
        "counts" => {
            let a = Arguments::read(command, rest, 1..=1, "<project>", &["--item"])?;
            let mut s = open(&a.path(0))?;
            let mut params = json!({});
            if let Some(i) = a.option("--item") {
                params["id"] = json!(i);
            }
            let r = s.run("counts.get", &params)?;
            print(out, &r)?;
            Ok(Exit::Done)
        }
        "verify" => {
            let a = Arguments::read(command, rest, 2..=2, "<source> <result>", &[])?;
            let check = Check::of(&words_of(&a.path(0))?, &words_of(&a.path(1))?);
            // The check ran: its answer is a result, on standard output, whichever it is.
            print(out, &json!({ "same": check.passed(), "check": check.to_string() }))?;
            Ok(if check.passed() { Exit::Done } else { Exit::Differs })
        }
        "report" => {
            let a = Arguments::read(command, rest, 1..=1, "<project>", &[])?;
            let s = open(&a.path(0))?;
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
            Ok(Exit::Done)
        }
        "run" => {
            // The parameters are JSON, which may hold anything: only the first two arguments
            // are read as arguments, and `run` has no options.
            let (project, id, json_text) = match rest {
                [project, id, text] => (project.as_str(), id.as_str(), Some(text.as_str())),
                [project, id] => (project.as_str(), id.as_str(), None),
                _ => return Err(usage("run takes <project> <command> [JSON]")),
            };
            if let Some(option) = [project, id].into_iter().find(|a| a.starts_with('-') && a.len() > 1) {
                return Err(usage(format!("run has no option {option}")));
            }
            let params: Value = match json_text {
                Some(text) => serde_json::from_str(text).map_err(|e| usage(format!("the parameters are not JSON: {e}")))?,
                None => json!({}),
            };
            if !params.is_object() {
                return Err(usage("the parameters must be a JSON object, such as {\"id\": \"…\"}"));
            }
            let mut s = open(Path::new(project))?;
            let r = s.run(id, &params)?;
            if s.is_dirty() && s.root().is_some() {
                s.save()?;
            }
            print(out, &r)?;
            Ok(Exit::Done)
        }
        "commands" => {
            Arguments::read(command, rest, 0..=0, "", &[])?;
            let list: Vec<Value> = wordsmith_manuscript_ui::commands::specs()
                .iter()
                .map(|c| json!({ "id": c.id, "label": c.label, "params": c.params, "about": c.about }))
                .collect();
            print(out, &Value::Array(list))?;
            Ok(Exit::Done)
        }
        "mcp" => {
            let mcp = McpArgs::parse(rest).map_err(usage)?;
            // Every tool works on a project, so one must be named.
            let Some(project) = mcp.file.as_deref() else { return Err(usage("mcp needs a project")) };
            // A folder that does not exist is a wrong command line: said before anything is opened.
            mcp.roots().map_err(usage)?;
            // The project is the person's choice, not the agent's: it is opened first, and the
            // folders hold for what the agent names from here on.
            let s = open(Path::new(project))?;
            let roots = mcp.confine(PROGRAM).map_err(usage)?;
            wordsmith_manuscript_ui::agents::serve(s, roots)?;
            Ok(Exit::Done)
        }
        // The version by its older name.
        "version" => {
            writeln!(out, "{}", version_line(PROGRAM, env!("CARGO_PKG_VERSION"))).map_err(|e| e.to_string())?;
            Ok(Exit::Done)
        }
        option if option.starts_with('-') => Err(usage(format!("unknown option {option}"))),
        other => Err(usage(format!("unknown command `{other}`"))),
    }
}
