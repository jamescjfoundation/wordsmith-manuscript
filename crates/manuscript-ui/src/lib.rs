// SPDX-License-Identifier: MIT OR Apache-2.0
//! Wordsmith Manuscript's views: the binder, the editor and stitched view, the corkboard, the
//! outline, the inspector, the bible, search, the export dialog, ribbon tabs and panels (layer 4).

#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

pub mod agents;
pub mod bible;
pub mod binder;
pub mod board;
pub mod bookmarks;
pub mod brand;
pub mod collections;
pub mod commands;
pub mod comments;
pub mod cork;
pub mod drafts;
pub mod editor;
pub mod entry_types;
pub mod exports;
pub mod fields;
pub mod filter;
pub mod find;
pub mod findbar;
pub mod history;
pub mod i18n;
pub mod imports;
pub mod inspector;
pub mod media;
pub mod offers;
pub mod outline;
pub mod pages;
pub mod progress;
pub mod revisions;
pub mod ribbon;
pub mod search;
pub mod series;
pub mod session;
pub mod snapshots;
pub mod spelling;
pub mod stitch;
pub mod table;
pub mod texts;
pub mod tree;
pub mod typing;
pub mod user_templates;
pub mod view;

use std::path::{Path, PathBuf};
use std::time::Instant;

use jcj_shell::commands::ids;
use jcj_shell::egui;
use jcj_shell::{About, App, Cmd, CommandInfo, Commands, Page, PageSpec, PanelSpec, Shell, Side};
use serde_json::json;
use wordsmith_manuscript_model::{ItemId, Project, templates};

use crate::binder::BinderView;
use crate::session::Session;

/// The product name shown in the window and the About page.
pub const NAME: &str = "Wordsmith Manuscript";

/// The binder panel's id.
pub const BINDER_PANEL: &str = "manuscript.binder";
/// The inspector panel's id.
pub const INSPECTOR_PANEL: &str = "manuscript.inspector";

/// What the central area shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ViewMode {
    /// The text editor.
    #[default]
    Editor,
    /// Index cards of a folder's children.
    Corkboard,
    /// The outline table.
    Outline,
    /// The freeform board (PLN-3).
    Board,
    /// Standard manuscript pages (ED-8).
    Pages,
}

/// How often the project folder is checked for changes made elsewhere.
pub const OUTSIDE_EVERY: std::time::Duration = std::time::Duration::from_secs(3);

