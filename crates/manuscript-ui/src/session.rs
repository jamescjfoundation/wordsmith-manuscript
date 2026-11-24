// SPDX-License-Identifier: MIT OR Apache-2.0
//! An open project: the model, its folder, the undo history of project commands, and saving.
//!
//! Every change to the project is a command run through [`Session::run`] by id with JSON
//! parameters, so the interface, the command line and agents share one path, and every command
//! that changes the project is one undo step.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime};

use serde_json::{Value, json};
use wordsmith_manuscript_format::files;
use wordsmith_manuscript_format::folder::{Folder, Stamp, hash};
use wordsmith_manuscript_format::pack;
use wordsmith_manuscript_model::{Binder, Entry, ItemId, ItemText, Project, Settings};

/// What [`Session::check_outside`] found and did.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Outside {
    /// The whole project was read again (nothing here was unsaved).
    pub project_reloaded: bool,
    /// Documents and entries read again (changed only elsewhere).
    pub reloaded: Vec<ItemId>,
    /// Documents and entries changed both here and elsewhere: this version kept, the other a
    /// snapshot (documents) or a copy in `conflicts/` (entries).
    pub conflicts: Vec<ItemId>,
    /// Copies kept in `conflicts/`.
    pub copies: Vec<String>,
    /// Files removed elsewhere that the next save writes again.
    pub restored: Vec<String>,
}

impl Outside {
    /// Whether anything was found.
    pub fn is_empty(&self) -> bool {
        *self == Outside::default()
    }

    /// A sentence for the status bar.
    pub fn summary(&self) -> String {
        if self.project_reloaded {
            return "The project was changed on another device or by a sync service; it was read again.".into();
        }
        let mut parts = Vec::new();
        if !self.reloaded.is_empty() {
            parts.push(format!("{} changed elsewhere and read again", self.reloaded.len()));
        }
        if !self.conflicts.is_empty() {
            parts.push(format!(
                "{} changed both here and elsewhere (yours kept; the other version is a snapshot named “Changed elsewhere”)",
                self.conflicts.len()
            ));
        }
        if !self.copies.is_empty() {
            parts.push(format!("{} file{} kept in the project's conflicts folder", self.copies.len(), if self.copies.len() == 1 { "" } else { "s" }));
        }
        if !self.restored.is_empty() {
            parts.push(format!("{} removed elsewhere, kept here", self.restored.len()));
        }
        format!("Outside changes: {}.", parts.join("; "))
    }
}

/// What a command touched, kept to undo it.
#[derive(Clone, Debug)]
struct Before {
    label: String,
    binder: Binder,
    settings: Settings,
    texts: BTreeMap<ItemId, Option<ItemText>>,
    entries: BTreeMap<ItemId, Option<Entry>>,
    /// The stored drafts, for commands that change them (kept only then: they can be large).
    drafts: Option<BTreeMap<String, wordsmith_manuscript_model::drafts::DraftBody>>,
}

/// How often a packed backup is made while the writer works.
pub const BACKUP_EVERY: std::time::Duration = std::time::Duration::from_secs(30 * 60);
/// How many backups are kept per project.
pub const BACKUPS_KEPT: usize = 10;

/// An open project.
pub struct Session {
    /// The project.
    pub project: Project,
    folder: Option<Folder>,
    undo: Vec<Before>,
    redo: Vec<Before>,
    revision: u64,
    saved: u64,
    last_backup: Option<Instant>,
    backups_dir: Option<PathBuf>,
    /// Media added since the last save (images, PDFs), by item: written into `media/` on save.
    pub media: BTreeMap<ItemId, Vec<u8>>,
    /// The series bible this book shares (BND-8): its folder and its project as last read or
    /// saved.
    series: Option<(Folder, Project)>,
    /// Why the series bible could not be opened, if it could not.
    pub series_problem: Option<String>,
}

/// The result of a command: a JSON value for agents and the command line.
pub type Outcome = Result<Value, String>;

