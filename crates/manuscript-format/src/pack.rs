// SPDX-License-Identifier: MIT OR Apache-2.0
//! The single-file form of a project (`.wsmz`), recovery copies and backups.
//!
//! A packed project is a deterministic zip of the folder: a stored `mimetype` entry first, then
//! every file in sorted order with fixed times and permissions, without `.recovery/`, so the same
//! project always gives the same bytes. Reading never trusts the zip: the file size, the number of
//! entries, each entry's size (counted while reading, whatever the header says) and the total are
//! bounded, and only names inside the project's own folders are accepted. Nothing is extracted
//! blindly: entries are read one by one into memory and written into a folder only after all of
//! them passed.

use std::collections::BTreeMap;
use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use wordsmith_manuscript_model::Project;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, DateTime, ZipArchive, ZipWriter};

use crate::folder::{Folder, MemorySource, load};
use crate::{Error, files, limits};

/// The `mimetype` entry of a packed project.
pub const MIMETYPE: &str = "application/x-wordsmith-manuscript-packed";
/// The packed project's extension.
pub const EXTENSION: &str = "wsmz";
/// The recovery folder inside a project.
pub const RECOVERY_DIR: &str = ".recovery";
/// The recovery copy inside the recovery folder.
pub const RECOVERY_FILE: &str = "unsaved.wsmz";

/// The largest packed project read.
pub const MAX_PACKED: u64 = 2 * 1024 * 1024 * 1024;
/// The largest single file inside a packed project (media included).
pub const MAX_ENTRY: u64 = 512 * 1024 * 1024;
/// The most entries in a packed project.
pub const MAX_ENTRIES: usize = 4 * limits::MAX_ITEMS + 16;
/// The most bytes unpacked in all.
pub const MAX_UNPACKED: u64 = 4 * 1024 * 1024 * 1024;

const TOP: [&str; 5] = ["text/", "bible/", "media/", "scripts/", "snapshots/"];

/// Whether `name` may be a file inside a packed project: `project.toml`, `binder.toml`, or a
/// plain relative path under one of the project's folders, with no `..`, backslash, colon,
/// leading slash or empty part.
pub fn allowed_name(name: &str) -> bool {
    if name == "project.toml" || name == "binder.toml" {
        return true;
    }
    if name.len() > 512 || name.contains(['\\', ':', '\0']) || name.starts_with('/') {
        return false;
    }
    if !TOP.iter().any(|t| name.starts_with(t)) && !name.starts_with("drafts/") {
        return false;
    }
    name.split('/').all(|part| !part.is_empty() && part != "." && part != "..")
}

fn zip_err(e: impl std::fmt::Display) -> Error {
    Error::Damaged { file: "packed project".into(), why: e.to_string() }
}

/// Pack files (relative path → bytes) into a deterministic zip.
pub fn pack_files(files: &BTreeMap<String, Vec<u8>>) -> Result<Vec<u8>, Error> {
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    let base = SimpleFileOptions::default().last_modified_time(DateTime::default()).unix_permissions(0o644);
    zip.start_file("mimetype", base.compression_method(CompressionMethod::Stored)).map_err(|e| Error::Write(e.to_string()))?;
    zip.write_all(MIMETYPE.as_bytes()).map_err(|e| Error::Write(e.to_string()))?;
    for (name, bytes) in files {
        if !allowed_name(name) {
            continue;
        }
        let opts = base.compression_method(CompressionMethod::Deflated).large_file(bytes.len() as u64 >= u64::from(u32::MAX));
        zip.start_file(name.as_str(), opts).map_err(|e| Error::Write(e.to_string()))?;
        zip.write_all(bytes).map_err(|e| Error::Write(e.to_string()))?;
    }
    zip.finish().map(Cursor::into_inner).map_err(|e| Error::Write(e.to_string()))
}

