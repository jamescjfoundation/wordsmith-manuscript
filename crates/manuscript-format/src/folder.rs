// SPDX-License-Identifier: MIT OR Apache-2.0
//! Reading and saving a project folder.
//!
//! Saving writes only the files that changed since they were read or last saved, each atomically
//! through `jcj-project`; text and entry files first, `project.toml`, and `binder.toml` last, so an
//! interrupted save leaves the previous binder pointing at whole files. Files of items that left
//! the binder are removed after the binder is written.
//!
//! A file whose model did not change is never rewritten, so text written by hand (or by an older
//! version) stays byte for byte as it was.

use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use wordsmith_manuscript_model::{Kind, Project};

use crate::{Error, files, limits};

/// A content hash.
pub type Hash = [u8; 32];

/// The hash of some bytes.
pub fn hash(bytes: &[u8]) -> Hash {
    Sha256::digest(bytes).into()
}

/// Where a save's files go: the folder on disk, or (in tests) somewhere that can fail on cue.
pub trait Sink {
    /// Write a whole file, atomically.
    fn write(&mut self, rel: &str, bytes: &[u8]) -> Result<(), Error>;
    /// Remove a file (it is not an error if it is already gone).
    fn remove(&mut self, rel: &str) -> Result<(), Error>;
}

/// The project folder on disk.
pub struct Disk {
    root: PathBuf,
}

impl Disk {
    /// The folder at `root`.
    pub fn new(root: &Path) -> Disk {
        Disk { root: root.to_path_buf() }
    }
}

impl Sink for Disk {
    fn write(&mut self, rel: &str, bytes: &[u8]) -> Result<(), Error> {
        let path = self.root.join(rel);
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| Error::Io { file: rel.into(), why: e.to_string() })?;
        }
        jcj_project::files::write_atomic(&path, bytes).map_err(|e| Error::Io { file: rel.into(), why: e.to_string() })
    }

    fn remove(&mut self, rel: &str) -> Result<(), Error> {
        match std::fs::remove_file(self.root.join(rel)) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(Error::Io { file: rel.into(), why: e.to_string() }),
        }
    }
}

/// What is known about one file since it was read or saved.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stamp {
    /// The bytes on disk.
    pub disk: Hash,
    /// What the model rendered to when it was read or saved.
    pub model: Hash,
}

/// What a save did.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Saved {
    /// Files written, in order.
    pub written: Vec<String>,
    /// Files removed.
    pub removed: Vec<String>,
}

/// An open project folder: where it is and what its files held when read or last saved.
#[derive(Clone, Debug)]
pub struct Folder {
    root: PathBuf,
    index: BTreeMap<String, Stamp>,
}

/// Read a file of at most `limit` bytes as UTF-8.
pub fn read_text(root: &Path, rel: &str, limit: u64) -> Result<Option<(String, Hash)>, Error> {
    let path = root.join(rel);
    let io = |e: std::io::Error| Error::Io { file: rel.into(), why: e.to_string() };
    let file = match std::fs::File::open(&path) {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(io(e)),
    };
    let mut bytes = Vec::new();
    file.take(limit + 1).read_to_end(&mut bytes).map_err(io)?;
    if bytes.len() as u64 > limit {
        return Err(Error::TooLarge { what: rel.into(), limit });
    }
    let h = hash(&bytes);
    String::from_utf8(bytes).map(|s| Some((s, h))).map_err(|_| Error::Damaged { file: rel.into(), why: "not UTF-8 text".into() })
}

impl Folder {
    /// The folder's path.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// What is known about each file.
    pub fn index(&self) -> &BTreeMap<String, Stamp> {
        &self.index
    }

    /// Open the project in `root`.
    pub fn open(root: &Path) -> Result<(Folder, Project), Error> {
        let mut index = BTreeMap::new();
        let (text, h) = read_text(root, "project.toml", limits::MAX_PROJECT_FILE)?.ok_or(Error::NotAProject)?;
        let settings = files::read_project(&text)?;
        index.insert("project.toml".to_owned(), Stamp { disk: h, model: hash(files::write_project(&settings)?.as_bytes()) });
        let (text, h) =
            read_text(root, "binder.toml", limits::MAX_BINDER_FILE)?.ok_or(Error::Damaged { file: "binder.toml".into(), why: "missing".into() })?;
        let binder = files::read_binder(&text)?;
        index.insert("binder.toml".to_owned(), Stamp { disk: h, model: hash(files::write_binder(&binder)?.as_bytes()) });
        let mut project = Project { settings, binder, ..Project::default() };
        let mut total = 0u64;
        let rows: Vec<_> = project.binder.rows().into_iter().map(|r| (r.item.id, r.item.kind)).collect();
        for (id, kind) in rows {
            let rel = match kind {
                Kind::Text | Kind::Folder => format!("text/{id}.md"),
                Kind::Entry => format!("bible/{id}.md"),
                _ => continue,
            };
            let Some((text, h)) = read_text(root, &rel, limits::MAX_TEXT_FILE)? else { continue };
            total += text.len() as u64;
            if total > limits::MAX_PROJECT_TEXT {
                return Err(Error::TooLarge { what: "the project's text".into(), limit: limits::MAX_PROJECT_TEXT });
            }
            let model = if kind == Kind::Entry {
                let e = files::read_entry(&text, &rel)?;
                let m = hash(files::write_entry(&e)?.as_bytes());
                project.entries.insert(id, e);
                m
            } else {
                let t = files::read_item(&text, &rel)?;
                let m = if t.is_empty() { hash(b"") } else { hash(files::write_item(&t)?.as_bytes()) };
                project.texts.insert(id, t);
                m
            };
            index.insert(rel, Stamp { disk: h, model });
        }
        Ok((Folder { root: root.to_path_buf(), index }, project))
    }

    /// Make a new project folder at `root` (which must not hold a project yet) and save into it.
    pub fn create(root: &Path, project: &Project) -> Result<Folder, Error> {
        if root.join("project.toml").exists() {
            return Err(Error::Io { file: "project.toml".into(), why: "a project already exists here".into() });
        }
        std::fs::create_dir_all(root).map_err(|e| Error::Io { file: root.display().to_string(), why: e.to_string() })?;
        let mut folder = Folder { root: root.to_path_buf(), index: BTreeMap::new() };
        folder.save(project)?;
        Ok(folder)
    }

    /// Save the project into its folder.
    pub fn save(&mut self, project: &Project) -> Result<Saved, Error> {
        let mut disk = Disk::new(&self.root);
        self.save_to(project, &mut disk)
    }

    /// Save through a sink. On an error the files written so far stay written and the index
    /// records them, so the next save picks up where this one stopped.
    pub fn save_to(&mut self, project: &Project, sink: &mut dyn Sink) -> Result<Saved, Error> {
        let rendered = files::render(project)?;
        let mut saved = Saved::default();
        let mut keep: Vec<&str> = Vec::with_capacity(rendered.len());
        for (rel, bytes) in &rendered {
            keep.push(rel);
            let h = hash(bytes);
            if self.index.get(rel).is_some_and(|s| s.model == h) {
                continue;
            }
            sink.write(rel, bytes)?;
            self.index.insert(rel.clone(), Stamp { disk: h, model: h });
            saved.written.push(rel.clone());
        }
        let stale: Vec<String> =
            self.index.keys().filter(|k| (k.starts_with("text/") || k.starts_with("bible/")) && !keep.contains(&k.as_str())).cloned().collect();
        for rel in stale {
            sink.remove(&rel)?;
            self.index.remove(&rel);
            saved.removed.push(rel);
        }
        Ok(saved)
    }
}