/// The most undo steps kept for project commands.
pub const MAX_UNDO: usize = 200;

impl Session {
    /// A session on a project that has no folder yet.
    pub fn new(project: Project) -> Session {
        Session {
            project,
            folder: None,
            undo: Vec::new(),
            redo: Vec::new(),
            revision: 0,
            saved: 0,
            last_backup: None,
            backups_dir: None,
            media: BTreeMap::new(),
            series: None,
            series_problem: None,
        }
    }

    /// Open the project folder at `path` (the folder, its `project.toml`, or a `.wsmz` to unpack
    /// into `unpack_to`).
    pub fn open(path: &Path) -> Result<Session, String> {
        let root = project_root(path);
        let (folder, project) = Folder::open(&root).map_err(|e| e.to_string())?;
        let mut s = Session::new(project);
        s.folder = Some(folder);
        s.attach_series();
        Ok(s)
    }

    /// The series bible's folder, if this book shares one.
    pub fn series_root(&self) -> Option<&Path> {
        self.series.as_ref().map(|(f, _)| f.root())
    }

    /// The series bible's details, if this book shares one.
    pub fn series_info(&self) -> Option<wordsmith_manuscript_model::series::SeriesInfo> {
        self.series.as_ref().and_then(|(_, p)| wordsmith_manuscript_model::series::info(p))
    }

    /// Open the series bible the settings name and merge it in (a problem is kept in
    /// `series_problem`; the book then opens with its own bible).
    fn attach_series(&mut self) {
        self.series = None;
        self.series_problem = None;
        let rel = self.project.settings.series.clone();
        if rel.is_empty() {
            return;
        }
        let Some(root) = self.root().map(|r| r.join(&rel)) else { return };
        let root = std::fs::canonicalize(&root).unwrap_or(root);
        match Folder::open(&root) {
            Ok((f, bible)) if wordsmith_manuscript_model::series::info(&bible).is_some() => {
                self.project = wordsmith_manuscript_model::series::merge(&self.project, &bible);
                self.series = Some((f, bible));
            }
            Ok(_) => self.series_problem = Some(format!("{} is not a series bible", root.display())),
            Err(e) => {
                self.series_problem = Some(format!("the series bible at {} could not be opened ({e}); this book shows its own bible", root.display()))
            }
        }
    }

    /// Make a new series bible at `root` named `name` from this book's Story Bible, and share it
    /// from now on (BND-8). The book must have a folder.
    pub fn create_series(&mut self, root: &Path, name: &str) -> Result<(), String> {
        let own = canonical(self.root().ok_or("save the book first")?);
        let root = &absolute(root);
        if self.series.is_some() {
            return Err("this book is already in a series".into());
        }
        let mut base = wordsmith_manuscript_model::series::new_bible(name.trim());
        let mut info = wordsmith_manuscript_model::series::info(&base).unwrap_or_default();
        info.books.push(wordsmith_manuscript_model::series::relative(root, &own));
        wordsmith_manuscript_model::series::set_info(&mut base, &info);
        let (_, bible) = wordsmith_manuscript_model::series::split(&self.project, &base);
        let folder = Folder::create(root, &bible).map_err(|e| e.to_string())?;
        self.series = Some((folder, bible));
        self.project.settings.series = wordsmith_manuscript_model::series::relative(&own, root);
        self.revision += 1;
        self.save()
    }