/// A `#RRGGBB` color.
fn hex_color(s: &str) -> Option<egui::Color32> {
    let h = s.strip_prefix('#')?;
    let b = |i: usize| h.get(i..i + 2).and_then(|x| u8::from_str_radix(x, 16).ok());
    Some(egui::Color32::from_rgb(b(0)?, b(2)?, b(4)?))
}

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
    /// Path typed on the Import page.
    import_path: String,
    /// The last import's report.
    pub import_report: String,
    /// The Export page's path, format and options.
    export_path: String,
    export_format: String,
    /// The export options on the Export page.
    pub export_options: wordsmith_manuscript_io::compile::Options,
    /// The last export's report.
    pub export_report: String,
    /// The import rules on the Import page.
    pub import_rules: wordsmith_manuscript_io::mapping::Rules,
    import_join: bool,
    /// The open editor, if an item is open.
    pub editor: Option<editor::Editor>,
    /// How the editor looks.
    pub look: editor::EditorLook,
    /// The page view's state (ED-8).
    pub pages: pages::PageView,
    /// The freeform board's state (PLN-3).
    pub board: board::BoardView,
    /// Pictures and PDFs loaded for the media viewer (BND-6).
    pub media_view: media::MediaView,
    /// The collection shown in place of the binder (SRC-2).
    pub showing: Option<String>,
    /// The Series window (BND-8).
    pub series_view: series::SeriesView,
    /// The drafts window (DRF-1).
    pub drafts_view: drafts::DraftsView,
    /// The "Offered fixes" inbox (plan 9.2 item 4).
    pub offers_view: offers::InboxView,
    /// How many offered fixes wait (looked up with the outside check).
    pub offers_pending: usize,
    /// When the project folder was last checked for outside changes (FIL-4).
    outside_checked: Option<Instant>,
    /// Where writing histories are kept on this computer (the app's settings folder), if anywhere.
    pub history_dir: Option<std::path::PathBuf>,
    /// Where the writer's own project templates are kept on this computer, if anywhere.
    pub templates_dir: Option<std::path::PathBuf>,
    /// Saving the project as a template: the name and description being typed.
    pub save_template: Option<(String, String)>,
    /// The entry types window (BIB-2).
    pub entry_types: entry_types::TypesView,
    /// The writing history window (PRG-3).
    pub history_view: history::HistoryView,
    /// The snapshot comparison window (INS-5).
    pub compare: snapshots::CompareView,
    /// The other pane's editor, when the editor is split (ED-6).
    pub other: Option<editor::Editor>,
    /// How the editor is split.
    pub split: jcj_shell::split::Split,
    split_state: jcj_shell::split::SplitState,
    /// Which pane is active (0 the first, 1 the second).
    pub active_pane: usize,
    /// Whether the editor had the keyboard last (undo goes to it then).
    pub editor_focused: bool,
    /// The find bar.
    pub find: findbar::FindBar,
    /// The inspector panel.
    pub inspector: inspector::InspectorView,
    /// What the central area shows.
    pub mode: ViewMode,
    /// The outline's state.
    pub outline: outline::OutlineView,
    /// The corkboard's state.
    pub cork: cork::CorkView,
    /// The bible entry form.
    pub entry: bible::EntryView,
    /// Project search: the index, the query and the results.
    pub searcher: search::Searcher,
    search_text: String,
    /// The last search's results: (item, matches).
    pub search_results: Option<Vec<(ItemId, usize)>>,
    /// The writing session's statistics, for the open project.
    pub progress: Option<progress::Progress>,
    /// The folder the corkboard and outline show (follows the binder's selection).
    pub view_root: Option<ItemId>,
    /// The filter on binder, corkboard and outline.
    pub filter: filter::Filter,
    filtered_at: u64,
    /// The view preferences.
    pub view: view::ViewPrefs,
    /// Focus mode: the chrome it hid, while on.
    pub focus: Option<view::Chrome>,
    fullscreen: Option<bool>,
    pending_unfocus: bool,
    registry: std::sync::Arc<jcj_engine::Registry>,
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
            import_path: String::new(),
            import_report: String::new(),
            export_path: docs.join("Manuscript.docx").to_string_lossy().into_owned(),
            export_format: "manuscript-docx".into(),
            export_options: wordsmith_manuscript_io::compile::Options::default(),
            export_report: String::new(),
            import_rules: wordsmith_manuscript_io::mapping::Rules::default(),
            import_join: true,
            editor: None,
            other: None,
            compare: snapshots::CompareView::default(),
            history_dir: None,
            outside_checked: None,
            offers_view: offers::InboxView::default(),
            drafts_view: drafts::DraftsView::default(),
            series_view: series::SeriesView::default(),
            offers_pending: 0,
            history_view: history::HistoryView::default(),
            entry_types: entry_types::TypesView::default(),
            templates_dir: None,
            save_template: None,
            showing: None,
            media_view: media::MediaView::default(),
            board: board::BoardView::default(),
            pages: pages::PageView::default(),
            split: jcj_shell::split::Split::None,
            split_state: jcj_shell::split::SplitState::default(),
            active_pane: 0,
            look: editor::EditorLook::default(),
            editor_focused: false,
            find: findbar::FindBar::default(),
            inspector: inspector::InspectorView::default(),
            mode: ViewMode::Editor,
            outline: outline::OutlineView::default(),
            cork: cork::CorkView::default(),
            entry: bible::EntryView::default(),
            view_root: None,
            progress: None,
            searcher: search::Searcher::default(),
            search_text: String::new(),
            search_results: None,
            filter: filter::Filter::default(),
            filtered_at: u64::MAX,
            view: view::ViewPrefs::default(),
            focus: None,
            fullscreen: None,
            pending_unfocus: false,
            registry: editor::registry(),
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
        self.keep_history();
        self.session = Some(s);
        self.binder = BinderView::default();
        self.progress = None;
        self.status = "Opened.".into();
        Ok(())
    }

    /// Take in changes made to the project folder elsewhere (FIL-4), reloading editors whose
    /// text was read again.
    pub fn check_outside(&mut self) {
        if self.session.as_ref().and_then(Session::root).is_none() {
            return;
        }
        self.write_back();
        let Some(s) = self.session.as_mut() else { return };
        match s.check_outside() {
            Ok(o) if o.is_empty() => {}
            Ok(o) => {
                self.status = o.summary();
                let open: Vec<ItemId> = [self.editor.as_ref(), self.other.as_ref()].into_iter().flatten().map(|e| e.item).collect();
                if o.project_reloaded {
                    self.other = None;
                    self.split = jcj_shell::split::Split::None;
                    self.active_pane = 0;
                    if self.editor.as_ref().is_some_and(|e| self.session.as_ref().is_some_and(|s| !s.project.binder.contains(e.item))) {
                        self.editor = None;
                    }
                    self.reload_editor();
                } else if o.reloaded.iter().any(|i| open.contains(i)) || self.editor.as_ref().is_some_and(stitch::is_stitched) {
                    self.reload_editor();
                    if let Some(oth) = self.other.take() {
                        self.other = Some(oth);
                        self.switch_pane();
                        self.reload_editor();
                        self.switch_pane();
                    }
                }
            }
            Err(e) => self.status = format!("Could not look for outside changes: {e}"),
        }
    }

    /// Start counting this session's words, with the project's writing history from this
    /// computer.
    pub fn start_progress(&mut self) {
        if self.progress.is_some() {
            return;
        }
        let Some(s) = self.session.as_ref() else { return };
        let mut p = progress::Progress::start(&s.project);
        if let (Some(dir), Some(root)) = (self.history_dir.as_ref(), s.root()) {
            p.writing.history = history::load(dir, root);
        }
        self.progress = Some(p);
    }

    /// Keep the writing history on this computer (not in the project).
    pub fn keep_history(&mut self) {
        let (Some(dir), Some(p), Some(root)) = (self.history_dir.as_ref(), self.progress.as_ref(), self.session.as_ref().and_then(Session::root))
        else {
            return;
        };
        if let Err(e) = history::save(dir, root, &p.writing.history) {
            self.status = e;
        }
    }

    /// Close the open project: save if needed, back up.
    pub fn close(&mut self) -> Result<(), String> {
        self.keep_history();
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

    /// A free folder for a new project from `template` in the Documents folder (or beside the
    /// path typed on the New page).
    pub fn new_project_path(&self, template: &str) -> PathBuf {
        let typed = PathBuf::from(&self.new_path);
        let dir = typed.parent().map_or_else(documents_dir, Path::to_path_buf);
        let base = templates::TEMPLATES.iter().find(|t| t.id == template).map_or("Untitled", |t| t.name);
        let mut n = 1;
        loop {
            let name = if n == 1 { format!("{base}.wsm") } else { format!("{base} {n}.wsm") };
            let p = dir.join(name);
            if !p.exists() || n > 999 {
                return p;
            }
            n += 1;
        }
    }

    /// Make a new project from a template at `root` and open it.
    pub fn new_from_template(&mut self, template: &str, root: &Path) -> Result<(), String> {
        let title = root.file_stem().map_or("Untitled".into(), |n| n.to_string_lossy().into_owned());
        let project = match (template.strip_prefix("user:"), self.templates_dir.as_ref()) {
            (Some(slug), Some(dir)) => user_templates::build(dir, slug, &title)?,
            (Some(slug), None) => return Err(format!("there is no template `{slug}`")),
            (None, _) => templates::build(template, &title, &mut ItemId::new).ok_or_else(|| format!("no template `{template}`"))?,
        };
        self.close()?;
        let s = Session::create(root, project)?;
        self.session = Some(s);
        self.binder = BinderView::default();
        self.status = format!("Made {}.", root.display());
        Ok(())
    }

    /// Write the editor's text back into the project if it changed (a save point).
    pub fn write_back(&mut self) {
        let Some(s) = self.session.as_mut() else { return };
        for ed in [self.editor.as_mut(), self.other.as_mut()].into_iter().flatten() {
            if !ed.is_dirty() {
                continue;
            }
            if crate::stitch::write_back(ed, &mut s.project) {
                s.touch();
            }
            ed.written = ed.revision;
        }
    }

    /// Give the open editors the project's revision passes: typing follows the current pass and
    /// each pass shows in its color (PRG-4).
    pub fn mark_revisions(&mut self) {
        let Some(s) = self.session.as_ref() else { return };
        let r = revisions::of(&s.project);
        let colors: Vec<egui::Color32> = r.sets.iter().map(|set| hex_color(&set.color).unwrap_or(egui::Color32::LIGHT_BLUE)).collect();
        for ed in [self.editor.as_mut(), self.other.as_mut()].into_iter().flatten() {
            let title = s.project.binder.get(ed.item).map_or_else(String::new, |i| i.title.clone());
            ed.label =
                if stitch::is_stitched(ed) { format!("Text of “{title}” and the documents after it") } else { format!("Text of “{title}”") };
            if jcj_features::revisions::Revisions::of_doc(&ed.session.doc) != r {
                let _ = r.store(&mut ed.session.doc);
            }
            if ed.pass_colors != colors {
                ed.pass_colors.clone_from(&colors);
                ed.relayout();
            }
        }
    }

    /// Whether the other pane holds `id`, alone or in a stitched view (one item is only ever
    /// editable in one pane, so no edit can overwrite another; here it opens read-only).
    fn held_elsewhere(&self, id: ItemId) -> bool {
        self.other.as_ref().is_some_and(|o| if stitch::is_stitched(o) { stitch::dividers(&o.session.doc).contains(&id) } else { o.item == id })
    }

    /// Back to one pane: the remaining editor, read-only only because of the other, is editable
    /// again (reloaded, so it shows what the other pane wrote).
    fn one_pane(&mut self) {
        self.write_back();
        self.other = None;
        self.split = jcj_shell::split::Split::None;
        self.active_pane = 0;
        if self.editor.as_ref().is_some_and(|e| e.read_only) {
            self.reload_editor();
        }
    }

    /// Open the editor on an item (writing the current one back first).
    pub fn open_item(&mut self, id: ItemId) {
        if self.editor.as_ref().is_some_and(|e| e.item == id && !crate::stitch::is_stitched(e)) {
            return;
        }
        self.write_back();
        let Some(s) = self.session.as_ref() else { return };
        let Some(item) = s.project.binder.get(id) else { return };
        let text = if let Some(e) = s.project.entries.get(&id) {
            e.body.clone()
        } else if item.kind.has_text() {
            s.project.text(id).map(|t| t.body.clone()).unwrap_or_default()
        } else {
            self.editor = None;
            return;
        };
        let mut ed = editor::Editor::new(id, &text, self.registry.clone());
        ed.smart = typing::Smart::of(&s.project.settings);
        ed.spell = Some(spelling::SpellCache::new(&s.project));
        if self.held_elsewhere(id) {
            ed.read_only = true;
            self.status = "This document is open for editing in the stitched view in the other pane; here it is read-only.".into();
        }
        self.editor = Some(ed);
        self.mark_revisions();
    }

    /// Reload the open editor from the project (after a project-wide change).
    pub fn reload_editor(&mut self) {
        let Some(ed) = self.editor.as_ref() else { return };
        let Some(s) = self.session.as_ref() else { return };
        let fresh = if stitch::is_stitched(ed) {
            let items: Vec<ItemId> = stitch::split(&ed.session.doc).into_iter().map(|(i, _)| i).collect();
            stitch::open(&s.project, &items, self.registry.clone())
        } else {
            let text = s.project.text(ed.item).map(|t| t.body.clone()).unwrap_or_default();
            Some(editor::Editor::new(ed.item, &text, self.registry.clone()))
        };
        if let Some(mut f) = fresh {
            f.smart = typing::Smart::of(&s.project.settings);
            f.spell = Some(spelling::SpellCache::new(&s.project));
            self.editor = Some(f);
        }
    }

    /// Open a stitched view (ED-2): of the given items, of the selected item and what is under
    /// it, or of the whole included manuscript.
    pub fn stitch(&mut self, cmd: &Cmd) -> Result<(), String> {
        self.write_back();
        let s = self.session.as_ref().ok_or("no project is open")?;
        let items: Vec<ItemId> = if cmd.id == "view.stitchManuscript" {
            s.project.included_manuscript().into_iter().filter(|i| s.project.binder.get(*i).is_some_and(|it| stitch::shows(&s.project, it))).collect()
        } else if let Some(list) = cmd.params.get("items").and_then(|v| v.as_array()) {
            list.iter().filter_map(|v| v.as_str()).filter_map(|v| v.parse().ok()).collect()
        } else {
            let root = match cmd.params.get("id").and_then(|v| v.as_str()) {
                Some(id) => id.parse().map_err(|e: wordsmith_manuscript_model::BadId| e.to_string())?,
                None => self.binder.selected.ok_or("select a chapter, part or folder to read as one text")?,
            };
            stitch::items_under(&s.project, root)
        };
        let mut ed = stitch::open(&s.project, &items, self.registry.clone()).ok_or("there is no text to show")?;
        if items.iter().any(|i| self.held_elsewhere(*i)) {
            ed.read_only = true;
        }
        self.editor = Some(ed);
        if let Some(e) = self.editor.as_mut() {
            e.smart = typing::Smart::of(&s.project.settings);
            e.spell = Some(spelling::SpellCache::new(&s.project));
        }
        self.status = format!("{} documents as one text.", items.len());
        self.mark_revisions();
        Ok(())
    }

    /// After a project command: an editor whose item's text changed in the project reloads, and
    /// one whose item left the binder closes.
    fn sync_editor(&mut self) {
        self.sync_editor_text();
        self.mark_revisions();
    }

    fn sync_editor_text(&mut self) {
        let Some(s) = self.session.as_ref() else {
            self.editor = None;
            return;
        };
        let smart = typing::Smart::of(&s.project.settings);
        let words = spelling::project_words(&s.project);
        if let Some(ed) = self.editor.as_mut() {
            ed.smart = smart;
            if let Some(sp) = ed.spell.as_mut() {
                sp.set_words(words);
            }
        }
        let Some(ed) = self.editor.as_ref() else { return };
        if crate::stitch::is_stitched(ed) {
            return;
        }
        let id = ed.item;
        if !s.project.binder.contains(id) {
            self.editor = None;
            return;
        }
        let want = match s.project.entries.get(&id) {
            Some(e) => e.body.clone(),
            None => s.project.text(id).map(|t| t.body.clone()).unwrap_or_default(),
        };
        if !ed.is_dirty() && ed.text() != want {
            let mut fresh = editor::Editor::new(id, &want, self.registry.clone());
            fresh.smart = ed.smart.clone();
            self.editor = Some(fresh);
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
            CommandInfo::new(ids::NEW_FROM_TEMPLATE, "New Project from Template", ""),
            CommandInfo { icon: "open".into(), ..CommandInfo::new(ids::OPEN, "Open Project", "Mod+O") },
            CommandInfo::new(ids::OPEN_PATH, "Open Recent", ""),
            CommandInfo::new(ids::SAVE_AS, "Save As", "Mod+Shift+S"),
            CommandInfo { icon: "undo".into(), ..CommandInfo::new(ids::UNDO, "Undo", "Mod+Z") },
            CommandInfo { icon: "redo".into(), ..CommandInfo::new(ids::REDO, "Redo", "Mod+Shift+Z") },
        ];
        v.push(CommandInfo::new("session.reset", "Start a New Session", ""));
        v.push(CommandInfo::new("sprint.start", "Start a Writing Sprint", ""));
        v.push(CommandInfo::new("filter.set", "Filter", ""));
        v.push(CommandInfo::new("filter.clear", "Show All Items", ""));
        v.push(CommandInfo::new("view.split", "Split Side by Side", "Mod+Alt+\\"));
        v.push(CommandInfo::new("view.splitStacked", "Split Top and Bottom", ""));
        v.push(CommandInfo::new("view.unsplit", "One Pane", ""));
        v.push(CommandInfo::new("view.history", "Writing History", ""));
        v.push(CommandInfo::new("view.entryTypes", "Entry Types", ""));
        v.push(CommandInfo::new("view.offers", "Offered Fixes", ""));
        v.push(CommandInfo::new("view.drafts", "Drafts", ""));
        v.push(CommandInfo::new("view.series", "Series", ""));
        v.push(CommandInfo::new("file.saveTemplate", "Save as Template", ""));
        v.push(CommandInfo::new("write.comment", "Add Comment", "Mod+Alt+M"));
        v.push(CommandInfo::new("view.otherPane", "Go to the Other Pane", "Mod+Alt+O"));
        v.push(CommandInfo::new("view.editor", "Editor", "Mod+1"));
        v.push(CommandInfo::new("view.corkboard", "Corkboard", "Mod+2"));
        v.push(CommandInfo::new("view.outline", "Outline", "Mod+3"));
        v.push(CommandInfo::new("view.board", "Board", "Mod+4"));
        v.push(CommandInfo::new("view.pages", "Pages", "Mod+5"));
        v.push(CommandInfo::new("view.focus", "Focus Mode", "F11"));
        v.push(CommandInfo::new("view.sepia", "Sepia Page", ""));
        v.push(CommandInfo::new("view.typewriter", "Typewriter Scrolling", ""));
        v.push(CommandInfo::new("view.grammar", "Grammar Hints", ""));
        v.push(CommandInfo::new("view.dimAround", "Dim Around the Caret", ""));
        v.push(CommandInfo::new("view.textSize", "Text Size", ""));
        v.push(CommandInfo::new("view.width", "Text Width", ""));
        v.push(CommandInfo::new("find.show", "Find and Replace", "Mod+F"));
        v.push(CommandInfo::new("find.next", "Find Next", "Mod+G"));
        v.push(CommandInfo::new("find.previous", "Find Previous", "Mod+Shift+G"));
        v.push(CommandInfo::new("view.stitch", "Read as One Text", "Mod+Alt+R"));
        v.push(CommandInfo::new("view.stitchManuscript", "Read the Whole Manuscript", "Mod+Alt+Shift+R"));
        for spec in editor::styles::specs() {
            v.push(CommandInfo::new(spec.id, spec.label, spec.key));
        }
        for spec in commands::specs() {
            if spec.id == "edit.undo" || spec.id == "edit.redo" {
                continue;
            }
            v.push(CommandInfo::new(spec.id, spec.label, spec.key));
        }
        for (code, name, _) in i18n::LANGUAGES {
            v.push(CommandInfo::new(format!("view.language.{code}"), format!("{}: {name}", i18n::tr("Interface Language")), ""));
        }
        for c in &mut v {
            c.label = i18n::tr(&c.label);
        }
        v
    }
}

