// SPDX-License-Identifier: MIT OR Apache-2.0
//! Agent tools: every Manuscript command is a tool and nothing else is, a project can be driven
//! through them, and the files their parameters name are held to the folders the agent was given.

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use jcj_engine::file_access::AutomationRoots;
use serde_json::{Value, json};
use wordsmith_manuscript_model::Project;
use wordsmith_manuscript_ui::agents::{self, request};
use wordsmith_manuscript_ui::session::Session;

fn scratch(name: &str) -> PathBuf {
    let d = Path::new(env!("CARGO_TARGET_TMPDIR")).join("agents").join(name);
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn listed(server: &mut jcj_agents::AgentServer) -> Vec<Value> {
    let list = server.handle(request(1, "tools/list", json!({}))).unwrap();
    list["result"]["tools"].as_array().unwrap().clone()
}

/// A tool's answer: `Ok` with its result, `Err` with the reason it gave.
fn try_call(server: &mut jcj_agents::AgentServer, tool: &str, args: Value) -> Result<Value, String> {
    let r = server.handle(request(9, "tools/call", json!({ "name": tool, "arguments": args }))).unwrap();
    let text = r["result"]["content"][0]["text"].as_str().unwrap_or("").to_owned();
    if r["result"]["isError"] == json!(true) { Err(text) } else { Ok(serde_json::from_str(&text).unwrap_or(Value::String(text))) }
}

fn call(server: &mut jcj_agents::AgentServer, tool: &str, args: Value) -> Value {
    try_call(server, tool, args).unwrap_or_else(|e| panic!("{tool}: {e}"))
}

#[test]
fn every_command_is_a_tool_and_nothing_else_is() {
    let mut server = agents::server(Session::new(Project::new()), None);
    let tools = listed(&mut server);
    let names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();
    let commands = wordsmith_manuscript_ui::commands::specs();
    assert_eq!(names.len(), commands.len(), "one tool for each command, and no general document tool");
    for c in &commands {
        let name = format!("manuscript_{}", c.id.replace('.', "_"));
        assert!(names.contains(&name.as_str()), "{name} is listed");
    }
    assert_eq!(names, agents::tool_names());
    assert_eq!(server.tool_names(), agents::tool_names());
    for general in ["execute", "list_commands", "open_document", "save_document"] {
        assert!(!names.contains(&general), "{general} works on a document, and there is none");
        assert!(try_call(&mut server, general, json!({})).unwrap_err().contains("unknown tool"));
    }
}

#[test]
fn a_tool_says_whether_it_only_reads() {
    let mut server = agents::server(Session::new(Project::new()), None);
    let tools = listed(&mut server);
    let read_only =
        |name: &str| tools.iter().find(|t| t["name"] == name).unwrap_or_else(|| panic!("{name}"))["annotations"]["readOnlyHint"].as_bool().unwrap();
    assert!(read_only("manuscript_binder_list") && read_only("manuscript_text_get") && read_only("manuscript_import_preview"));
    assert!(!read_only("manuscript_text_set"));
    // An export leaves the project as it was, but it writes a file.
    assert!(!read_only("manuscript_export_file") && !read_only("manuscript_export_convert"));
}

#[test]
fn every_path_parameter_is_declared() {
    // A parameter that names a file and is not declared is not checked against the agent's
    // folders, so a command that gains one must say so in `agents::files_of`.
    for spec in wordsmith_manuscript_ui::commands::specs() {
        let (reads, writes) = agents::files_of(spec.id);
        for key in ["path", "paths", "folder", "file", "files", "dir", "directory"] {
            let named = spec.params.contains(&format!("\"{key}\":")) || spec.params.contains(&format!("\"{key}\"?:"));
            let declared = reads.contains(&key) || writes.contains(&key);
            assert_eq!(named, declared, "{}: `{key}` in its parameters ({}) and in files_of", spec.id, spec.params);
        }
    }
    assert_eq!(agents::files_of("series.create"), (&[][..], &["path"][..]), "making a series bible writes its folder");
    assert_eq!(agents::files_of("export.convert").1, &["path"]);
    assert_eq!(agents::files_of("import.set").0, &["paths", "folder"]);
}

#[test]
fn a_project_is_driven_through_the_tools() {
    let session = Rc::new(RefCell::new(Session::new(Project::new())));
    let mut server = agents::server_over(session.clone(), None);
    let init = server
        .handle(request(
            1,
            "initialize",
            json!({ "protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": { "name": "test", "version": "0" } }),
        ))
        .unwrap();
    assert!(init["result"]["instructions"].as_str().unwrap().contains("binder"));
    assert_eq!(init["result"]["serverInfo"]["name"], "wordsmith-manuscript");
    let made = call(&mut server, "manuscript_binder_new", json!({ "area": "manuscript", "title": "Opening" }));
    let id = made["id"].as_str().unwrap().to_owned();
    call(&mut server, "manuscript_text_set", json!({ "id": id, "text": "The *first* line.\n" }));
    call(&mut server, "manuscript_item_setStatus", json!({ "id": id, "status": "First draft" }));
    let got = call(&mut server, "manuscript_text_get", json!({ "id": id }));
    assert_eq!(got["words"], 3);
    call(&mut server, "manuscript_bible_new", json!({ "kind": "character", "name": "Mara" }));
    call(&mut server, "manuscript_edit_undo", json!({}));
    let s = session.borrow();
    assert!(s.project.entries.is_empty(), "undo through a tool");
    assert_eq!(s.project.binder.get(id.parse().unwrap()).unwrap().status.as_deref(), Some("First draft"));
}

#[test]
fn the_files_a_tool_names_are_kept_to_the_agents_folders() {
    let dir = scratch("roots");
    for d in ["in", "out", "elsewhere"] {
        std::fs::create_dir_all(dir.join(d)).unwrap();
    }
    std::fs::write(dir.join("in/chapter.md"), "# One\n\nA line to bring in.\n").unwrap();
    std::fs::write(dir.join("elsewhere/secret.md"), "# Two\n\nNot the agent's to read.\n").unwrap();
    // The project lives outside both folders: the person chose it, and it is saved where it is.
    let project = dir.join("elsewhere/Book.wsm");
    let session = Session::create(&project, Project::new()).unwrap();
    let roots = AutomationRoots::new(Some(&dir.join("in")), Some(&dir.join("out"))).unwrap();
    assert!(roots.is_some());
    let mut server = agents::server(session, roots);
    let text = |p: PathBuf| p.to_string_lossy().into_owned();

    // Reads: inside the read folder only.
    call(&mut server, "manuscript_import_document", json!({ "path": text(dir.join("in/chapter.md")) }));
    for (tool, args) in [
        ("manuscript_import_document", json!({ "path": text(dir.join("elsewhere/secret.md")) })),
        ("manuscript_import_preview", json!({ "path": text(dir.join("elsewhere/secret.md")) })),
        ("manuscript_import_set", json!({ "folder": text(dir.join("elsewhere")) })),
        ("manuscript_import_set", json!({ "paths": [text(dir.join("in/chapter.md")), text(dir.join("elsewhere/secret.md"))] })),
        ("manuscript_import_notes", json!({ "path": text(dir.join("elsewhere")) })),
        ("manuscript_media_import", json!({ "path": text(dir.join("elsewhere/secret.md")) })),
        ("manuscript_offers_list", json!({ "path": text(dir.join("elsewhere/offers.json")) })),
        ("manuscript_series_join", json!({ "path": text(dir.join("elsewhere/series")) })),
        // The write folder is not a folder to read from.
        ("manuscript_import_document", json!({ "path": text(dir.join("out/chapter.md")) })),
    ] {
        let why = try_call(&mut server, tool, args.clone()).unwrap_err();
        assert!(why.contains("outside"), "{tool} {args}: {why}");
    }

    // Writes: inside the write folder only.
    let made = call(&mut server, "manuscript_export_file", json!({ "path": text(dir.join("out/book.md")) }));
    assert!(made["check"].as_str().unwrap().contains("every word matches"), "{made}");
    assert!(dir.join("out/book.md").is_file());
    for (tool, args) in [
        ("manuscript_export_file", json!({ "path": text(dir.join("elsewhere/book.md")) })),
        ("manuscript_export_file", json!({ "path": text(dir.join("in/book.md")) })),
        ("manuscript_series_create", json!({ "path": text(dir.join("elsewhere/series")), "name": "Saga" })),
        ("manuscript_series_create", json!({ "path": text(dir.join("in/series")), "name": "Saga" })),
    ] {
        let why = try_call(&mut server, tool, args.clone()).unwrap_err();
        assert!(why.contains("outside"), "{tool} {args}: {why}");
    }
    assert!(!dir.join("elsewhere/book.md").exists() && !dir.join("in/book.md").exists());
    assert!(!dir.join("elsewhere/series").exists() && !dir.join("in/series").exists());
    // A series bible is made where the agent may write, and the book, which is the person's, is
    // saved where it lives.
    call(&mut server, "manuscript_series_create", json!({ "path": text(dir.join("out/series")), "name": "Saga" }));
    assert!(dir.join("out/series/project.toml").is_file());

    // A program the agent names cannot be kept to a folder.
    let why = try_call(
        &mut server,
        "manuscript_export_convert",
        json!({ "path": text(dir.join("out/book.epub")), "program": "anything", "args": "{input} {output}" }),
    )
    .unwrap_err();
    assert!(why.contains("runs a program"), "{why}");

    // The imported chapter was saved in the project, outside both folders.
    drop(server);
    let again = Session::open(&project).unwrap();
    assert!(again.project.manuscript_words() >= 5, "{}", again.project.manuscript_words());
    assert!(!again.project.settings.series.is_empty());
}

#[test]
fn one_folder_grants_only_its_own_kind_of_access() {
    let dir = scratch("one-root");
    std::fs::create_dir_all(dir.join("out")).unwrap();
    std::fs::write(dir.join("out/chapter.md"), "A line.\n").unwrap();
    let roots = AutomationRoots::new(None, Some(&dir.join("out"))).unwrap();
    let mut server = agents::server(Session::new(Project::new()), roots);
    let inside = dir.join("out/chapter.md").to_string_lossy().into_owned();
    assert!(try_call(&mut server, "manuscript_import_document", json!({ "path": inside })).is_err(), "nothing may be read");
    // The project itself still answers: it is not a file the agent named.
    call(&mut server, "manuscript_binder_list", json!({}));
    call(&mut server, "manuscript_binder_new", json!({ "area": "notes", "title": "A note" }));
}