    /// Share the series bible at `root`: this book's own entries join it (BND-8).
    pub fn join_series(&mut self, root: &Path) -> Result<(), String> {
        let own = canonical(self.root().ok_or("save the book first")?);
        let root = &canonical(root);
        if self.series.is_some() {
            return Err("this book is already in a series".into());
        }
        let (folder, mut bible) = Folder::open(root).map_err(|e| e.to_string())?;
        let mut info = wordsmith_manuscript_model::series::info(&bible).ok_or_else(|| format!("{} is not a series bible", root.display()))?;
        let me = wordsmith_manuscript_model::series::relative(root, &own);
        if !info.books.contains(&me) {
            info.books.push(me);
        }
        wordsmith_manuscript_model::series::set_info(&mut bible, &info);
        // The series' entry types win; this book's own entries are added to the series.
        if let Some(t) = bible.settings.extra.get(wordsmith_manuscript_model::series::TEMPLATES).cloned() {
            self.project.settings.extra.insert(wordsmith_manuscript_model::series::TEMPLATES.into(), t);
        }
        self.project = wordsmith_manuscript_model::series::merge(&self.project, &bible);
        self.series = Some((folder, bible));
        self.project.settings.series = wordsmith_manuscript_model::series::relative(&own, root);
        self.revision += 1;
        self.save()
    }

    /// Stop sharing the series bible: this book keeps a copy of it as its own bible.
    pub fn leave_series(&mut self) -> Result<(), String> {
        let own = canonical(self.root().ok_or("save the book first")?);
        let Some((mut folder, mut bible)) = self.series.take() else { return Err("this book is not in a series".into()) };
        let me = wordsmith_manuscript_model::series::relative(&canonical(folder.root()), &own);
        if let Some(mut info) = wordsmith_manuscript_model::series::info(&bible) {
            info.books.retain(|b| *b != me);
            wordsmith_manuscript_model::series::set_info(&mut bible, &info);
            pack::save_safely(&mut folder, &bible).map_err(|e| e.to_string())?;
        }
        self.project.settings.series.clear();
        self.revision += 1;
        self.save()
    }

    /// Unpack a `.wsmz` into a new folder and open it.
    pub fn open_packed(packed: &Path, unpack_to: &Path) -> Result<Session, String> {
        let bytes = std::fs::read(packed).map_err(|e| format!("{}: {e}", packed.display()))?;
        let (folder, project) = pack::unpack_into(&bytes, unpack_to).map_err(|e| e.to_string())?;
        let mut s = Session::new(project);
        s.folder = Some(folder);
        Ok(s)
    }

    /// Make a new project folder at `root` for `project` and open it.
    pub fn create(root: &Path, project: Project) -> Result<Session, String> {
        let folder = Folder::create(root, &project).map_err(|e| e.to_string())?;
        let mut s = Session::new(project);
        s.folder = Some(folder);
        Ok(s)
    }

    /// The project folder, if the project has one.
    pub fn root(&self) -> Option<&Path> {
        self.folder.as_ref().map(Folder::root)
    }

    /// Where backups go (by default, a `Backups` folder beside the project folder).
    pub fn set_backups_dir(&mut self, dir: PathBuf) {
        self.backups_dir = Some(dir);
    }

    fn backups_dir(&self) -> Option<PathBuf> {
        self.backups_dir.clone().or_else(|| self.root().and_then(Path::parent).map(|p| p.join("Backups")))
    }

    /// A number that changes with every change.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// Whether there are changes not saved yet.
    pub fn is_dirty(&self) -> bool {
        self.revision != self.saved
    }

    /// Mark a change made outside [`Session::run`] (typing in an editor that writes back).
    pub fn touch(&mut self) {
        self.revision += 1;
    }

    /// Save into the project folder (recovery copy first, removed after). New media files are
    /// written before the binder that names them.
    pub fn save(&mut self) -> Result<(), String> {
        let root = self.root().ok_or("the project has no folder yet: use Save As")?.to_path_buf();
        self.write_media(&root, &self.media.clone())?;
        self.media.clear();
        let folder = self.folder.as_mut().ok_or("the project has no folder yet: use Save As")?;
        match self.series.as_mut() {
            // A book of a series: its own part here, the bible in the series folder.
            Some((sf, base)) => {
                let (own, bible) = wordsmith_manuscript_model::series::split(&self.project, base);
                pack::save_safely(folder, &own).map_err(|e| e.to_string())?;
                pack::save_safely(sf, &bible).map_err(|e| e.to_string())?;
                *base = bible;
            }
            None => {
                pack::save_safely(folder, &self.project).map_err(|e| e.to_string())?;
            }
        }
        self.saved = self.revision;
        Ok(())
    }