impl App for Manuscript {
    fn document_name(&self) -> String {
        self.project_name()
    }

    fn has_document(&self) -> bool {
        self.session.is_some()
    }

    fn templates(&self) -> Vec<jcj_shell::TemplateSpec> {
        let mut out: Vec<jcj_shell::TemplateSpec> =
            templates::TEMPLATES.iter().map(|t| jcj_shell::TemplateSpec::new(t.id, t.name, t.about).in_group("Fiction")).collect();
        if let Some(dir) = self.templates_dir.as_ref() {
            for t in user_templates::list(dir) {
                out.push(jcj_shell::TemplateSpec::new(user_templates::id_of(&t.slug), t.name, t.about).in_group("Your Templates"));
            }
        }
        out
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

    fn run(&mut self, cmd: &Cmd, shell: &mut Shell) -> Result<(), String> {
        let r = self.run_cmd(cmd, shell);
        self.sync_editor();
        r
    }

    fn document_ui(&mut self, ui: &mut egui::Ui, shell: &mut Shell) {
        self.draw_document(ui, shell);
        if std::mem::take(&mut self.pending_unfocus) && self.focus.is_some() {
            shell.run(Cmd::new("view.focus"));
        }
    }

    fn panels(&self) -> Vec<PanelSpec> {
        if self.focus.is_some() {
            return Vec::new();
        }
        vec![
            PanelSpec::new(BINDER_PANEL, i18n::tr("Binder"), "list", Side::Left),
            PanelSpec::new(INSPECTOR_PANEL, i18n::tr("Inspector"), "info", Side::Right),
        ]
    }

    fn ribbon_tabs(&self) -> Vec<jcj_shell::TabSpec> {
        vec![
            jcj_shell::TabSpec::new("write", i18n::tr("Write"), "W"),
            jcj_shell::TabSpec::new("bible", i18n::tr("Bible"), "B"),
            jcj_shell::TabSpec::new("view", i18n::tr("View"), "V"),
        ]
    }

    fn ribbon_ui(&mut self, tab: &str, ui: &mut egui::Ui, shell: &mut Shell) {
        match tab {
            "write" => ribbon::write_tab(self, ui, shell),
            "view" => ribbon::view_tab(self, ui, shell),
            "bible" => ribbon::bible_tab(self, ui, shell),
            _ => {}
        }
    }

    fn autosave(&mut self) -> Result<(), String> {
        self.write_back();
        let Some(s) = self.session.as_mut() else { return Ok(()) };
        if s.root().is_none() {
            return s.write_recovery();
        }
        s.save()?;
        if s.backup_due(Instant::now()) {
            s.backup()?;
        }
        self.keep_history();
        Ok(())
    }

    fn panel_ui(&mut self, panel: &str, ui: &mut egui::Ui, _shell: &mut Shell) {
        if panel == INSPECTOR_PANEL {
            self.write_back();
            let Some(s) = self.session.as_mut() else {
                ui.label(crate::i18n::tr("No project open."));
                return;
            };
            let rev = s.revision();
            let errors = inspector::panel(ui, s, &mut self.inspector, self.binder.selected);
            if let Some(e) = errors.into_iter().last() {
                self.status = e;
            }
            if self.session.as_ref().is_some_and(|s| s.revision() != rev) {
                self.sync_editor();
            }
            return;
        }
        if panel != BINDER_PANEL {
            return;
        }
        let Some(s) = self.session.as_mut() else {
            ui.label(crate::i18n::tr("No project open."));
            return;
        };
        let before = self.binder.selected;
        let rev = s.revision();
        self.searcher.refresh(&s.project, rev);
        let marks = bookmarks::all(&s.project);
        if !marks.is_empty() {
            let mut go = None;
            ui.horizontal_wrapped(|ui| {
                ui.label(egui::RichText::new("★").weak()).on_hover_text("Bookmarks");
                for m in &marks {
                    let title = s.project.binder.get(*m).map(|i| i.title.clone()).unwrap_or_default();
                    if ui.small_button(&title).on_hover_text("Open").clicked() {
                        go = Some(*m);
                    }
                }
            });
            if let Some(id) = go {
                self.showing = None;
                self.search_results = None;
                self.binder.selected = Some(id);
                self.open_item(id);
                return;
            }
        }
        let cols = collections::all(&s.project);
        if self.showing.as_ref().is_some_and(|n| !cols.iter().any(|c| &c.name == n)) {
            self.showing = None;
        }
        if !cols.is_empty() {
            let c = egui::ComboBox::from_id_salt("manuscript-collections")
                .width(ui.available_width() - 8.0)
                .selected_text(self.showing.clone().unwrap_or_else(|| "Binder".into()))
                .show_ui(ui, |ui| {
                    if ui.selectable_label(self.showing.is_none(), "Binder").clicked() {
                        self.showing = None;
                    }
                    for c in &cols {
                        let label = if c.search.is_some() { format!("{} (search)", c.name) } else { c.name.clone() };
                        if ui.selectable_label(self.showing.as_ref() == Some(&c.name), label).clicked() {
                            self.showing = Some(c.name.clone());
                        }
                    }
                });
            c.response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::ComboBox, true, "Show"));
        }
        if let Some(c) = self.showing.as_ref().and_then(|n| cols.iter().find(|c| &c.name == n)).cloned() {
            let items = collections::items_of(&s.project, &c).unwrap_or_default();
            ui.horizontal(|ui| {
                ui.label(format!("{} items", items.len()));
                if ui.small_button(crate::i18n::tr("Delete Collection")).on_hover_text("The items stay in the binder").clicked() {
                    match s.run("collection.delete", &json!({ "name": c.name })) {
                        Ok(_) => self.showing = None,
                        Err(e) => self.status = e,
                    }
                }
            });
            let mut open = None;
            let mut remove = None;
            egui::ScrollArea::vertical().id_salt("manuscript-collection-items").show(ui, |ui| {
                for id in &items {
                    let title = s.project.binder.get(*id).map(|i| i.title.clone()).unwrap_or_default();
                    ui.horizontal(|ui| {
                        if ui.selectable_label(self.binder.selected == Some(*id), &title).clicked() {
                            open = Some(*id);
                        }
                        if c.search.is_none() && ui.small_button("×").on_hover_text("Take out of the collection").clicked() {
                            remove = Some(*id);
                        }
                    });
                }
            });
            if let Some(id) = remove
                && let Err(e) = s.run("collection.remove", &json!({ "name": c.name, "items": [id.as_text()] }))
            {
                self.status = e;
            }
            if let Some(id) = open {
                self.binder.selected = Some(id);
                self.open_item(id);
            }
            return;
        }
        ui.horizontal(|ui| {
            let r = ui.add(egui::TextEdit::singleline(&mut self.search_text).hint_text("Search the project").desired_width(f32::INFINITY));
            r.widget_info(|| egui::WidgetInfo::labeled(egui::Role::SearchInput, true, "Search the project"));
            if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                if self.search_text.trim().is_empty() {
                    self.search_results = None;
                } else {
                    let q = find::Query { text: self.search_text.clone(), ..find::Query::default() };
                    match self.searcher.search(&s.project, rev, &q) {
                        Ok(found) => {
                            let mut out: Vec<(ItemId, usize)> = Vec::new();
                            for f in found {
                                match out.last_mut() {
                                    Some((i, n)) if *i == f.item => *n += 1,
                                    _ => out.push((f.item, 1)),
                                }
                            }
                            self.search_results = Some(out);
                        }
                        Err(e) => self.status = e,
                    }
                }
            }
        });
        if let Some(results) = self.search_results.clone() {
            ui.horizontal(|ui| {
                ui.label(format!("{} documents", results.len()));
                if ui.small_button("Back to the Binder").clicked() {
                    self.search_results = None;
                }
                if ui.small_button(crate::i18n::tr("Save Search")).on_hover_text("Keep this search as a collection that stays up to date").clicked() {
                    let name = self.search_text.trim().to_owned();
                    match s.run("collection.saveSearch", &json!({ "name": name, "query": name })) {
                        Ok(_) => {
                            self.search_results = None;
                            self.showing = Some(name);
                        }
                        Err(e) => self.status = e,
                    }
                }
            });
            let mut open = None;
            egui::ScrollArea::vertical().id_salt("manuscript-search-results").show(ui, |ui| {
                for (id, n) in &results {
                    let title = s.project.binder.get(*id).map(|i| i.title.clone()).unwrap_or_default();
                    if ui.selectable_label(self.binder.selected == Some(*id), format!("{title} ({n})")).clicked() {
                        open = Some(*id);
                    }
                }
            });
            if let Some(id) = open {
                self.binder.selected = Some(id);
                self.open_item(id);
                self.find.query.text = self.search_text.clone();
                let q = self.find.query.clone();
                if let Some(ed) = self.editor.as_mut() {
                    ed.set_caret(0, 0, false);
                    let _ = ed.find_next(&q, false);
                }
            }
            return;
        }
        if filter::bar(ui, &s.project, &mut self.filter) || self.filtered_at != rev {
            self.binder.hidden = filter::hidden(&s.project, &self.filter);
            self.filtered_at = rev;
        }
        ui.separator();
        let errors = binder::panel(ui, s, &mut self.binder);
        if let Some(e) = errors.into_iter().last() {
            self.status = e;
        }
        if self.binder.selected != before
            && let Some(id) = self.binder.selected
        {
            self.open_item(id);
            self.view_root = self.session.as_ref().and_then(|s| outline::root_of(&s.project, Some(id)));
        }
        if self.session.as_ref().is_some_and(|s| s.revision() != rev) {
            self.sync_editor();
        }
    }

    fn status_ui(&mut self, ui: &mut egui::Ui, _shell: &mut Shell) {
        if let Some(ed) = &self.editor {
            let (doc, sel) = ed.counts();
            if sel > 0 {
                ui.label(format!("{} of {} words", binder::format_words(sel), binder::format_words(doc)));
            } else {
                ui.label(format!("{} words", binder::format_words(doc)));
            }
            ui.separator();
        }
        if self.session.is_some() {
            self.start_progress();
            if let (Some(pr), Some(s)) = (self.progress.as_mut(), self.session.as_ref()) {
                pr.observe(s);
                pr.ui(ui, &s.project);
            }
        }
        if self.offers_pending > 0 {
            ui.separator();
            let n = self.offers_pending;
            if ui
                .small_button(format!("{n} offered fix{}", if n == 1 { "" } else { "es" }))
                .on_hover_text("Fixes Wordsmith Book offers back")
                .clicked()
            {
                self.offers_view.open = true;
            }
        }
        if !self.status.is_empty() {
            ui.separator();
            ui.label(&self.status);
        }
    }

    fn file_pages(&self) -> Vec<PageSpec> {
        vec![PageSpec::new("import", "Import", "import"), PageSpec::new("targets", "Targets", "target")]
    }

    fn file_page_ui(&mut self, page: &Page, ui: &mut egui::Ui, shell: &mut Shell) {
        match page {
            Page::New => {
                ui.label(crate::i18n::tr("Folder for the new project (it must not exist yet):"));
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
                ui.label(crate::i18n::tr("A project folder (.wsm), its project.toml, or a packed project (.wsmz):"));
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
                ui.label(crate::i18n::tr("Save a copy as a new project folder, or as one packed file (.wsmz):"));
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
            Page::App(id) if id == "import" => self.import_page(ui),
            Page::Export => self.export_page(ui),
            Page::App(id) if id == "targets" => self.targets_page(ui),
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

impl Manuscript {
    fn run_cmd(&mut self, cmd: &Cmd, shell: &mut Shell) -> Result<(), String> {
        if cmd.id == "filter.set" || cmd.id == "filter.clear" {
            let list = |k: &str| {
                cmd.params
                    .get(k)
                    .and_then(|v| v.as_array())
                    .map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_owned)).collect())
                    .unwrap_or_default()
            };
            self.filter = if cmd.id == "filter.clear" {
                filter::Filter::default()
            } else {
                filter::Filter { labels: list("labels"), statuses: list("statuses"), keywords: list("keywords"), povs: list("povs") }
            };
            if let Some(s) = self.session.as_ref() {
                self.binder.hidden = filter::hidden(&s.project, &self.filter);
            }
            return Ok(());
        }
        if cmd.id == "session.reset" {
            if let Some(p) = self.progress.as_mut() {
                p.writing.reset();
            }
            return Ok(());
        }
        if cmd.id == "sprint.start" {
            let minutes = cmd.params.get("minutes").and_then(|m| m.as_u64()).unwrap_or(15).clamp(1, 240);
            self.start_progress();
            if let Some(p) = self.progress.as_mut() {
                p.writing.start_sprint(std::time::Duration::from_secs(minutes * 60), Instant::now());
            }
            return Ok(());
        }
        if let Some(r) = self.view_command(cmd, shell) {
            return r;
        }
        match cmd.id.as_str() {
            "view.editor" => {
                self.mode = ViewMode::Editor;
                return Ok(());
            }
            "view.outline" => {
                self.write_back();
                self.mode = ViewMode::Outline;
                return Ok(());
            }
            "view.corkboard" => {
                self.write_back();
                self.mode = ViewMode::Corkboard;
                return Ok(());
            }
            "view.board" => {
                self.write_back();
                self.mode = ViewMode::Board;
                return Ok(());
            }
            "view.pages" => {
                self.write_back();
                self.mode = ViewMode::Pages;
                return Ok(());
            }
            _ => {}
        }
        if cmd.id == "file.saveTemplate" {
            let dir = self.templates_dir.clone().ok_or("templates cannot be kept on this computer")?;
            let s = self.session.as_ref().ok_or("open a project to make a template of it")?;
            match cmd.params.get("name").and_then(|n| n.as_str()) {
                Some(name) => {
                    let about = cmd.params.get("about").and_then(|n| n.as_str()).unwrap_or("");
                    self.write_back();
                    let s = self.session.as_ref().ok_or("no project is open")?;
                    user_templates::save(&dir, name, about, &s.project)?;
                    self.status = format!("Kept “{}” as a template: File › New offers it.", name.trim());
                }
                None => self.save_template = Some((s.project.settings.book.title.clone(), String::new())),
            }
            return Ok(());
        }
        if cmd.id == "file.deleteTemplate" {
            let dir = self.templates_dir.clone().ok_or("templates cannot be kept on this computer")?;
            let t = cmd.params.get("template").and_then(|n| n.as_str()).ok_or("`template` is required")?;
            user_templates::delete(&dir, t.strip_prefix("user:").unwrap_or(t))?;
            return Ok(());
        }
        if let Some(code) = cmd.id.strip_prefix("view.language.") {
            self.view.language = code.to_owned();
            i18n::set_language(code);
            return Ok(());
        }
        if cmd.id == "view.series" {
            self.write_back();
            self.series_view.open = true;
            return Ok(());
        }
        if cmd.id == "view.drafts" {
            self.write_back();
            self.drafts_view.open = true;
            return Ok(());
        }
        if cmd.id == "view.offers" {
            self.offers_view.open = true;
            return Ok(());
        }
        if cmd.id == "view.entryTypes" {
            self.entry_types.open = true;
            return Ok(());
        }
        if cmd.id == "view.history" {
            self.start_progress();
            self.history_view.open = true;
            return Ok(());
        }
        if cmd.id == "write.comment" {
            return self.add_comment();
        }
        if cmd.id == "view.otherPane" {
            if self.split == jcj_shell::split::Split::None {
                return Err("the editor is not split".into());
            }
            self.switch_pane();
            return Ok(());
        }
        if matches!(cmd.id.as_str(), "view.split" | "view.splitStacked" | "view.unsplit") {
            self.write_back();
            self.split = match cmd.id.as_str() {
                "view.split" => jcj_shell::split::Split::Side,
                "view.splitStacked" => jcj_shell::split::Split::Stacked,
                _ => jcj_shell::split::Split::None,
            };
            if self.split == jcj_shell::split::Split::None {
                self.one_pane();
            }
            return Ok(());
        }
        if cmd.id == "find.show" {
            self.find.show();
            return Ok(());
        }
        if cmd.id == "find.next" || cmd.id == "find.previous" {
            let q = self.find.query.clone();
            let ed = self.editor.as_mut().ok_or("open a document first")?;
            ed.find_next(&q, cmd.id == "find.previous")?;
            return Ok(());
        }
        if cmd.id == "view.stitch" || cmd.id == "view.stitchManuscript" {
            return self.stitch(cmd);
        }
        if cmd.id.starts_with("write.") {
            let ed = self.editor.as_mut().ok_or("open a document to style its text")?;
            return ed.command(&cmd.id, &cmd.params);
        }
        match cmd.id.as_str() {
            ids::SAVE => {
                self.write_back();
                let s = self.session.as_mut().ok_or("no project is open")?;
                s.save()?;
                self.status = "Saved.".into();
                self.keep_history();
                Ok(())
            }
            ids::NEW => {
                shell.open_file_page(Page::New);
                Ok(())
            }
            ids::NEW_FROM_TEMPLATE => {
                let template = cmd.params.get("template").and_then(|t| t.as_str()).unwrap_or("novel").to_owned();
                let path = cmd.params.get("path").and_then(|p| p.as_str()).map_or_else(|| self.new_project_path(&template), PathBuf::from);
                self.new_from_template(&template, &path)?;
                shell.add_recent(&path.to_string_lossy());
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
            ids::UNDO if self.editor_focused && self.editor.is_some() => {
                if let Some(ed) = self.editor.as_mut() {
                    ed.run("edit.undo", &json!({}))?;
                }
                Ok(())
            }
            ids::REDO if self.editor_focused && self.editor.is_some() => {
                if let Some(ed) = self.editor.as_mut() {
                    ed.run("edit.redo", &json!({}))?;
                }
                Ok(())
            }
            ids::UNDO => {
                self.write_back();
                let s = self.session.as_mut().ok_or("no project is open")?;
                self.status = s.undo().map_or("Nothing to undo.".into(), |l| format!("Undid {l}."));
                Ok(())
            }
            ids::REDO => {
                self.write_back();
                let s = self.session.as_mut().ok_or("no project is open")?;
                self.status = s.redo().map_or("Nothing to redo.".into(), |l| format!("Redid {l}."));
                Ok(())
            }
            "binder.emptyTrash" if cmd.params.get("confirm").is_none() => {
                self.binder.confirm_empty = true;
                Ok(())
            }
            id => {
                self.write_back();
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
                    && let Some(sel) = self.binder.selected
                {
                    params["after"] = json!(sel.as_text());
                }
                let out = s.run(id, &params)?;
                if let Some(new) = out.get("id").and_then(|v| v.as_str()).and_then(|v| v.parse::<ItemId>().ok()) {
                    self.binder.selected = Some(new);
                    let title = self.session.as_ref().and_then(|s| s.project.binder.get(new)).map(|i| i.title.clone()).unwrap_or_default();
                    self.binder.tree.rename(binder::Node::Item(new), &title);
                    self.open_item(new);
                }
                Ok(())
            }
        }
    }
}

impl Manuscript {
    fn draw_document(&mut self, ui: &mut egui::Ui, _shell: &mut Shell) {
        if self.outside_checked.is_none_or(|t| t.elapsed() >= OUTSIDE_EVERY) {
            self.outside_checked = Some(Instant::now());
            self.check_outside();
            self.offers_pending = self.session.as_ref().map_or(0, offers::pending);
        }
        ui.ctx().request_repaint_after(OUTSIDE_EVERY);
        self.mark_revisions();
        for ed in [self.editor.as_mut(), self.other.as_mut()].into_iter().flatten() {
            if let Some(sp) = ed.spell.as_mut() {
                sp.grammar = self.view.grammar;
            }
        }
        if let Some(ask) = ui.ctx().data_mut(|d| d.remove_temp::<(ItemId, String)>(egui::Id::unique("manuscript-compare-ask"))) {
            self.write_back();
            self.compare.open = Some(ask);
        }
        if let Some(s) = self.session.as_ref() {
            snapshots::compare_window(ui.ctx(), s, &mut self.compare);
        }
        if let Some(p) = self.progress.as_ref() {
            history::window(ui.ctx(), &p.writing.history, &mut self.history_view, progress::today());
        }
        self.save_template_ui(ui.ctx(), _shell);
        if let Some(s) = self.session.as_mut() {
            let rev = s.revision();
            let (errors, open) = offers::window(ui.ctx(), s, &mut self.offers_view);
            if let Some(e) = errors.into_iter().last() {
                self.status = e;
            }
            if self.session.as_ref().is_some_and(|s| s.revision() != rev) {
                self.sync_editor();
                self.offers_pending = self.session.as_ref().map_or(0, offers::pending);
            }
            if let Some(id) = open {
                self.binder.selected = Some(id);
                self.open_item(id);
                self.mode = ViewMode::Editor;
            }
        }
        if let Some(s) = self.session.as_mut() {
            let rev = s.revision();
            if let Some(e) = drafts::window(ui.ctx(), s, &mut self.drafts_view).into_iter().last() {
                self.status = e;
            }
            if self.session.as_ref().is_some_and(|s| s.revision() != rev) {
                self.sync_editor();
            }
        }
        let mut book_to_open = None;
        if let Some(s) = self.session.as_mut() {
            let rev = s.revision();
            let (errors, book) = series::window(ui.ctx(), s, &mut self.series_view);
            if let Some(e) = errors.into_iter().last() {
                self.status = e;
            }
            if self.session.as_ref().is_some_and(|s| s.revision() != rev) {
                self.sync_editor();
            }
            book_to_open = book;
        }
        if let Some(book) = book_to_open {
            if let Err(e) = self.open(&book) {
                self.status = e;
            }
            self.series_view.open = true;
        }
        if std::mem::take(&mut self.binder.open_types) {
            self.entry_types.open = true;
        }
        if let Some(s) = self.session.as_mut() {
            let rev = s.revision();
            if let Some(e) = entry_types::window(ui.ctx(), s, &mut self.entry_types).into_iter().last() {
                self.status = e;
            }
            if self.session.as_ref().is_some_and(|s| s.revision() != rev) {
                self.entry = Default::default();
            }
        }
        if self.recovered.is_some() {
            ui.horizontal(|ui| {
                ui.label(crate::i18n::tr("Unsaved work from a session that did not end cleanly was found."));
                if ui.button("Recover It").clicked() {
                    self.accept_recovered();
                }
                if ui.button("Keep the Saved Version").clicked() {
                    self.discard_recovered();
                }
            });
            ui.separator();
        }
        if self.session.is_none() {
            ui.centered_and_justified(|ui| ui.label(crate::i18n::tr("No project open. Use File › New or File › Open.")));
            return;
        }
        if let Some(f) = self.fullscreen.take() {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Fullscreen(f));
        }
        if self.focus.is_some() && ui.input(|i| i.key_pressed(egui::Key::Escape)) && self.editor.as_ref().is_none_or(|e| e.menu.is_none()) {
            self.leave_focus_now();
        }
        let typewriter = self.view.typewriter || self.focus.is_some();
        self.look = self.view.look(ui.visuals().dark_mode, self.focus.is_some());
        if let Some(ed) = self.editor.as_mut() {
            ed.typewriter = typewriter;
        }
        match self.mode {
            ViewMode::Outline => {
                self.outline_ui(ui);
                return;
            }
            ViewMode::Corkboard => {
                self.corkboard_ui(ui);
                return;
            }
            ViewMode::Board => {
                self.board_ui(ui);
                return;
            }
            ViewMode::Pages => {
                self.write_back();
                match self.session.as_ref() {
                    Some(s) => pages::view(ui, s, &mut self.pages, self.binder.selected),
                    None => {
                        ui.centered_and_justified(|ui| ui.label(crate::i18n::tr("Open a project to see its pages.")));
                    }
                }
                return;
            }
            ViewMode::Editor => {}
        }
        if self.focus.is_none() {
            findbar::ui(self, ui);
        }
        let entry = self.editor.as_ref().map(|e| e.item).filter(|i| self.session.as_ref().is_some_and(|s| s.project.entries.contains_key(i)));
        if let Some(eid) = entry
            && self.focus.is_none()
        {
            self.write_back();
            let mut asks = Vec::new();
            if let Some(s) = self.session.as_mut() {
                egui::ScrollArea::vertical().id_salt("manuscript-entry-form").max_height(ui.available_height() * 0.5).show(ui, |ui| {
                    let (a, errors) = bible::form(ui, s, &mut self.entry, eid);
                    asks = a;
                    if let Some(e) = errors.into_iter().last() {
                        self.status = e;
                    }
                });
            }
            ui.separator();
            self.sync_editor();
            for bible::Ask::Open(d) in asks {
                self.binder.selected = Some(d);
                self.binder.tree.reveal = true;
                self.open_item(d);
            }
        }
        if self.split != jcj_shell::split::Split::None && self.focus.is_none() {
            self.split_panes(ui);
            return;
        }
        self.editor_pane(ui);
    }

    /// The active editor, its outcomes handled (errors, words added, links followed and previewed).
    fn editor_pane(&mut self, ui: &mut egui::Ui) {
        match self.editor.as_mut() {
            Some(ed) => {
                let resp = ed.ui(ui, &self.look);
                ed.menu_ui(ui.ctx());
                self.editor_focused = resp.has_focus() || (self.editor_focused && !ui.input(|i| i.pointer.any_pressed()));
                self.after_editor(ui);
            }
            None => {
                let shown = self.binder.selected.filter(|id| self.session.as_ref().and_then(|s| s.project.binder.get(*id)).is_some_and(media::shows));
                match (shown, self.session.as_mut()) {
                    (Some(id), Some(s)) => {
                        if let Some(e) = media::view(ui, s, &mut self.media_view, id) {
                            self.status = e;
                        }
                    }
                    _ => {
                        ui.centered_and_justified(|ui| ui.label(crate::i18n::tr("Select a document in the binder to write in it.")));
                    }
                }
            }
        }
    }

    fn after_editor(&mut self, ui: &egui::Ui) {
        let Some(ed) = self.editor.as_mut() else { return };
        if let Some(e) = ed.error.take() {
            self.status = e;
        }
        let word = ed.add_word.take();
        let follow = ed.follow.take();
        let hover = ed.hover_link.clone();
        if let Some(w) = word {
            let r = self.session.as_mut().map(|s| s.run("spelling.addWord", &json!({ "word": w })).map(|_| ()));
            if let Some(Err(e)) = r {
                self.status = e;
            }
            self.sync_editor();
        }
        if let Some((target, at)) = hover {
            self.link_preview(ui, &target, at);
        }
        if let Some(target) = follow {
            self.follow_link(&target);
        }
    }

    /// Two editors (ED-6): the active pane takes the binder's selection; a click makes the other
    /// one active.
    fn split_panes(&mut self, ui: &mut egui::Ui) {
        let title = |e: &Option<editor::Editor>, s: &Option<Session>| {
            e.as_ref()
                .and_then(|e| {
                    s.as_ref()
                        .and_then(|s| s.project.binder.get(e.item))
                        .map(|i| if stitch::is_stitched(e) { format!("{} and after", i.title) } else { i.title.clone() })
                })
                .unwrap_or_else(|| "Empty".into())
        };
        let (t0, t1) = if self.active_pane == 0 {
            (title(&self.editor, &self.session), title(&self.other, &self.session))
        } else {
            (title(&self.other, &self.session), title(&self.editor, &self.session))
        };
        match jcj_shell::split::tabs(ui, ui.make_persistent_id("manuscript-panes"), &[t0, t1], self.active_pane) {
            Some(jcj_shell::split::TabEvent::Select(i)) if i != self.active_pane => self.switch_pane(),
            Some(jcj_shell::split::TabEvent::Close(i)) => {
                self.write_back();
                if i == self.active_pane {
                    self.editor = self.other.take();
                }
                self.one_pane();
                return;
            }
            _ => {}
        }
        let look = self.look.clone();
        let active = self.active_pane;
        let (first, second) = if active == 0 { (&mut self.editor, &mut self.other) } else { (&mut self.other, &mut self.editor) };
        let mut state = self.split_state;
        let draw = |ui: &mut egui::Ui, e: &mut Option<editor::Editor>| match e.as_mut() {
            Some(ed) => {
                ed.ui(ui, &look);
                ed.menu_ui(ui.ctx());
            }
            None => {
                ui.centered_and_justified(|ui| ui.label(crate::i18n::tr("Select a document in the binder to open it here.")));
            }
        };
        let clicked = jcj_shell::split::split(
            ui,
            ui.make_persistent_id("manuscript-split"),
            self.split,
            &mut state,
            |ui| draw(ui, first),
            |ui| draw(ui, second),
        );
        self.split_state = state;
        if let Some(c) = clicked
            && c != self.active_pane
        {
            self.switch_pane();
        }
        self.editor_focused = true;
        self.after_editor(ui);
    }

    /// Anchor a new comment to the editor's selection and open it in the inspector to write.
    pub fn add_comment(&mut self) -> Result<(), String> {
        let ed = self.editor.as_ref().ok_or("open a document to comment on")?;
        if ed.read_only {
            return Err("this document is read-only here".into());
        }
        if ed.session.sel.is_collapsed() {
            return Err("select the words to comment on first".into());
        }
        let (a, _) = ed.session.sel.ordered();
        let item = stitch::item_at(ed, editor::para_index(&a));
        self.write_back();
        let key = comments::fresh_id(&self.session.as_ref().ok_or("no project is open")?.project, item);
        if let Some(ed) = self.editor.as_mut() {
            editor::styles::mark_comment(ed, &key)?;
        }
        self.write_back();
        if let Some(s) = self.session.as_mut() {
            s.run("comment.set", &json!({ "id": item.as_text(), "comment": key, "text": "" }))?;
        }
        self.inspector.focus_comment = Some(key);
        self.binder.selected = Some(item);
        self.status = "Comment added: write it in the inspector.".into();
        Ok(())
    }

    fn board_ui(&mut self, ui: &mut egui::Ui) {
        let Some(s) = self.session.as_mut() else {
            ui.centered_and_justified(|ui| ui.label(crate::i18n::tr("Open a project to plan on its board.")));
            return;
        };
        let rev = s.revision();
        let (asks, errors) = board::view(ui, s, &mut self.board, self.binder.selected);
        if let Some(e) = errors.into_iter().last() {
            self.status = e;
        }
        if self.session.as_ref().is_some_and(|s| s.revision() != rev) {
            self.sync_editor();
        }
        for a in asks {
            let board::Ask::Open(id) = a;
            self.binder.selected = Some(id);
            self.open_item(id);
            self.mode = ViewMode::Editor;
        }
    }

    fn save_template_ui(&mut self, ctx: &egui::Context, shell: &mut Shell) {
        let Some((name, about)) = self.save_template.as_mut() else { return };
        let mut go = false;
        let mut cancel = false;
        egui::Window::new(i18n::tr("Save as Template")).collapsible(false).resizable(false).anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0]).show(ctx, |ui| {
            ui.label(crate::i18n::tr("A template keeps this project's binder, lists, fields, entry types, targets, notes and front matter. The book's own text and its bible entries are not kept."));
            let l = ui.label(crate::i18n::tr("Name"));
            ui.add(egui::TextEdit::singleline(name).desired_width(320.0)).labelled_by(l.id);
            let l = ui.label(crate::i18n::tr("What it is for"));
            ui.add(egui::TextEdit::singleline(about).desired_width(320.0)).labelled_by(l.id);
            ui.horizontal(|ui| {
                go = ui.button("Save Template").clicked();
                cancel = ui.button("Cancel").clicked();
            });
        });
        if go {
            let (n, a) = (name.clone(), about.clone());
            match self.run(&Cmd::with("file.saveTemplate", json!({ "name": n, "about": a })), shell) {
                Ok(()) => self.save_template = None,
                Err(e) => self.status = e,
            }
        } else if cancel {
            self.save_template = None;
        }
    }

    /// Make the other pane the active one.
    pub fn switch_pane(&mut self) {
        self.write_back();
        std::mem::swap(&mut self.editor, &mut self.other);
        self.active_pane = 1 - self.active_pane;
        // A read-only pane shows what the other pane has written since.
        if self.editor.as_ref().is_some_and(|e| e.read_only) {
            self.reload_editor();
            if let Some(e) = self.editor.as_mut() {
                e.read_only = true;
            }
        }
        if let Some(id) = self.editor.as_ref().map(|e| e.item) {
            self.binder.selected = Some(id);
        }
    }
}

