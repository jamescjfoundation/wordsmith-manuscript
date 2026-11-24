// SPDX-License-Identifier: MIT OR Apache-2.0
//! Wordsmith Manuscript's views: the binder, the editor and stitched view, the corkboard, the
//! outline, the inspector, the bible, search, the export dialog, ribbon tabs and panels (layer 4).

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

pub mod binder;
pub mod brand;
pub mod commands;
pub mod inspector;
pub mod session;
pub mod texts;
pub mod tree;

use std::path::{Path, PathBuf};
use std::time::Instant;

use jcj_shell::commands::ids;
use jcj_shell::egui;
use jcj_shell::{About, App, Cmd, CommandInfo, Commands, Page, PageSpec, PanelSpec, Shell, Side};
use serde_json::json;
use wordsmith_manuscript_model::{ItemId, Project};

use crate::binder::BinderView;
use crate::session::Session;

/// The product name shown in the window and the About page.
pub const NAME: &str = "Wordsmith Manuscript";

/// The binder panel's id.
pub const BINDER_PANEL: &str = "manuscript.binder";

/// The desktop app's state: what the shell draws around.
pub struct Manuscript {
    /// The open project, if any.
    pub session: Option<Session>,
    /// The binder panel.
    pub binder: BinderView,
    /// The last message for the status bar.
    pub status: String,
    /// Path typed on the New page.
    new_path: String,
    /// Path typed on the Open page.
    open_path: String,
    /// Path typed on the Save page (Save As, one file).
    save_path: String,
    /// Unsaved work found in the project's recovery folder, waiting for the writer's choice.
    pub recovered: Option<Project>,
}

impl Default for Manuscript {
    fn default() -> Self {
        Manuscript::new()
    }
}

fn documents_dir() -> PathBuf {
    std::env::var_os("HOME").map(PathBuf::from).map_or_else(|| PathBuf::from("."), |h| h.join("Documents"))
}

impl Manuscript {
    /// An app with no project open.
    pub fn new() -> Manuscript {
        let docs = documents_dir();
        Manuscript {
            session: None,
            binder: BinderView::default(),
            status: String::new(),
            new_path: docs.join("Untitled.wsm").to_string_lossy().into_owned(),
            open_path: String::new(),
            save_path: String::new(),
            recovered: None,
        }
    }

    /// An app with a session already open (tests, agents).
    pub fn with_session(session: Session) -> Manuscript {
        let mut m = Manuscript::new();
        m.session = Some(session);
        m
    }

    /// Open a project folder, its `project.toml`, or a packed `.wsmz` (unpacked beside it).
    pub fn open(&mut self, path: &Path) -> Result<(), String> {
        let s = if path.extension().is_some_and(|e| e == wordsmith_manuscript_format::pack::EXTENSION) {
            let dest = path.with_extension("wsm");
            Session::open_packed(path, &dest)?
        } else {
            Session::open(path)?
        };
        self.recovered = s.root().and_then(wordsmith_manuscript_format::pack::recovered).filter(|r| *r != s.project);
        self.session = Some(s);
        self.binder = BinderView::default();
        self.status = "Opened.".into();
        Ok(())
    }

    /// Close the open project: save if needed, back up.
    pub fn close(&mut self) -> Result<(), String> {
        if let Some(s) = self.session.as_mut() {
            s.close()?;
        }
        Ok(())
    }

    fn project_name(&self) -> String {
        match &self.session {
            None => "No project".into(),
            Some(s) => {
                let title = &s.project.settings.book.title;
                if title.is_empty() {
                    s.root().and_then(|r| r.file_stem()).map_or("Untitled".into(), |n| n.to_string_lossy().into_owned())
                } else {
                    title.clone()
                }
            }
        }
    }

    fn report(&mut self, r: Result<(), String>) {
        if let Err(e) = r {
            self.status = e;
        }
    }
}

impl Commands for Manuscript {
    fn command(&self, id: &str) -> Option<CommandInfo> {
        self.all_commands().into_iter().find(|c| c.id == id)
    }

    fn all_commands(&self) -> Vec<CommandInfo> {
        let mut v = vec![
            CommandInfo { icon: "save".into(), ..CommandInfo::new(ids::SAVE, "Save", "Mod+S") },
            CommandInfo { icon: "file".into(), ..CommandInfo::new(ids::NEW, "New Project", "Mod+N") },
            CommandInfo { icon: "open".into(), ..CommandInfo::new(ids::OPEN, "Open Project", "Mod+O") },
            CommandInfo::new(ids::OPEN_PATH, "Open Recent", ""),
            CommandInfo::new(ids::SAVE_AS, "Save As", "Mod+Shift+S"),
            CommandInfo { icon: "undo".into(), ..CommandInfo::new(ids::UNDO, "Undo", "Mod+Z") },
            CommandInfo { icon: "redo".into(), ..CommandInfo::new(ids::REDO, "Redo", "Mod+Shift+Z") },
        ];
        for spec in commands::specs() {
            if spec.id == "edit.undo" || spec.id == "edit.redo" {
                continue;
            }
            v.push(CommandInfo::new(spec.id, spec.label, spec.key));
        }
        v
    }
}