    /// Save into a new folder and continue there (media copied along).
    pub fn save_as(&mut self, root: &Path) -> Result<(), String> {
        let media = self.all_media()?;
        if root.join("project.toml").exists() {
            return Err("a project already exists here".into());
        }
        std::fs::create_dir_all(root).map_err(|e| format!("{}: {e}", root.display()))?;
        self.write_media(root, &media)?;
        if let Some((sf, _)) = self.series.as_ref() {
            self.project.settings.series = wordsmith_manuscript_model::series::relative(root, sf.root());
        }
        let own = match self.series.as_ref() {
            Some((_, base)) => wordsmith_manuscript_model::series::split(&self.project, base).0,
            None => self.project.clone(),
        };
        let folder = Folder::create(root, &own).map_err(|e| e.to_string())?;
        self.folder = Some(folder);
        self.media.clear();
        self.saved = self.revision;
        Ok(())
    }

    /// Save the project as one packed file (`.wsmz`), leaving the folder as it is: the folder's
    /// files (media among them), the texts as they are now, and media not saved yet.
    pub fn save_packed(&self, file: &Path) -> Result<(), String> {
        let mut files = match self.root() {
            Some(root) => pack::folder_files(root).map_err(|e| e.to_string())?,
            None => BTreeMap::new(),
        };
        files.retain(|k, _| {
            !(k.starts_with("text/")
                || k.starts_with("bible/")
                || k.starts_with("snapshots/")
                || k.starts_with("drafts/")
                || k == "project.toml"
                || k == "binder.toml")
        });
        files.extend(wordsmith_manuscript_format::files::render(&self.project).map_err(|e| e.to_string())?);
        for (id, bytes) in &self.media {
            if let Some(path) = self.media_path(*id) {
                files.insert(path, bytes.clone());
            }
        }
        let bytes = pack::pack_files(&files).map_err(|e| e.to_string())?;
        jcj_project_write(file, &bytes)
    }