/// Every file of a project folder that belongs in a packed copy (not `.recovery/`, not files
/// outside the project's own folders).
pub fn folder_files(root: &Path) -> Result<BTreeMap<String, Vec<u8>>, Error> {
    let mut out = BTreeMap::new();
    let mut total = 0u64;
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries = std::fs::read_dir(&dir).map_err(|e| Error::Io { file: dir.display().to_string(), why: e.to_string() })?;
        for entry in entries.flatten() {
            let path = entry.path();
            let Ok(rel) = path.strip_prefix(root) else { continue };
            let rel = rel.to_string_lossy().replace('\\', "/");
            let Ok(kind) = entry.file_type() else { continue };
            if kind.is_symlink() {
                continue;
            }
            if kind.is_dir() {
                if rel != RECOVERY_DIR {
                    stack.push(path);
                }
                continue;
            }
            if !allowed_name(&rel) {
                continue;
            }
            let bytes = std::fs::read(&path).map_err(|e| Error::Io { file: rel.clone(), why: e.to_string() })?;
            total += bytes.len() as u64;
            if total > MAX_UNPACKED || out.len() >= MAX_ENTRIES {
                return Err(Error::TooLarge { what: "the project folder".into(), limit: MAX_UNPACKED });
            }
            out.insert(rel, bytes);
        }
    }
    Ok(out)
}

/// Pack a project folder.
pub fn pack_folder(root: &Path) -> Result<Vec<u8>, Error> {
    pack_files(&folder_files(root)?)
}

/// Pack a project held in memory (its text files; media must be added by the caller).
pub fn pack_project(project: &Project) -> Result<Vec<u8>, Error> {
    let files: BTreeMap<String, Vec<u8>> = files::render(project)?.into_iter().collect();
    pack_files(&files)
}

/// Bounds on reading a packed project.
#[derive(Clone, Copy, Debug)]
pub struct PackLimits {
    /// The largest packed file.
    pub packed: u64,
    /// The largest entry.
    pub entry: u64,
    /// The most entries.
    pub entries: usize,
    /// The most bytes unpacked in all.
    pub unpacked: u64,
}

impl Default for PackLimits {
    fn default() -> Self {
        PackLimits { packed: MAX_PACKED, entry: MAX_ENTRY, entries: MAX_ENTRIES, unpacked: MAX_UNPACKED }
    }
}

/// Pack a project held in memory with the media (and other files) of its folder, if it has one:
/// the folder's files, with the project's text files as they are now.
pub fn pack_current(project: &Project, root: Option<&Path>) -> Result<Vec<u8>, Error> {
    let mut files = match root {
        Some(r) => folder_files(r)?,
        None => BTreeMap::new(),
    };
    files.retain(|k, _| !(k.starts_with("text/") || k.starts_with("bible/") || k.starts_with("snapshots/")));
    files.extend(files::render(project)?);
    pack_files(&files)
}

/// Read a packed project's files into memory, checking every bound and name.
pub fn unpack(bytes: &[u8]) -> Result<BTreeMap<String, Vec<u8>>, Error> {
    unpack_with(bytes, &PackLimits::default())
}

/// [`unpack`] with other bounds.
pub fn unpack_with(bytes: &[u8], lim: &PackLimits) -> Result<BTreeMap<String, Vec<u8>>, Error> {
    if bytes.len() as u64 > lim.packed {
        return Err(Error::TooLarge { what: "the packed project".into(), limit: lim.packed });
    }
    let mut zip = ZipArchive::new(Cursor::new(bytes)).map_err(|_| Error::NotAProject)?;
    if zip.len() > lim.entries {
        return Err(Error::TooLarge { what: "the packed project (entries)".into(), limit: lim.entries as u64 });
    }
    let mut out = BTreeMap::new();
    let mut total = 0u64;
    for i in 0..zip.len() {
        let entry = zip.by_index(i).map_err(zip_err)?;
        let name = entry.name().to_owned();
        if i == 0 {
            if name != "mimetype" {
                return Err(Error::NotAProject);
            }
            let mut m = Vec::new();
            entry.take(256).read_to_end(&mut m).map_err(zip_err)?;
            if m != MIMETYPE.as_bytes() {
                return Err(Error::NotAProject);
            }
            continue;
        }
        if entry.is_dir() {
            continue;
        }
        if !allowed_name(&name) {
            return Err(Error::Damaged {
                file: "packed project".into(),
                why: format!("an entry has a name outside the project: {}", name.chars().take(80).collect::<String>()),
            });
        }
        let mut data = Vec::new();
        entry.take(lim.entry + 1).read_to_end(&mut data).map_err(zip_err)?;
        if data.len() as u64 > lim.entry {
            return Err(Error::TooLarge { what: name, limit: lim.entry });
        }
        total += data.len() as u64;
        if total > lim.unpacked {
            return Err(Error::TooLarge { what: "the packed project (unpacked)".into(), limit: lim.unpacked });
        }
        if out.insert(name.clone(), data).is_some() {
            return Err(Error::Damaged { file: "packed project".into(), why: format!("{name} appears twice") });
        }
    }
    if zip.is_empty() {
        return Err(Error::NotAProject);
    }
    Ok(out)
}

