// SPDX-License-Identifier: MIT OR Apache-2.0
//! Agent tools: every Manuscript command is a tool, and a project can be driven through them.

use serde_json::{Value, json};
use wordsmith_manuscript_model::Project;
use wordsmith_manuscript_ui::agents::{self, TOOLS, request};
use wordsmith_manuscript_ui::session::Session;

#[test]
fn every_command_is_a_tool() {
    let mut ids: Vec<&str> = wordsmith_manuscript_ui::commands::specs().iter().map(|s| s.id).collect();
    ids.sort_unstable();
    let mut tools = TOOLS.to_vec();
    tools.sort_unstable();
    assert_eq!(tools, ids, "add new commands to the tools! list in agents.rs");
}

fn call(server: &mut jcj_agents::AgentServer, n: u64, tool: &str, args: Value) -> Value {
    let r = server.handle(request(n, "tools/call", json!({ "name": tool, "arguments": args }))).unwrap();
    let text = r["result"]["content"][0]["text"].as_str().unwrap_or("").to_owned();
    assert_ne!(r["result"]["isError"], json!(true), "{tool}: {text}");
    serde_json::from_str(&text).unwrap_or(Value::String(text))
}

#[test]
fn a_project_is_driven_through_the_tools() {
    let mut server = agents::server(Session::new(Project::new()), None);
    let init = server
        .handle(request(
            1,
            "initialize",
            json!({ "protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": { "name": "test", "version": "0" } }),
        ))
        .unwrap();
    assert!(init["result"]["instructions"].as_str().unwrap().contains("binder"));
    let list = server.handle(request(2, "tools/list", json!({}))).unwrap();
    let names: Vec<&str> = list["result"]["tools"].as_array().unwrap().iter().filter_map(|t| t["name"].as_str()).collect();
    for t in ["manuscript_binder_new", "manuscript_text_set", "manuscript_export_file", "manuscript_bible_new"] {
        assert!(names.contains(&t), "{t} is listed");
    }
    let made = call(&mut server, 3, "manuscript_binder_new", json!({ "area": "manuscript", "title": "Opening" }));
    let id = made["id"].as_str().unwrap().to_owned();
    call(&mut server, 4, "manuscript_text_set", json!({ "id": id, "text": "The *first* line.\n" }));
    call(&mut server, 5, "manuscript_item_setStatus", json!({ "id": id, "status": "First draft" }));
    let got = call(&mut server, 6, "manuscript_text_get", json!({ "id": id }));
    assert_eq!(got["words"], 3);
    call(&mut server, 7, "manuscript_bible_new", json!({ "kind": "character", "name": "Mara" }));
    call(&mut server, 8, "manuscript_edit_undo", json!({}));
    let s = agents::release().unwrap();
    assert!(s.project.entries.is_empty(), "undo through a tool");
    assert_eq!(s.project.binder.get(id.parse().unwrap()).unwrap().status.as_deref(), Some("First draft"));
}