    /// Look for changes made to the project folder outside the app (another device, a sync
    /// service) and take them in (FIL-4):
    ///
    /// - a document or entry changed only there is read again;
    /// - one changed both there and here keeps this version, and the other version becomes a
    ///   snapshot of it ("Changed elsewhere") to compare and restore; an entry's other version is
    ///   copied into `conflicts/`;
    /// - a changed binder or project settings reload the whole project when nothing here is
    ///   unsaved; otherwise this version is kept and theirs copied into `conflicts/`.
    ///
    /// Nothing is overwritten without a copy, and nothing is lost.
    pub fn check_outside(&mut self) -> Result<Outside, String> {
        let mut out = Outside::default();
        // The series bible first: another book (or another device) may have changed it.
        if let Some((sf, _)) = self.series.as_ref() {
            let changes = sf.outside_changes().map_err(|e| e.to_string())?;
            if !changes.is_empty() {
                if self.is_dirty() {
                    // Keep this version; theirs is copied, and the next save writes ours.
                    let sroot = sf.root().to_path_buf();
                    let mut disk = wordsmith_manuscript_format::folder::Disk::new(&sroot);
                    let now = SystemTime::now();
                    for c in changes {
                        let stamp = self.series.as_ref().and_then(|(f, _)| f.index().get(&c.rel).copied());
                        if let Some((text, h)) = c.now {
                            let copy = wordsmith_manuscript_format::folder::conflict_name(&c.rel, now);
                            wordsmith_manuscript_format::folder::Sink::write(&mut disk, &copy, text.as_bytes()).map_err(|e| e.to_string())?;
                            out.copies.push(copy);
                            if let (Some((f, _)), Some(st)) = (self.series.as_mut(), stamp) {
                                f.accept(&c.rel, Some(Stamp { disk: h, model: st.model }));
                            }
                        }
                    }
                } else {
                    let (own, _) = match self.series.as_ref() {
                        Some((_, base)) => wordsmith_manuscript_model::series::split(&self.project, base),
                        None => (self.project.clone(), Project::default()),
                    };
                    self.project = own;
                    self.attach_series();
                    self.undo.clear();
                    self.redo.clear();
                    self.revision += 1;
                    self.saved = self.revision;
                    out.project_reloaded = true;
                    return Ok(out);
                }
            }
        }
        let Some(folder) = self.folder.as_ref() else { return Ok(out) };
        let root = folder.root().to_path_buf();
        let changes = folder.outside_changes().map_err(|e| e.to_string())?;
        if changes.is_empty() {
            return Ok(out);
        }
        let structural = changes.iter().any(|c| c.rel == "binder.toml" || c.rel == "project.toml");
        if structural && !self.is_dirty() && self.media.is_empty() {
            let (f, p) = Folder::open(&root).map_err(|e| e.to_string())?;
            self.project = p;
            self.folder = Some(f);
            self.attach_series();
            self.undo.clear();
            self.redo.clear();
            self.revision += 1;
            self.saved = self.revision;
            out.project_reloaded = true;
            return Ok(out);
        }
        let now = SystemTime::now();
        let mut disk = wordsmith_manuscript_format::folder::Disk::new(&root);
        for c in changes {
            let stamp = self.folder.as_ref().and_then(|f| f.index().get(&c.rel).copied());
            let id: Option<ItemId> = c.rel.split_once('/').and_then(|(_, n)| n.strip_suffix(".md")).and_then(|n| n.parse().ok());
            let Some((text, h)) = c.now else {
                // Removed there: this version is written again at the next save.
                if let Some(f) = self.folder.as_mut() {
                    f.accept(&c.rel, None);
                }
                out.restored.push(c.rel);
                continue;
            };
            let keep_copy = |disk: &mut wordsmith_manuscript_format::folder::Disk, out: &mut Outside| -> Result<(), String> {
                let copy = wordsmith_manuscript_format::folder::conflict_name(&c.rel, now);
                wordsmith_manuscript_format::folder::Sink::write(disk, &copy, text.as_bytes()).map_err(|e| e.to_string())?;
                out.copies.push(copy);
                Ok(())
            };
            match (c.rel.as_str(), id) {
                ("binder.toml" | "project.toml", _) => {
                    keep_copy(&mut disk, &mut out)?;
                    if let (Some(f), Some(st)) = (self.folder.as_mut(), stamp) {
                        f.accept(&c.rel, Some(Stamp { disk: h, model: st.model }));
                    }
                }
                (rel, Some(id)) if rel.starts_with("text/") && self.project.binder.contains(id) => {
                    let theirs = files::read_item(&text, rel).map_err(|e| e.to_string())?;
                    let mine = self.project.texts.get(&id).cloned().unwrap_or_default();
                    let mine_hash = if mine.is_empty() { hash(b"") } else { hash(files::write_item(&mine).map_err(|e| e.to_string())?.as_bytes()) };
                    let unchanged_here = stamp.is_none_or(|st| st.model == mine_hash);
                    let theirs_hash =
                        if theirs.is_empty() { hash(b"") } else { hash(files::write_item(&theirs).map_err(|e| e.to_string())?.as_bytes()) };
                    if unchanged_here {
                        self.project.texts.insert(id, theirs);
                        out.reloaded.push(id);
                        if let Some(f) = self.folder.as_mut() {
                            f.accept(rel, Some(Stamp { disk: h, model: theirs_hash }));
                        }
                    } else {
                        let taken = wordsmith_manuscript_model::snapshot::stamp(now);
                        let list = self.project.snapshots.entry(id).or_default();
                        if !list.iter().any(|x| x.taken == taken) {
                            list.push(wordsmith_manuscript_model::Snapshot { taken, name: "Changed elsewhere".into(), text: theirs.body });
                        }
                        out.conflicts.push(id);
                        if let (Some(f), Some(st)) = (self.folder.as_mut(), stamp) {
                            f.accept(rel, Some(Stamp { disk: h, model: st.model }));
                        }
                    }
                    self.revision += 1;
                }
                (rel, Some(id)) if rel.starts_with("bible/") && self.project.binder.contains(id) => {
                    let theirs = files::read_entry(&text, rel).map_err(|e| e.to_string())?;
                    let mine = self.project.entries.get(&id).cloned();
                    let mine_hash = match &mine {
                        Some(e) => hash(files::write_entry(e).map_err(|e| e.to_string())?.as_bytes()),
                        None => hash(b""),
                    };
                    let theirs_hash = hash(files::write_entry(&theirs).map_err(|e| e.to_string())?.as_bytes());
                    if stamp.is_none_or(|st| st.model == mine_hash) {
                        self.project.entries.insert(id, theirs);
                        out.reloaded.push(id);
                        if let Some(f) = self.folder.as_mut() {
                            f.accept(rel, Some(Stamp { disk: h, model: theirs_hash }));
                        }
                    } else {
                        keep_copy(&mut disk, &mut out)?;
                        out.conflicts.push(id);
                        if let (Some(f), Some(st)) = (self.folder.as_mut(), stamp) {
                            f.accept(rel, Some(Stamp { disk: h, model: st.model }));
                        }
                    }
                    self.revision += 1;
                }
                // A file of an item this binder does not hold (yet): kept as it is, and copied
                // so it is safe whatever happens to the binder.
                _ => {
                    keep_copy(&mut disk, &mut out)?;
                    if let Some(f) = self.folder.as_mut() {
                        f.accept(&c.rel, Some(Stamp { disk: h, model: hash(text.as_bytes()) }));
                    }
                }
            }
        }
        if !out.reloaded.is_empty() {
            // Undo steps from before would put the other device's changes back out.
            self.undo.clear();
            self.redo.clear();
        }
        Ok(out)
    }