impl Manuscript {
    /// The title and synopsis a link points at, for its preview (ED-10).
    pub fn link_target(&self, target: &str) -> Option<(String, String)> {
        let s = self.session.as_ref()?;
        let id: ItemId = target.strip_prefix("wsm:")?.parse().ok()?;
        let item = s.project.binder.get(id);
        let Some(item) = item else { return Some(("A deleted item".into(), "This link points at an item no longer in the project.".into())) };
        let synopsis = match s.project.entries.get(&id) {
            Some(e) => e.body.plain_paragraphs().into_iter().take(2).collect::<Vec<_>>().join(" "),
            None => s.project.text(id).map(|t| t.meta.synopsis.clone()).unwrap_or_default(),
        };
        Some((item.title.clone(), synopsis))
    }

    fn link_preview(&self, ui: &egui::Ui, target: &str, at: egui::Pos2) {
        let (title, about) = self.link_target(target).unwrap_or_else(|| (target.to_owned(), String::new()));
        egui::Area::new(egui::Id::unique("manuscript-link-preview")).order(egui::Order::Tooltip).fixed_pos(at + egui::vec2(12.0, 16.0)).show(
            ui.ctx(),
            |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.set_max_width(320.0);
                    ui.strong(title);
                    if !about.is_empty() {
                        ui.label(about);
                    }
                });
            },
        );
    }

    /// Follow a link: an item opens in the editor; other addresses are left to the system.
    pub fn follow_link(&mut self, target: &str) {
        let Some(id) = target.strip_prefix("wsm:").and_then(|t| t.parse::<ItemId>().ok()) else {
            self.status = format!("Link to {target}");
            return;
        };
        if self.session.as_ref().is_some_and(|s| s.project.binder.contains(id)) {
            self.binder.selected = Some(id);
            self.binder.tree.reveal = true;
            self.open_item(id);
        } else {
            self.status = "This link points at an item no longer in the project.".into();
        }
    }
}