impl App for Manuscript {
    fn document_name(&self) -> String {
        self.project_name()
    }

    fn document_path(&self) -> Option<String> {
        self.session.as_ref().and_then(|s| s.root()).map(|r| r.to_string_lossy().into_owned())
    }

    fn is_dirty(&self) -> bool {
        self.session.as_ref().is_some_and(Session::is_dirty)
    }

    fn revision(&self) -> u64 {
        self.session.as_ref().map_or(0, Session::revision)
    }

    fn autosave(&mut self) -> Result<(), String> {
        let Some(s) = self.session.as_mut() else { return Ok(()) };
        if s.root().is_none() {
            return s.write_recovery();
        }
        s.save()?;
        if s.backup_due(Instant::now()) {
            s.backup()?;
        }
        Ok(())
    }

    fn run(&mut self, cmd: &Cmd, shell: &mut Shell) -> Result<(), String> {
        match cmd.id.as_str() {
            ids::SAVE => {
                let s = self.session.as_mut().ok_or("no project is open")?;
                s.save()?;
                self.status = "Saved.".into();
                Ok(())
            }
            ids::NEW => {
                shell.open_file_page(Page::New);
                Ok(())
            }
            ids::OPEN => {
                shell.open_file_page(Page::Open);
                Ok(())
            }
            ids::SAVE_AS => {
                shell.open_file_page(Page::Save);
                Ok(())
            }
            ids::OPEN_PATH => {
                let path = cmd.params.get("path").and_then(|p| p.as_str()).ok_or("no path")?.to_owned();
                self.close()?;
                self.open(Path::new(&path))?;
                shell.add_recent(&path);
                Ok(())
            }
            ids::UNDO => {
                let s = self.session.as_mut().ok_or("no project is open")?;
                self.status = s.undo().map_or("Nothing to undo.".into(), |l| format!("Undid {l}."));
                Ok(())
            }
            ids::REDO => {
                let s = self.session.as_mut().ok_or("no project is open")?;
                self.status = s.redo().map_or("Nothing to redo.".into(), |l| format!("Redid {l}."));
                Ok(())
            }
            "binder.emptyTrash" if cmd.params.get("confirm").is_none() => {
                self.binder.confirm_empty = true;
                Ok(())
            }
            id => {
                let s = self.session.as_mut().ok_or("no project is open")?;
                let mut params = cmd.params.clone();
                // Commands on "the selected item" from the ribbon and keys.
                if params.get("id").is_none()
                    && let Some(sel) = self.binder.selected
                    && commands::find(id).is_some_and(|spec| spec.params.contains("\"id\""))
                {
                    params["id"] = json!(sel.as_text());
                }
                if (id == "binder.new" || id == "binder.newFolder")
                    && params.get("after").is_none()
                    && params.get("inside").is_none()
                    && params.get("area").is_none()
                {
                    if let Some(sel) = self.binder.selected {
                        params["after"] = json!(sel.as_text());
                    }
                }
                let out = s.run(id, &params)?;
                if let Some(new) = out.get("id").and_then(|v| v.as_str()).and_then(|v| v.parse::<ItemId>().ok()) {
                    self.binder.selected = Some(new);
                    self.binder.tree.rename(binder::Node::Item(new), "Untitled");
                }
                Ok(())
            }
        }
    }