    /// Where a media item's file is inside the project (`media/<id>.<ext>`).
    pub fn media_path(&self, id: ItemId) -> Option<String> {
        self.project
            .binder
            .get(id)
            .filter(|i| matches!(i.kind, wordsmith_manuscript_model::Kind::Media | wordsmith_manuscript_model::Kind::Screenplay))
            .and_then(wordsmith_manuscript_format::files::path_of)
    }

    /// A media item's bytes: added since the last save, or from the project folder.
    pub fn media_bytes(&self, id: ItemId) -> Result<Vec<u8>, String> {
        if let Some(b) = self.media.get(&id) {
            return Ok(b.clone());
        }
        let path = self.media_path(id).ok_or("this item is not a media file")?;
        let root = self.root().ok_or("the file is not in the project")?;
        let full = root.join(&path);
        let len = std::fs::metadata(&full).map_err(|e| format!("{path}: {e}"))?.len();
        if len > crate::media::MAX_BYTES {
            return Err(format!("{path} is larger than {} MB", crate::media::MAX_BYTES >> 20));
        }
        std::fs::read(&full).map_err(|e| format!("{path}: {e}"))
    }

    /// Every media item's bytes (for Save As).
    fn all_media(&self) -> Result<BTreeMap<ItemId, Vec<u8>>, String> {
        let mut out = BTreeMap::new();
        for r in self.project.binder.rows() {
            if matches!(r.item.kind, wordsmith_manuscript_model::Kind::Media | wordsmith_manuscript_model::Kind::Screenplay) {
                match self.media_bytes(r.item.id) {
                    Ok(b) => {
                        out.insert(r.item.id, b);
                    }
                    // A file already missing from the old folder cannot be copied; the item stays.
                    Err(_) if !self.media.contains_key(&r.item.id) => {}
                    Err(e) => return Err(e),
                }
            }
        }
        Ok(out)
    }

