// SPDX-License-Identifier: MIT OR Apache-2.0
//! The command line, held to the core's convention for command lines and agent tools
//! (`docs/command-lines-and-agent-tools.md` in open-source-core): the built program is run and
//! its exit code, its standard output and its standard error are read.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use jcj_agents::cli::Exit;
use serde_json::{Value, json};

fn run(args: &[&str], input: &str) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_manuscript-cli"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(input.as_bytes()).unwrap();
    child.wait_with_output().unwrap()
}

fn code(o: &Output) -> Option<i32> {
    o.status.code()
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn exit(e: Exit) -> Option<i32> {
    Some(i32::from(e.code()))
}

fn scratch(name: &str) -> PathBuf {
    let d = Path::new(env!("CARGO_TARGET_TMPDIR")).join("command-line").join(name);
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn path(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}

/// A folder with a small project in it, made by the program itself.
fn project(name: &str) -> (PathBuf, String) {
    let dir = scratch(name);
    let source = dir.join("source.md");
    std::fs::write(&source, "# One\n\nIt was a bright cold day.\n").unwrap();
    let project = path(&dir.join("Book.wsm"));
    let o = run(&["convert", &path(&source), &project], "");
    assert_eq!(code(&o), exit(Exit::Done), "{}", text(&o.stderr));
    (dir, project)
}

fn request(id: u64, method: &str, params: Value) -> String {
    json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}).to_string() + "\n"
}

fn call(id: u64, tool: &str, arguments: Value) -> String {
    request(id, "tools/call", json!({"name": tool, "arguments": arguments}))
}

fn replies(o: &Output) -> Vec<Value> {
    text(&o.stdout).lines().map(|l| serde_json::from_str(l).unwrap()).collect()
}

#[test]
fn help_and_version_are_answered_on_standard_output() {
    for help in ["--help", "-h", "help"] {
        let o = run(&[help], "");
        assert_eq!(code(&o), exit(Exit::Done), "{help}");
        assert!(text(&o.stdout).starts_with("usage:") && o.stderr.is_empty(), "{help}");
        assert!(text(&o.stdout).contains("mcp <project> [--read-root <folder>] [--write-root <folder>]"));
    }
    for version in ["--version", "-V", "version"] {
        let o = run(&[version], "");
        assert_eq!(code(&o), exit(Exit::Done), "{version}");
        assert_eq!(text(&o.stdout).lines().next(), Some(concat!("manuscript-cli ", env!("CARGO_PKG_VERSION"))));
        assert!(o.stderr.is_empty());
    }
}

#[test]
fn no_argument_is_a_usage_error() {
    let o = run(&[], "");
    assert_eq!(code(&o), exit(Exit::Usage));
    assert!(o.stdout.is_empty(), "nothing was asked, so nothing is on standard output");
    assert!(text(&o.stderr).contains("usage:"), "{}", text(&o.stderr));
}

#[test]
fn a_wrong_command_line_is_a_usage_error_on_standard_error() {
    let (dir, project) = project("usage");
    let out = path(&dir.join("out.md"));
    let bad: &[&[&str]] = &[
        // An unknown command or option.
        &["nonsense"],
        &["--bogus"],
        &["-v"],
        &["counts", &project, "--bogus", "value"],
        &["export", &project, &out, "--fromat", "markdown"],
        &["verify", &project, &project, "--strict"],
        &["commands", "--all"],
        &["run", "--bogus", "binder.list"],
        // A missing argument.
        &["convert"],
        &["convert", &project],
        &["export", &project],
        &["counts"],
        &["verify", &project],
        &["report"],
        &["run", &project],
        &["mcp"],
        &["mcp", "--read-root", &path(&dir)],
        // An extra one.
        &["counts", &project, "extra"],
        &["report", &project, "extra"],
        &["commands", "extra"],
        &["run", &project, "binder.list", "{}", "extra"],
        &["mcp", &project, "second"],
        // An option without its value, or given twice.
        &["counts", &project, "--item"],
        &["export", &project, &out, "--format"],
        &["export", &project, &out, "--format", "text", "--format", "html"],
        &["mcp", &project, "--read-root"],
        &["mcp", &project, "--bogus"],
        // A folder for the agent that does not exist.
        &["mcp", &project, "--write-root", &path(&dir.join("no-such-folder"))],
        // Parameters that are not JSON, or not an object.
        &["run", &project, "binder.list", "{not json"],
        &["run", &project, "binder.list", "[1, 2]"],
    ];
    for args in bad {
        let o = run(args, "");
        assert_eq!(code(&o), exit(Exit::Usage), "{args:?}: {}", text(&o.stderr));
        assert!(o.stdout.is_empty(), "{args:?}: nothing on standard output");
        let said = text(&o.stderr);
        assert!(said.starts_with("manuscript-cli: ") && said.contains("usage:"), "{args:?}: {said}");
    }
    // Nothing was done: the misspelled option wrote no file.
    assert!(!dir.join("out.md").exists());
}

#[test]
fn a_failure_is_not_a_usage_error() {
    let (dir, project) = project("failure");
    let missing = path(&dir.join("missing.wsm"));
    let bad: &[&[&str]] = &[
        &["counts", &missing],
        &["report", &missing],
        &["mcp", &missing],
        &["export", &project, &path(&dir.join("book.xyz"))],
        &["run", &project, "no.such.command"],
        &["run", &project, "binder.rename", "{}"],
        &["convert", &path(&dir.join("source.md")), &project],
    ];
    for args in bad {
        let o = run(args, "");
        assert_eq!(code(&o), exit(Exit::Failed), "{args:?}: {}", text(&o.stderr));
        assert!(o.stdout.is_empty(), "{args:?}: nothing on standard output");
        let said = text(&o.stderr);
        assert!(said.starts_with("manuscript-cli: ") && !said.contains("usage:"), "{args:?}: {said}");
    }
}

