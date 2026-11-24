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
use wordsmith_manuscript_format::folder::Folder;
use wordsmith_manuscript_format::pack;
use wordsmith_manuscript_model::{Binder, Entry, ItemId, ItemText, Project, Settings};

/// What a command touched, kept to undo it.
#[derive(Clone, Debug)]
struct Before {
    label: String,
    binder: Binder,
    settings: Settings,
    texts: BTreeMap<ItemId, Option<ItemText>>,
    entries: BTreeMap<ItemId, Option<Entry>>,
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
        }
    }

    /// Open the project folder at `path` (the folder, its `project.toml`, or a `.wsmz` to unpack
    /// into `unpack_to`).
    pub fn open(path: &Path) -> Result<Session, String> {
        let root = project_root(path);
        let (folder, project) = Folder::open(&root).map_err(|e| e.to_string())?;
        let mut s = Session::new(project);
        s.folder = Some(folder);
        Ok(s)
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
        pack::save_safely(folder, &self.project).map_err(|e| e.to_string())?;
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
        let folder = Folder::create(root, &self.project).map_err(|e| e.to_string())?;
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
            !(k.starts_with("text/") || k.starts_with("bible/") || k.starts_with("snapshots/") || k == "project.toml" || k == "binder.toml")
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

    /// Where a media item's file is inside the project (`media/<id>.<ext>`).
    pub fn media_path(&self, id: ItemId) -> Option<String> {
        self.project
            .binder
            .get(id)
            .filter(|i| i.kind == wordsmith_manuscript_model::Kind::Media)
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
            if r.item.kind == wordsmith_manuscript_model::Kind::Media {
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
        }
    }

    fn restore(&mut self, b: &Before) -> Before {
        let ids: Vec<ItemId> = b.texts.keys().copied().collect();
        let eids: Vec<ItemId> = b.entries.keys().copied().collect();
        let now = self.snapshot(&b.label, &ids, &eids);
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