    fn write_media(&self, root: &Path, media: &BTreeMap<ItemId, Vec<u8>>) -> Result<(), String> {
        let mut disk = wordsmith_manuscript_format::folder::Disk::new(root);
        for (id, bytes) in media {
            if let Some(path) = self.media_path(*id) {
                wordsmith_manuscript_format::folder::Sink::write(&mut disk, &path, bytes).map_err(|e| e.to_string())?;
            }
        }
        Ok(())
    }

    /// Keep the unsaved state in `.recovery/` (between saves, while editing).
    pub fn write_recovery(&self) -> Result<(), String> {
        match self.root() {
            Some(root) if self.is_dirty() => pack::write_recovery(root, &self.project).map_err(|e| e.to_string()),
            _ => Ok(()),
        }
    }

    /// Make a backup now (after saving).
    pub fn backup(&mut self) -> Result<Option<PathBuf>, String> {
        let (Some(root), Some(dir)) = (self.root().map(Path::to_path_buf), self.backups_dir()) else { return Ok(None) };
        let name = root.file_stem().and_then(|n| n.to_str()).unwrap_or("Project").to_owned();
        let path = pack::backup(&root, &dir, &name, BACKUPS_KEPT, SystemTime::now()).map_err(|e| e.to_string())?;
        // The series bible is backed up beside it.
        if let Some((sf, _)) = self.series.as_ref() {
            let sname = sf.root().file_stem().and_then(|n| n.to_str()).unwrap_or("Series").to_owned();
            pack::backup(sf.root(), &dir, &sname, BACKUPS_KEPT, SystemTime::now()).map_err(|e| e.to_string())?;
        }
        self.last_backup = Some(Instant::now());
        Ok(Some(path))
    }

    /// Whether a scheduled backup is due.
    pub fn backup_due(&self, now: Instant) -> bool {
        self.root().is_some() && self.last_backup.is_none_or(|t| now.duration_since(t) >= BACKUP_EVERY)
    }

    /// On close: save if needed, then back up.
    pub fn close(&mut self) -> Result<(), String> {
        if self.root().is_none() {
            return Ok(());
        }
        if self.is_dirty() {
            self.save()?;
        }
        self.backup().map(|_| ())
    }

    fn snapshot(&self, label: &str, texts: &[ItemId], entries: &[ItemId]) -> Before {
        Before {
            label: label.to_owned(),
            binder: self.project.binder.clone(),
            settings: self.project.settings.clone(),
            texts: texts.iter().map(|id| (*id, self.project.texts.get(id).cloned())).collect(),
            entries: entries.iter().map(|id| (*id, self.project.entries.get(id).cloned())).collect(),
            drafts: None,
        }
    }

    fn restore(&mut self, b: &Before) -> Before {
        let ids: Vec<ItemId> = b.texts.keys().copied().collect();
        let eids: Vec<ItemId> = b.entries.keys().copied().collect();
        let mut now = self.snapshot(&b.label, &ids, &eids);
        if let Some(d) = &b.drafts {
            now.drafts = Some(std::mem::replace(&mut self.project.drafts, d.clone()));
        }
        self.project.binder = b.binder.clone();
        self.project.settings = b.settings.clone();
        for (id, t) in &b.texts {
            match t {
                Some(t) => {
                    self.project.texts.insert(*id, t.clone());
                }
                None => {
                    self.project.texts.remove(id);
                }
            }
        }
        for (id, e) in &b.entries {
            match e {
                Some(e) => {
                    self.project.entries.insert(*id, e.clone());
                }
                None => {
                    self.project.entries.remove(id);
                }
            }
        }
        now
    }

    /// Run a change as one undo step. `texts` and `entries` name the items whose text or entry
    /// the change may alter (the binder and settings are always kept).
    pub fn change<T>(
        &mut self,
        label: &str,
        texts: &[ItemId],
        entries: &[ItemId],
        f: impl FnOnce(&mut Project) -> Result<T, String>,
    ) -> Result<T, String> {
        let before = self.snapshot(label, texts, entries);
        match f(&mut self.project) {
            Ok(v) => {
                self.undo.push(before);
                if self.undo.len() > MAX_UNDO {
                    self.undo.remove(0);
                }
                self.redo.clear();
                self.revision += 1;
                Ok(v)
            }
            Err(e) => {
                self.restore(&before);
                Err(e)
            }
        }
    }

