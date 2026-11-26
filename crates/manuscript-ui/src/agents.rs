// SPDX-License-Identifier: MIT OR Apache-2.0
//! Every Manuscript command as an agent tool (AUT-2), through the core's `jcj-agents`.
//!
//! Manuscript's commands work on a project session, not on a document, so they are given to the
//! core's server as an app's own commands ([`AppCommands`]): the server names a tool
//! `manuscript_<command>` after each, lists it, checks the files its parameters name against the
//! folders the agent may read and write, and calls it. There are no general document tools
//! beside them: no document stands behind them here.
//!
//! - Which parameters of a command name files it reads and files it writes is said in one place,
//!   [`files_of`]. A parameter that names a file and is not listed there is not checked.
//! - The project is the person's, not the agent's: it was opened before any folder was put in
//!   force, it is read where it lives, and a command that changes it saves it there.
//! - `export.convert` runs a program the caller names. A program cannot be kept to a folder, so
//!   the command is refused while folders are in force.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use jcj_agents::{AgentConfig, AgentInfo, AgentServer, AppCommand, AppCommands};
use jcj_engine::file_access::{self, AutomationRoots};
use serde_json::{Value, json};

use crate::session::Session;

/// The app's short name: every tool's name starts with it and an underscore.
pub const PREFIX: &str = "manuscript";

/// What the agent is told about the app.
pub const INSTRUCTIONS: &str = "Wordsmith Manuscript holds a novel or other long work as a project: a binder of items in five areas \
(manuscript, bible, research, notes, trash), each item with a role (part, chapter, scene, front-matter, back-matter), a text in a Markdown \
dialect, metadata (synopsis, label, status, keywords, target) and snapshots. Start with manuscript_binder_list to see the items and their ids, \
read a text with manuscript_text_get and change it with manuscript_text_set. Every change is one undo step (manuscript_edit_undo). Imports and \
exports are checked word for word.";

/// A project session the tools and their caller both hold.
pub type Shared = Rc<RefCell<Session>>;

/// The parameters of command `id` that name files or folders: those it reads, then those it
/// writes. Every other command names none.
pub fn files_of(id: &str) -> (&'static [&'static str], &'static [&'static str]) {
    match id {
        "import.preview" | "import.document" | "import.notes" | "media.import" => (&["path"], &[]),
        "import.set" => (&["paths", "folder"], &[]),
        "offers.list" | "offers.accept" | "offers.reject" => (&["path"], &[]),
        "export.file" | "export.convert" | "series.create" => (&[], &["path"]),
        // Joining reads the series bible and adds this book to its list.
        "series.join" => (&["path"], &["path"]),
        _ => (&[], &[]),
    }
}

/// Every Manuscript command as the agent sees it.
pub fn commands() -> Vec<AppCommand> {
    crate::commands::specs()
        .iter()
        .map(|spec| {
            let (reads, writes) = files_of(spec.id);
            let mut command = AppCommand::new(spec.id, spec.label).params(spec.params).reads(reads).writes(writes);
            // An export changes nothing in the project, but it writes a file.
            command.read_only = !spec.mutates && writes.is_empty();
            command
        })
        .collect()
}

/// The names of the tools, in the order the server lists them.
pub fn tool_names() -> Vec<String> {
    commands().iter().map(|c| jcj_agents::app_tool_name(PREFIX, &c.id)).collect()
}

/// Manuscript's commands on one project, for the core's server.
pub struct ProjectTools {
    session: Shared,
}

impl ProjectTools {
    /// The tools over `session`.
    pub fn new(session: Shared) -> Self {
        ProjectTools { session }
    }
}

impl AppCommands for ProjectTools {
    fn commands(&self) -> Vec<AppCommand> {
        commands()
    }

    fn run(&mut self, id: &str, params: &Value) -> Result<Value, String> {
        if id == "export.convert" && file_access::current().is_some() {
            return Err("export.convert runs a program, which cannot be kept to the folders this agent was given".into());
        }
        let mut s = self.session.try_borrow_mut().map_err(|_| "the project is in use".to_owned())?;
        // The server has checked the files the parameters name. What is left is the project's
        // own folder, which is the person's choice and is read and saved where it lives.
        file_access::unconfined(|| {
            let before = s.revision();
            let out = s.run(id, params)?;
            if s.revision() != before && s.root().is_some() {
                s.save()?;
            }
            Ok(out)
        })
    }
}

fn config(roots: Option<Arc<AutomationRoots>>) -> AgentConfig {
    let info = AgentInfo::new("wordsmith-manuscript", crate::NAME, env!("CARGO_PKG_VERSION"), INSTRUCTIONS);
    AgentConfig::new(info).confined(roots)
}

/// An agent server over a project the caller keeps a hold of: every Manuscript command, and
/// nothing else. `roots` are the folders the agent's commands may read and write (`None`:
/// nothing is confined).
pub fn server_over(session: Shared, roots: Option<Arc<AutomationRoots>>) -> AgentServer {
    AgentServer::for_app(Box::new(ProjectTools::new(session)), PREFIX, config(roots))
}

/// [`server_over`] for a project nothing else needs.
pub fn server(s: Session, roots: Option<Arc<AutomationRoots>>) -> AgentServer {
    server_over(Rc::new(RefCell::new(s)), roots)
}

/// Serve the tools over standard input and output until the client goes away.
pub fn serve(s: Session, roots: Option<Arc<AutomationRoots>>) -> Result<(), String> {
    server(s, roots).serve_stdio().map_err(|e| e.to_string())
}

/// A JSON-RPC request (tests, and clients driving the server in process).
pub fn request(id: u64, method: &str, params: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params })
}
