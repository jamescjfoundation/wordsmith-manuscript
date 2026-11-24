// SPDX-License-Identifier: MIT OR Apache-2.0
//! Every Manuscript command as an agent tool (AUT-2), through the core's `jcj-agents`.
//!
//! The core's agent server works on an engine session; Manuscript's commands work on a project
//! session. Each Manuscript command is registered with the engine as `manuscript.<id>` (the tool
//! `manuscript_<id>`), whose run passes its parameters to the project session held here. A
//! command that changes a project with a folder saves it. Paths in parameters are checked against
//! the folders the agent may read and write (`--read-root`, `--write-root`).

use std::sync::{Arc, Mutex};

use jcj_agents::{AgentConfig, AgentInfo, AgentServer};
use jcj_engine::{CmdError, CmdResult, CommandSpec};
use serde_json::{Value, json};

use crate::session::Session;

/// The project the tools work on.
static PROJECT: Mutex<Option<Session>> = Mutex::new(None);

/// The prefix of Manuscript's commands in the engine's registry.
pub const PREFIX: &str = "manuscript.";

/// What the agent is told about the app.
pub const INSTRUCTIONS: &str = "Wordsmith Manuscript holds a novel or other long work as a project: a binder of items in five areas \
(manuscript, bible, research, notes, trash), each item with a role (part, chapter, scene, front-matter, back-matter), a text in a Markdown \
dialect, metadata (synopsis, label, status, keywords, target) and snapshots. Start with manuscript_binder_list to see the items and their ids, \
read a text with manuscript_text_get and change it with manuscript_text_set. Every change is one undo step (manuscript_edit_undo). Imports and \
exports are checked word for word.";

fn run_tool(id: &str, v: &Value) -> CmdResult {
    for key in ["path", "folder"] {
        if let Some(p) = v.get(key).and_then(Value::as_str) {
            let writes = id == "export.file";
            let r = if writes { jcj_engine::file_access::check_write(p) } else { jcj_engine::file_access::check_read(p) };
            r.map_err(CmdError::Disabled)?;
        }
    }
    if let Some(list) = v.get("paths").and_then(Value::as_array) {
        for p in list.iter().filter_map(Value::as_str) {
            jcj_engine::file_access::check_read(p).map_err(CmdError::Disabled)?;
        }
    }
    let mut guard = PROJECT.lock().map_err(|_| CmdError::Failed("the project is not available".into()))?;
    let s = guard.as_mut().ok_or_else(|| CmdError::Failed("no project is open".into()))?;
    let before = s.revision();
    let out = s.run(id, v).map_err(CmdError::Failed)?;
    if s.revision() != before && s.root().is_some() {
        s.save().map_err(CmdError::Failed)?;
    }
    Ok(out)
}

macro_rules! tools {
    ($($id:literal),* $(,)?) => {
        /// The engine commands standing for Manuscript's commands.
        pub fn engine_specs() -> Vec<CommandSpec> {
            let mine = crate::commands::specs();
            let mut out = Vec::new();
            $(
                let mut spec = CommandSpec::new(concat!("manuscript.", $id), $id, "Manuscript", |_s, v| run_tool($id, v));
                if let Some(m) = mine.iter().find(|m| m.id == $id) {
                    spec.label = m.label;
                    spec.params = m.params;
                    spec.mutates = m.mutates;
                }
                out.push(spec);
            )*
            out
        }

        /// The ids of the commands that are tools.
        pub const TOOLS: &[&str] = &[$($id),*];
    };
}

tools!(
    "binder.new",
    "binder.newFolder",
    "binder.rename",
    "binder.move",
    "binder.moveUp",
    "binder.moveDown",
    "binder.indent",
    "binder.outdent",
    "binder.setRole",
    "binder.setInclude",
    "binder.trash",
    "binder.restore",
    "binder.emptyTrash",
    "settings.typing",
    "binder.list",
    "edit.undo",
    "edit.redo",
    "item.setSynopsis",
    "item.setNotes",
    "item.setLabel",
    "item.setStatus",
    "item.addKeyword",
    "item.removeKeyword",
    "item.linkEntry",
    "item.unlinkEntry",
    "item.setTarget",
    "item.setField",
    "item.get",
    "project.setLabels",
    "project.setStatuses",
    "project.setKeywords",
    "project.setBook",
    "text.get",
    "text.set",
    "find.project",
    "find.replaceProject",
    "spelling.addWord",
    "spelling.check",
    "snapshot.take",
    "snapshot.list",
    "snapshot.restore",
    "snapshot.rename",
    "snapshot.compare",
    "comment.add",
    "comment.set",
    "comment.remove",
    "comment.list",
    "project.setFields",
    "project.renameField",
    "snapshot.delete",
    "snapshot.get",
    "outline.list",
    "outline.applySort",
    "bible.new",
    "bible.list",
    "bible.templates",
    "entry.get",
    "entry.set",
    "entry.setField",
    "entry.setBody",
    "bible.backlinks",
    "import.preview",
    "import.set",
    "import.document",
    "import.notes",
    "export.file",
    "export.presets",
    "export.savePreset",
    "targets.set",
    "counts.get",
    "search.project",
);

/// Put a project in the tools' hands (replacing any other).
pub fn hold(s: Session) {
    if let Ok(mut g) = PROJECT.lock() {
        *g = Some(s);
    }
}

/// Take the project back from the tools.
pub fn release() -> Option<Session> {
    PROJECT.lock().ok().and_then(|mut g| g.take())
}

/// An agent server over a project: the engine's general tools and every Manuscript command.
pub fn server(s: Session, roots: Option<Arc<jcj_engine::file_access::AutomationRoots>>) -> AgentServer {
    hold(s);
    let registry = Arc::new(jcj_engine::registry_with(engine_specs()));
    let session = jcj_engine::session_with(jcj_doc::new_document(), registry);
    let info = AgentInfo::new("wordsmith-manuscript", crate::NAME, env!("CARGO_PKG_VERSION"), INSTRUCTIONS);
    AgentServer::headless(session, AgentConfig::new(info).export(PREFIX).confined(roots))
}

/// Serve the tools over standard input and output until the client goes away.
pub fn serve(s: Session) -> Result<(), String> {
    server(s, None).serve_stdio().map_err(|e| e.to_string())
}

/// A JSON-RPC request (tests, and clients driving the server in process).
pub fn request(id: u64, method: &str, params: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params })
}