    /// Run a change to the drafts as one undo step: the stored drafts and every text the
    /// Manuscript area holds before and after are kept.
    pub fn change_drafts<T>(&mut self, label: &str, f: impl FnOnce(&mut Project) -> Result<T, String>) -> Result<T, String> {
        let mut ids: Vec<ItemId> = self.project.binder.rows_of(wordsmith_manuscript_model::Area::Manuscript).iter().map(|r| r.item.id).collect();
        for d in self.project.drafts.values() {
            ids.extend(d.binder.rows().iter().map(|r| r.item.id));
        }
        ids.sort();
        ids.dedup();
        let mut before = self.snapshot(label, &ids, &[]);
        before.drafts = Some(self.project.drafts.clone());
        let had: std::collections::HashSet<ItemId> = self.project.texts.keys().copied().collect();
        match f(&mut self.project) {
            Ok(v) => {
                // Texts the change added (items given new ids) go again on undo.
                for id in self.project.texts.keys() {
                    if !had.contains(id) {
                        before.texts.entry(*id).or_insert(None);
                    }
                }
                self.undo.push(before);
                if self.undo.len() > MAX_UNDO {
                    self.undo.remove(0);
                }
                self.redo.clear();
                self.revision += 1;
                Ok(v)
            }
            Err(e) => {
                self.restore(&before);
                Err(e)
            }
        }
    }

    /// Undo the last project command. Returns its label.
    pub fn undo(&mut self) -> Option<String> {
        let b = self.undo.pop()?;
        let now = self.restore(&b);
        self.redo.push(now);
        self.revision += 1;
        Some(b.label)
    }

    /// Redo the last undone project command.
    pub fn redo(&mut self) -> Option<String> {
        let b = self.redo.pop()?;
        let now = self.restore(&b);
        self.undo.push(now);
        self.revision += 1;
        Some(b.label)
    }

    /// The label of what Undo would undo.
    pub fn undo_label(&self) -> Option<&str> {
        self.undo.last().map(|b| b.label.as_str())
    }

    /// Whether Redo has something to redo.
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// Run a command by id (see [`crate::commands`]).
    pub fn run(&mut self, id: &str, params: &Value) -> Outcome {
        crate::commands::run(self, id, params)
    }

    /// Run a command with no parameters.
    pub fn run0(&mut self, id: &str) -> Outcome {
        self.run(id, &json!({}))
    }
}

/// A path with `.` and `..` resolved through the file system (the path itself when it does not
/// exist).
fn canonical(p: &Path) -> PathBuf {
    std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf())
}

/// A path made absolute, its parent resolved (for a folder about to be made).
fn absolute(p: &Path) -> PathBuf {
    match (p.parent(), p.file_name()) {
        (Some(parent), Some(name)) if !parent.as_os_str().is_empty() => {
            std::fs::create_dir_all(parent).ok();
            canonical(parent).join(name)
        }
        _ => std::env::current_dir().map(|d| d.join(p)).unwrap_or_else(|_| p.to_path_buf()),
    }
}

fn jcj_project_write(file: &Path, bytes: &[u8]) -> Result<(), String> {
    wordsmith_manuscript_format::folder::Sink::write(
        &mut wordsmith_manuscript_format::folder::Disk::new(file.parent().unwrap_or(Path::new("."))),
        &file.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
        bytes,
    )
    .map_err(|e| e.to_string())
}

/// The project folder for a path the user gave: the folder itself, or the folder holding a
/// `project.toml` they picked.
pub fn project_root(path: &Path) -> PathBuf {
    if path.file_name().is_some_and(|n| n == "project.toml") {
        path.parent().map_or_else(|| path.to_path_buf(), Path::to_path_buf)
    } else {
        path.to_path_buf()
    }
}