#[test]
fn a_difference_is_not_a_failure() {
    let (dir, project) = project("verify");
    let file = |name: &str, words: &str| {
        let p = dir.join(name);
        std::fs::write(&p, words).unwrap();
        path(&p)
    };
    let (same, other) = (file("same.txt", "One\n\nIt was a bright cold day.\n"), file("other.txt", "One\n\nIt was a bright warm day.\n"));
    let o = run(&["verify", &project, &same], "");
    assert_eq!(code(&o), exit(Exit::Done), "{}", text(&o.stdout));
    let said: Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(said["same"], true);
    assert!(said["check"].as_str().unwrap().contains("every word matches (7 words)"), "{said}");
    // The check ran and said no: 3, with the answer on standard output.
    let o = run(&["verify", &project, &other], "");
    assert_eq!(code(&o), exit(Exit::Differs));
    assert!(o.stderr.is_empty(), "{}", text(&o.stderr));
    let said: Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(said["same"], false);
    assert!(said["check"].as_str().unwrap().contains("word 6 differs"), "{said}");
    // The check could not run: 1, with the reason on standard error.
    let o = run(&["verify", &project, &path(&dir.join("missing.txt"))], "");
    assert_eq!(code(&o), exit(Exit::Failed));
    assert!(o.stdout.is_empty() && text(&o.stderr).starts_with("manuscript-cli: "), "{}", text(&o.stderr));
}

#[test]
fn mcp_serves_the_projects_tools_until_its_input_ends() {
    let (_dir, project) = project("mcp");
    let o = run(&["mcp", &project], &(request(1, "initialize", json!({})) + &request(2, "tools/list", json!({}))));
    assert_eq!(code(&o), exit(Exit::Done), "{}", text(&o.stderr));
    // Unconfined: said on standard error; standard output carries only the protocol.
    assert!(text(&o.stderr).contains("no --read-root or --write-root given"), "{}", text(&o.stderr));
    let replies = replies(&o);
    assert_eq!(replies.len(), 2, "one line of JSON for each request, nothing else");
    assert_eq!(replies[0]["result"]["serverInfo"]["name"], "wordsmith-manuscript");
    let names: Vec<&str> = replies[1]["result"]["tools"].as_array().unwrap().iter().filter_map(|t| t["name"].as_str()).collect();
    assert!(names.contains(&"manuscript_binder_list") && names.contains(&"manuscript_export_file"), "{names:?}");
    // Every tool is the app's own: there is no document for the general tools to work on.
    assert!(names.iter().all(|n| n.starts_with("manuscript_")), "{names:?}");
}

#[test]
fn mcp_keeps_an_agent_to_the_folders_given_and_saves_the_project_where_it_lives() {
    let (dir, project) = project("mcp-roots");
    for d in ["in", "out"] {
        std::fs::create_dir_all(dir.join(d)).unwrap();
    }
    std::fs::write(dir.join("in/two.md"), "# Two\n\nA second chapter arrives.\n").unwrap();
    let outside = dir.join("outside.md");
    std::fs::write(&outside, "Not the agent's to read.\n").unwrap();
    let import = |id: u64, p: &Path| call(id, "manuscript_import_document", json!({"path": path(p)}));
    let export = |id: u64, p: &Path| call(id, "manuscript_export_file", json!({"path": path(p)}));
    // The project is in neither folder: the person named it, so it is opened and saved all the same.
    let input = import(1, &dir.join("in/two.md"))
        + &import(2, &outside)
        + &export(3, &dir.join("out/book.md"))
        + &export(4, &dir.join("book.md"))
        + &export(5, &dir.join("in/book.md"))
        + &call(6, "manuscript_series_create", json!({"path": path(&dir.join("series")), "name": "Saga"}))
        + &call(7, "manuscript_export_convert", json!({"path": path(&dir.join("out/book.epub")), "program": "anything", "args": "{output}"}));
    let o = run(&["mcp", &project, "--read-root", &path(&dir.join("in")), "--write-root", &path(&dir.join("out"))], &input);
    assert_eq!(code(&o), exit(Exit::Done), "{}", text(&o.stderr));
    assert!(!text(&o.stderr).contains("any file"), "{}", text(&o.stderr));
    let errors: Vec<Option<bool>> = replies(&o).iter().map(|r| r["result"]["isError"].as_bool()).collect();
    assert_eq!(errors, [false, true, false, true, true, true, true].map(Some), "{}", text(&o.stdout));
    assert!(dir.join("out/book.md").is_file());
    assert!(!dir.join("book.md").exists() && !dir.join("in/book.md").exists() && !dir.join("series").exists());
    // The import was saved in the project.
    let o = run(&["counts", &project], "");
    let counts: Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(counts["words"], 10, "six words converted and four imported: {counts}");

    // One flag grants only its kind of access: with a folder to write in, nothing may be read.
    let o = run(&["mcp", &project, "--write-root", &path(&dir.join("out"))], &import(1, &dir.join("out/book.md")));
    assert_eq!(code(&o), exit(Exit::Done));
    assert_eq!(replies(&o)[0]["result"]["isError"], true);
}