/// Read a packed project straight into memory (for the book view and recovery), without a folder.
pub fn read_packed(bytes: &[u8]) -> Result<Project, Error> {
    let files = unpack(bytes)?;
    load(&MemorySource(&files)).map(|(_, p)| p)
}

/// Unpack a packed project into `dest`, which must not exist or be empty. The project is read and
/// checked in memory first; only then are the files written.
pub fn unpack_into(bytes: &[u8], dest: &Path) -> Result<(Folder, Project), Error> {
    let files = unpack(bytes)?;
    load(&MemorySource(&files))?;
    if dest.exists() && std::fs::read_dir(dest).map(|mut d| d.next().is_some()).unwrap_or(true) {
        return Err(Error::Io { file: dest.display().to_string(), why: "the folder is not empty".into() });
    }
    let mut sink = crate::folder::Disk::new(dest);
    for (rel, data) in &files {
        crate::folder::Sink::write(&mut sink, rel, data)?;
    }
    Folder::open(dest)
}

/// The recovery copy's path for a project folder.
pub fn recovery_path(root: &Path) -> PathBuf {
    root.join(RECOVERY_DIR).join(RECOVERY_FILE)
}

/// Keep the project's unsaved state in `.recovery/` (called before a save and while editing).
pub fn write_recovery(root: &Path, project: &Project) -> Result<(), Error> {
    let bytes = pack_project(project)?;
    let path = recovery_path(root);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| Error::Io { file: RECOVERY_DIR.into(), why: e.to_string() })?;
    }
    jcj_project::files::write_atomic(&path, &bytes).map_err(|e| Error::Io { file: RECOVERY_FILE.into(), why: e.to_string() })
}

/// The unsaved work left by a session that did not end cleanly, if any.
pub fn recovered(root: &Path) -> Option<Project> {
    let bytes = std::fs::read(recovery_path(root)).ok()?;
    read_packed(&bytes).ok()
}

/// Remove the recovery copy (after a good save, or when the writer declines it).
pub fn discard_recovery(root: &Path) -> Result<(), Error> {
    match std::fs::remove_dir_all(root.join(RECOVERY_DIR)) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(Error::Io { file: RECOVERY_DIR.into(), why: e.to_string() }),
    }
}

/// Save a project with recovery around it: the unsaved state goes to `.recovery/` first, the
/// folder is saved, and the recovery copy is removed only after the save succeeded. If the app
/// stops part way, the next open finds whole files and the recovery copy.
pub fn save_safely(folder: &mut Folder, project: &Project) -> Result<crate::folder::Saved, Error> {
    write_recovery(folder.root(), project)?;
    let saved = folder.save(project)?;
    discard_recovery(folder.root())?;
    Ok(saved)
}

/// The backups of a project in `dir`, newest first.
pub fn backups(dir: &Path, name: &str) -> Vec<PathBuf> {
    let prefix = format!("{name} ");
    let mut out: Vec<PathBuf> = std::fs::read_dir(dir)
        .map(|d| {
            d.flatten()
                .map(|e| e.path())
                .filter(|p| {
                    p.extension().is_some_and(|e| e == EXTENSION) && p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with(&prefix))
                })
                .collect()
        })
        .unwrap_or_default();
    out.sort();
    out.reverse();
    out
}

/// Write a packed backup of the folder into `dir` as `<name> <UTC time>.wsmz`, then keep only the
/// newest `keep` backups of this project.
pub fn backup(root: &Path, dir: &Path, name: &str, keep: usize, at: SystemTime) -> Result<PathBuf, Error> {
    let bytes = pack_folder(root)?;
    std::fs::create_dir_all(dir).map_err(|e| Error::Io { file: dir.display().to_string(), why: e.to_string() })?;
    let path = dir.join(format!("{name} {}.{EXTENSION}", jcj_project::files::stamp(at)));
    jcj_project::files::write_atomic(&path, &bytes).map_err(|e| Error::Io { file: path.display().to_string(), why: e.to_string() })?;
    for old in backups(dir, name).into_iter().skip(keep.max(1)) {
        let _ = std::fs::remove_file(old);
    }
    Ok(path)
}