    fn document_ui(&mut self, ui: &mut egui::Ui, _shell: &mut Shell) {
        if self.recovered.is_some() {
            ui.horizontal(|ui| {
                ui.label("Unsaved work from a session that did not end cleanly was found.");
                if ui.button("Recover It").clicked() {
                    self.accept_recovered();
                }
                if ui.button("Keep the Saved Version").clicked() {
                    self.discard_recovered();
                }
            });
            ui.separator();
        }
        let Some(s) = self.session.as_ref() else {
            ui.centered_and_justified(|ui| ui.label("No project open. Use File › New or File › Open."));
            return;
        };
        let Some(id) = self.binder.selected else {
            ui.centered_and_justified(|ui| ui.label("Select a document in the binder."));
            return;
        };
        let title = s.project.binder.get(id).map(|i| i.title.clone()).unwrap_or_default();
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            ui.heading(title);
            if let Some(t) = s.project.text(id) {
                for p in t.body.plain_paragraphs() {
                    ui.label(p);
                }
            }
        });
    }

    fn panels(&self) -> Vec<PanelSpec> {
        vec![PanelSpec::new(BINDER_PANEL, "Binder", "list", Side::Left)]
    }

    fn panel_ui(&mut self, panel: &str, ui: &mut egui::Ui, _shell: &mut Shell) {
        if panel != BINDER_PANEL {
            return;
        }
        let Some(s) = self.session.as_mut() else {
            ui.label("No project open.");
            return;
        };
        let errors = binder::panel(ui, s, &mut self.binder);
        if let Some(e) = errors.into_iter().last() {
            self.status = e;
        }
    }

    fn status_ui(&mut self, ui: &mut egui::Ui, _shell: &mut Shell) {
        if let Some(s) = &self.session {
            ui.label(format!("Manuscript: {} words", binder::format_words(s.project.manuscript_words())));
        }
        if !self.status.is_empty() {
            ui.separator();
            ui.label(&self.status);
        }
    }

    fn file_pages(&self) -> Vec<PageSpec> {
        Vec::new()
    }

    fn file_page_ui(&mut self, page: &Page, ui: &mut egui::Ui, shell: &mut Shell) {
        match page {
            Page::New => {
                ui.label("Folder for the new project (it must not exist yet):");
                ui.text_edit_singleline(&mut self.new_path);
                if ui.button("Create Empty Project").clicked() {
                    let r = self.close().and_then(|()| Session::create(Path::new(&self.new_path), Project::new()));
                    match r {
                        Ok(s) => {
                            shell.add_recent(&self.new_path);
                            self.session = Some(s);
                            self.binder = BinderView::default();
                            shell.open_file_page(Page::App(String::new()));
                        }
                        Err(e) => self.status = e,
                    }
                }
            }
            Page::Open => {
                ui.label("A project folder (.wsm), its project.toml, or a packed project (.wsmz):");
                ui.text_edit_singleline(&mut self.open_path);
                if ui.button("Open").clicked() {
                    let path = self.open_path.clone();
                    let r = self.close().and_then(|()| self.open(Path::new(&path)));
                    if r.is_ok() {
                        shell.add_recent(&path);
                    }
                    self.report(r);
                }
            }
            Page::Save => {
                ui.label("Save a copy as a new project folder, or as one packed file (.wsmz):");
                ui.text_edit_singleline(&mut self.save_path);
                ui.horizontal(|ui| {
                    if ui.button("Save As Folder").clicked() {
                        let path = PathBuf::from(&self.save_path);
                        let r = self.session.as_mut().ok_or_else(|| "no project is open".to_owned()).and_then(|s| s.save_as(&path));
                        self.report(r);
                    }
                    if ui.button("Save as One File").clicked() {
                        let mut path = PathBuf::from(&self.save_path);
                        if path.extension().is_none_or(|e| e != wordsmith_manuscript_format::pack::EXTENSION) {
                            path.set_extension(wordsmith_manuscript_format::pack::EXTENSION);
                        }
                        let r = self.session.as_ref().ok_or_else(|| "no project is open".to_owned()).and_then(|s| s.save_packed(&path));
                        self.report(r);
                    }
                });
            }
            _ => {}
        }
    }

    fn about(&self) -> About {
        About {
            summary: "An open-source app for writing novels and other long work.".into(),
            credit: "Based on WordCraft by the ArtCraft team.".into(),
            ..About::default()
        }
    }

    fn mark_width(&self, height: f32) -> f32 {
        height
    }

    fn paint_mark(&self, painter: &egui::Painter, rect: egui::Rect, ink: egui::Color32) {
        brand::paint_mark(painter, rect, ink);
    }
}

/// Recovery: what to do with unsaved work found when a project opened.
impl Manuscript {
    /// Take the recovered work as the project (one undoable change).
    pub fn accept_recovered(&mut self) {
        if let (Some(r), Some(s)) = (self.recovered.take(), self.session.as_mut()) {
            let mut texts: Vec<ItemId> = r.texts.keys().chain(s.project.texts.keys()).copied().collect();
            texts.sort();
            texts.dedup();
            let mut entries: Vec<ItemId> = r.entries.keys().chain(s.project.entries.keys()).copied().collect();
            entries.sort();
            entries.dedup();
            let _ = s.change("Recover Unsaved Work", &texts, &entries, |p| {
                *p = r;
                Ok(())
            });
        }
    }

    /// Keep the saved project and drop the recovered work.
    pub fn discard_recovered(&mut self) {
        self.recovered = None;
        if let Some(root) = self.session.as_ref().and_then(|s| s.root()) {
            let _ = wordsmith_manuscript_format::pack::discard_recovery(root);
        }
    }
}
