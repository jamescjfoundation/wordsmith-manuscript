// SPDX-License-Identifier: MIT OR Apache-2.0
//! The single-file form of a project (`.wsmz`), recovery copies and backups, on the core's
//! folder layer (`jcj_project::folder`).
//!
//! A packed project is a deterministic zip of the folder: a stored `mimetype` entry first, then
//! every file in sorted order with fixed times and permissions, without `.recovery/`, so the same
//! project always gives the same bytes. Reading never trusts the zip: the file size, the number of
//! entries, each entry's size (counted while reading, whatever the header says) and the total are
//! bounded, and only names inside the project's own folders are accepted. Nothing is extracted
//! blindly: entries are read one by one into memory and written into a folder only after all of
//! them passed. All of that is the core's; the bounds and the names are [`crate::folder::FORMAT`].

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use jcj_project::folder as core;
use wordsmith_manuscript_model::Project;

use crate::folder::{FORMAT, Folder, Memory, Saved, load, rendered};
use crate::{Error, limits};

/// The `mimetype` entry of a packed project.
pub const MIMETYPE: &str = "application/x-wordsmith-manuscript-packed";
/// The packed project's extension.
pub const EXTENSION: &str = "wsmz";

/// The largest packed project read.
pub const MAX_PACKED: u64 = 2 * 1024 * 1024 * 1024;
/// The largest single file inside a packed project (media included).
pub const MAX_ENTRY: u64 = 512 * 1024 * 1024;
/// The most entries in a packed project.
pub const MAX_ENTRIES: usize = 4 * limits::MAX_ITEMS + 16;
/// The most bytes unpacked in all.
pub const MAX_UNPACKED: u64 = 4 * 1024 * 1024 * 1024;

/// Pack files (relative path → bytes) into a deterministic zip. A file that is not one of a
/// project's is left out.
pub fn pack_files(files: &BTreeMap<String, Vec<u8>>) -> Result<Vec<u8>, Error> {
    if files.keys().all(|k| FORMAT.allows(k)) {
        return Ok(core::pack(&FORMAT, files)?);
    }
    let ours: BTreeMap<String, Vec<u8>> = files.iter().filter(|(k, _)| FORMAT.allows(k)).map(|(k, v)| (k.clone(), v.clone())).collect();
    Ok(core::pack(&FORMAT, &ours)?)
}

/// Every file of a project folder that belongs in a packed copy (not `.recovery/`, not files
/// outside the project's own folders, not a symbolic link).
pub fn folder_files(root: &Path) -> Result<BTreeMap<String, Vec<u8>>, Error> {
    Ok(core::folder_files(&FORMAT, root)?)
}

/// Pack a project folder.
pub fn pack_folder(root: &Path) -> Result<Vec<u8>, Error> {
    Ok(core::pack_folder(&FORMAT, root)?)
}

/// Pack a project held in memory (its text files; media must be added by the caller).
pub fn pack_project(project: &Project) -> Result<Vec<u8>, Error> {
    Ok(core::pack(&FORMAT, &rendered(project)?)?)
}

/// Pack a project held in memory with the media (and other files) of its folder, if it has one:
/// the folder's files, with the project's text files as they are now.
pub fn pack_current(project: &Project, root: Option<&Path>) -> Result<Vec<u8>, Error> {
    let mut files = match root {
        Some(r) => folder_files(r)?,
        None => BTreeMap::new(),
    };
    files.retain(|k, _| !(k.starts_with("text/") || k.starts_with("bible/") || k.starts_with("snapshots/") || k.starts_with("drafts/")));
    files.extend(rendered(project)?);
    Ok(core::pack(&FORMAT, &files)?)
}

/// Read a packed project's files into memory, checking every bound and name.
pub fn unpack(bytes: &[u8]) -> Result<BTreeMap<String, Vec<u8>>, Error> {
    Ok(core::unpack(&FORMAT, bytes)?)
}

/// Read a packed project straight into memory (for the book view and recovery), without a folder.
pub fn read_packed(bytes: &[u8]) -> Result<Project, Error> {
    let files = unpack(bytes)?;
    load(&mut Memory(&files))
}

/// Unpack a packed project into `dest`, which must not exist or be empty. The project is read and
/// checked in memory first; only then are the files written.
pub fn unpack_into(bytes: &[u8], dest: &Path) -> Result<(Folder, Project), Error> {
    // The core asks for its own error when the check fails; Manuscript's reason is kept here.
    let mut refused = None;
    let checked = core::unpack_into(&FORMAT, bytes, dest, |files| match load(&mut Memory(files)) {
        Ok(_) => Ok(()),
        Err(e) => {
            let said = e.to_string();
            refused = Some(e);
            Err(jcj_project::Error::Invalid(said))
        }
    });
    if let Err(e) = checked {
        return Err(refused.unwrap_or_else(|| e.into()));
    }
    Folder::open(dest)
}

/// The recovery copy's path for a project folder (`.recovery/unsaved.wsmz`).
pub fn recovery_path(root: &Path) -> PathBuf {
    core::recovery_path(&FORMAT, root)
}

/// Keep the project's unsaved state in `.recovery/` (called before a save and while editing).
pub fn write_recovery(root: &Path, project: &Project) -> Result<(), Error> {
    Ok(core::write_recovery(&FORMAT, root, &rendered(project)?)?)
}

/// The unsaved work left by a session that did not end cleanly, if any.
pub fn recovered(root: &Path) -> Option<Project> {
    load(&mut Memory(&core::recovered(&FORMAT, root)?)).ok()
}

/// Remove the recovery copy (after a good save, or when the writer declines it), and the
/// `.recovery` folder when nothing else is in it.
pub fn discard_recovery(root: &Path) -> Result<(), Error> {
    Ok(core::discard_recovery(&FORMAT, root)?)
}

/// Save a project with recovery around it: the unsaved state goes to `.recovery/` first, the
/// folder is saved, and the recovery copy is removed only after the save succeeded. If the app
/// stops part way, the next open finds whole files and the recovery copy.
pub fn save_safely(folder: &mut Folder, project: &Project) -> Result<Saved, Error> {
    folder.save_safely(project)
}

/// The backups of a project in `dir`, newest first: the files named
/// `<name> <YYYY-MM-DD HH.MM.SS> UTC.wsmz`, with a number after the time for a second backup in
/// the same second.
pub fn backups(dir: &Path, name: &str) -> Vec<PathBuf> {
    let mut out = core::backups(&FORMAT, dir, name);
    out.reverse();
    out
}

/// Write a packed backup of the folder into `dir` as `<name> <UTC time>.wsmz`, then keep only the
/// newest `keep` backups of this project. An existing file is never replaced.
pub fn backup(root: &Path, dir: &Path, name: &str, keep: usize, at: SystemTime) -> Result<PathBuf, Error> {
    Ok(core::backup(&FORMAT, root, dir, name, keep, at)?)
}