impl Manuscript {
    fn view_command(&mut self, cmd: &Cmd, shell: &mut Shell) -> Option<Result<(), String>> {
        let num = |k: &str| cmd.params.get(k).and_then(|v| v.as_f64()).map(|v| v as f32);
        match cmd.id.as_str() {
            "view.focus" => {
                match self.focus.take() {
                    Some(chrome) => {
                        shell.options.ribbon = chrome.ribbon;
                        shell.options.quick_row = chrome.quick_row;
                        shell.options.status_bar = chrome.status_bar;
                        self.fullscreen = Some(false);
                    }
                    None => {
                        self.focus = Some(view::Chrome {
                            ribbon: shell.options.ribbon,
                            quick_row: shell.options.quick_row,
                            status_bar: shell.options.status_bar,
                        });
                        shell.options.ribbon = false;
                        shell.options.quick_row = false;
                        shell.options.status_bar = false;
                        self.fullscreen = Some(true);
                        self.find.open = false;
                    }
                }
                Some(Ok(()))
            }
            "view.sepia" => {
                self.view.surface =
                    if self.view.surface == brand::EditorSurface::Sepia { brand::EditorSurface::Theme } else { brand::EditorSurface::Sepia };
                self.relayout();
                Some(Ok(()))
            }
            "view.typewriter" => {
                self.view.typewriter = !self.view.typewriter;
                Some(Ok(()))
            }
            "view.grammar" => {
                self.view.grammar = !self.view.grammar;
                self.status = if self.view.grammar { "Grammar hints are on.".into() } else { "Grammar hints are off.".into() };
                Some(Ok(()))
            }
            "view.dimAround" => {
                self.view.dim_in_focus = !self.view.dim_in_focus;
                Some(Ok(()))
            }
            "view.textSize" => {
                let size = num("size").unwrap_or(self.view.size + num("by").unwrap_or(1.0));
                self.view = view::ViewPrefs { size, ..self.view.clone() }.bounded();
                self.relayout();
                Some(Ok(()))
            }
            "view.width" => {
                let width = num("width").unwrap_or(self.view.width + num("by").unwrap_or(40.0));
                self.view = view::ViewPrefs { width, ..self.view.clone() }.bounded();
                self.relayout();
                Some(Ok(()))
            }
            _ => None,
        }
    }

    /// Leave focus mode from inside the frame (Escape).
    fn leave_focus_now(&mut self) {
        // The shell's options are put back the next time a command reaches the app; Escape queues one.
        self.pending_unfocus = true;
    }

    fn relayout(&mut self) {
        if let Some(ed) = self.editor.as_mut() {
            ed.relayout();
        }
    }
}

impl Manuscript {
    fn apply_asks(&mut self, asks: Vec<outline::Ask>) {
        for a in asks {
            match a {
                outline::Ask::Select(id) => {
                    self.binder.selected = Some(id);
                    self.binder.tree.reveal = true;
                }
                outline::Ask::Open(id) => {
                    self.binder.selected = Some(id);
                    self.open_item(id);
                    self.mode = ViewMode::Editor;
                }
            }
        }
    }

    fn outline_ui(&mut self, ui: &mut egui::Ui) {
        let Some(s) = self.session.as_mut() else { return };
        let root = self.view_root.filter(|r| s.project.binder.contains(*r));
        let hidden = self.binder.hidden.clone();
        let (asks, errors) = outline::view(ui, s, &mut self.outline, root, &hidden, self.binder.selected);
        if let Some(e) = errors.into_iter().last() {
            self.status = e;
        }
        self.apply_asks(asks);
        self.sync_editor();
    }

    fn corkboard_ui(&mut self, ui: &mut egui::Ui) {
        let Some(s) = self.session.as_mut() else { return };
        let root = self.view_root.filter(|r| s.project.binder.contains(*r));
        let hidden = self.binder.hidden.clone();
        let (asks, errors) = cork::view(ui, s, &mut self.cork, root, &hidden, self.binder.selected);
        if let Some(e) = errors.into_iter().last() {
            self.status = e;
        }
        self.apply_asks(asks);
        self.sync_editor();
    }
}

impl Manuscript {
    fn import_page(&mut self, ui: &mut egui::Ui) {
        ui.heading("Import");
        ui.label(crate::i18n::tr("A file or folder to bring into the open project:"));
        let l = ui.label(crate::i18n::tr("Path"));
        ui.text_edit_singleline(&mut self.import_path).labelled_by(l.id);
        ui.checkbox(&mut self.import_rules.split_scenes, "Scene breaks split chapters into scenes");
        ui.checkbox(&mut self.import_rules.chapter_lines, "Lines such as “Chapter 12” start chapters");
        ui.checkbox(&mut self.import_rules.underline_as_italic, "Underlined text becomes italic");
        ui.checkbox(&mut self.import_join, "Plain text: lines without a blank line between are one paragraph");
        let mut run: Option<&str> = None;
        ui.horizontal_wrapped(|ui| {
            if ui.button("Preview").clicked() {
                run = Some("import.preview");
            }
            if ui.button("Import into the Manuscript").clicked() {
                run = Some("import.document");
            }
            if ui.button("Document Set (a Folder)").clicked() {
                run = Some("import.set");
            }
            if ui.button("Notes Folder into the Story Bible").clicked() {
                run = Some("import.notes");
            }
        });
        if let Some(cmd) = run {
            self.write_back();
            let path = self.import_path.clone();
            let params = json!({ "path": path, "folder": path, "rules": self.import_rules, "join_lines": self.import_join });
            self.import_report = match self.session.as_mut() {
                None => "Open or create a project first.".into(),
                Some(s) => match s.run(cmd, &params) {
                    Ok(v) if cmd == "import.preview" => preview_text(&v),
                    Ok(v) => serde_json::to_string_pretty(&v).unwrap_or_default(),
                    Err(e) => e,
                },
            };
            self.sync_editor();
        }
        if !self.import_report.is_empty() {
            ui.separator();
            ui.label(crate::i18n::tr("Report"));
            egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| ui.monospace(&self.import_report));
        }
    }
}

/// A preview as an indented list with the checks.
fn preview_text(v: &serde_json::Value) -> String {
    let mut out = String::new();
    for i in v["items"].as_array().into_iter().flatten() {
        let depth = i["depth"].as_u64().unwrap_or(0) as usize;
        out.push_str(&format!(
            "{}{} ({}, {} words)\n",
            "    ".repeat(depth),
            i["title"].as_str().unwrap_or(""),
            i["role"].as_str().unwrap_or(""),
            i["words"]
        ));
    }
    out.push_str(&format!("\nWord check: {}\nNotes: {}\nNot kept: {}\n", v["word_check"], v["notes_check"], v["dropped"]));
    out
}

impl Manuscript {
    fn export_page(&mut self, ui: &mut egui::Ui) {
        use wordsmith_manuscript_io::compile::Titles;
        ui.label(crate::i18n::tr("Write the manuscript to a file. Export never changes the project."));
        let l = ui.label(crate::i18n::tr("File"));
        ui.text_edit_singleline(&mut self.export_path).labelled_by(l.id);
        let formats = [
            ("manuscript-docx", "Standard manuscript (DOCX)"),
            ("manuscript-pdf", "Standard manuscript (PDF)"),
            ("docx", "Word processor (DOCX)"),
            ("odt", "OpenDocument (ODT)"),
            ("rtf", "Rich text (RTF)"),
            ("markdown", "Markdown"),
            ("text", "Plain text"),
            ("html", "Web page (HTML)"),
            ("epub", "Reading copy (EPUB)"),
            ("book", "Book package for Wordsmith Book (.wsmz)"),
        ];
        let current = formats.iter().find(|f| f.0 == self.export_format).map_or("", |f| f.1);
        let c = egui::ComboBox::from_id_salt("export-format").selected_text(current).show_ui(ui, |ui| {
            for (k, label) in formats {
                ui.selectable_value(&mut self.export_format, k.to_owned(), label);
            }
        });
        c.response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::ComboBox, true, "Format"));
        let o = &mut self.export_options;
        let titles = [
            (Titles::Title, "Their titles"),
            (Titles::Numbered, "Chapter 1"),
            (Titles::Words, "Chapter One"),
            (Titles::WordOnly, "One"),
            (Titles::NumberedWithTitle, "Chapter 1: Title"),
            (Titles::None, "No heading"),
        ];
        let pick = |ui: &mut egui::Ui, label: &str, v: &mut Titles| {
            ui.horizontal(|ui| {
                ui.label(label);
                let name = titles.iter().find(|t| t.0 == *v).map_or("", |t| t.1);
                egui::ComboBox::from_id_salt(label).selected_text(name).show_ui(ui, |ui| {
                    for (t, n) in titles {
                        ui.selectable_value(v, t, n);
                    }
                });
            });
        };
        pick(ui, "Chapters", &mut o.chapters);
        pick(ui, "Parts", &mut o.parts);
        ui.checkbox(&mut o.scene_titles, "Scene titles as headings");
        ui.horizontal(|ui| {
            let l = ui.label(crate::i18n::tr("Scene break"));
            ui.add(egui::TextEdit::singleline(&mut o.scene_break).desired_width(60.0)).labelled_by(l.id);
        });
        ui.checkbox(&mut o.front_matter, "Front matter");
        ui.checkbox(&mut o.back_matter, "Back matter");
        ui.checkbox(&mut o.end_mark, "END after the last line");
        ui.checkbox(&mut o.comments, "Comments as review comments (DOCX, ODT, RTF)");
        if ui.button(crate::i18n::tr("Export")).clicked() {
            self.write_back();
            let params = json!({ "path": self.export_path, "format": self.export_format, "options": self.export_options });
            self.export_report = match self.session.as_mut() {
                None => "Open a project first.".into(),
                Some(s) => match s.run("export.file", &params) {
                    Ok(v) => serde_json::to_string_pretty(&v).unwrap_or_default(),
                    Err(e) => e,
                },
            };
        }
        if !self.export_report.is_empty() {
            ui.separator();
            ui.monospace(&self.export_report);
        }
    }
}

impl Manuscript {
    fn targets_page(&mut self, ui: &mut egui::Ui) {
        ui.heading("Targets");
        let Some(s) = self.session.as_mut() else {
            ui.label(crate::i18n::tr("Open a project first."));
            return;
        };
        let t = s.project.settings.targets.clone();
        let mut manuscript = t.manuscript.map(|n| n.to_string()).unwrap_or_default();
        let mut session = t.session.map(|n| n.to_string()).unwrap_or_default();
        let mut deadline = t.deadline.clone();
        let mut days: Vec<bool> = wordsmith_manuscript_model::DAYS.iter().map(|d| t.writing_days.iter().any(|x| x == d)).collect();
        let mut changed = false;
        egui::Grid::new("targets").num_columns(2).show(ui, |ui| {
            let l = ui.label(crate::i18n::tr("Manuscript (words)"));
            changed |= ui.text_edit_singleline(&mut manuscript).labelled_by(l.id).lost_focus();
            ui.end_row();
            let l = ui.label(crate::i18n::tr("Deadline (YYYY-MM-DD)"));
            changed |= ui.text_edit_singleline(&mut deadline).labelled_by(l.id).lost_focus();
            ui.end_row();
            ui.label(crate::i18n::tr("Writing days"));
            ui.horizontal(|ui| {
                for (d, on) in ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"].iter().zip(days.iter_mut()) {
                    changed |= ui.checkbox(on, *d).changed();
                }
            });
            ui.end_row();
            let l = ui.label(crate::i18n::tr("This session (words)"));
            changed |= ui.text_edit_singleline(&mut session).labelled_by(l.id).lost_focus();
            ui.end_row();
        });
        if changed {
            let num = |s: &str| s.trim().replace(',', "").parse::<u64>().ok();
            let writing_days: Vec<&str> = wordsmith_manuscript_model::DAYS.iter().zip(&days).filter(|(_, on)| **on).map(|(d, _)| *d).collect();
            let params =
                json!({ "manuscript": num(&manuscript), "session": num(&session), "deadline": deadline.trim(), "writing_days": writing_days });
            if let Err(e) = s.run("targets.set", &params) {
                self.status = e;
            }
        }
        let c = progress::manuscript_counts(&s.project);
        ui.separator();
        ui.label(format!(
            "The manuscript: {} words, {} characters ({} without spaces).",
            binder::format_words(c.words),
            binder::format_words(c.chars),
            binder::format_words(c.chars_no_spaces)
        ));
        if let Some(n) = progress::daily_need(&s.project, progress::today()) {
            ui.label(format!("To meet the deadline: {} words a writing day.", binder::format_words(n)));
        }
        ui.horizontal(|ui| {
            if ui.button("Start a 15-Minute Sprint").clicked() {
                self.start_progress();
                if let Some(p) = self.progress.as_mut() {
                    p.writing.start_sprint(std::time::Duration::from_secs(15 * 60), Instant::now());
                }
            }
            if ui.button(crate::i18n::tr("Writing History")).clicked() {
                self.history_view.open = true;
            }
            if ui.button(crate::i18n::tr("Start a New Session")).clicked()
                && let Some(p) = self.progress.as_mut()
            {
                p.writing.reset();
            }
        });
    }
}
