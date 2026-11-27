// SPDX-License-Identifier: MIT OR Apache-2.0
//! Reading and saving a project folder, on the core's folder layer (`jcj_project::folder`, core
//! decision 0018).
//!
//! What every app's folder does is the core's: a save writes only the files that changed since
//! they were read or last saved, each atomically, `project.toml` and `binder.toml` after every
//! other file, so an interrupted save leaves the previous binder pointing at whole files; files
//! of items that left the binder are removed after the binder is written; a file changed
//! elsewhere is copied into `conflicts/` before a save writes over it. What is Manuscript's is
//! here: [`FORMAT`], the layout of a `.wsm` folder, and [`load`], which reads a project from its
//! files and says what each renders to.
//!
//! A file whose model did not change is never rewritten, so text written by hand (or by an older
//! version) stays byte for byte as it was.

use std::path::Path;

use jcj_project::folder::{self as core, Files, Format, Limits};
use wordsmith_manuscript_model::{Area, Kind, Project};

use crate::{Error, SCHEMA, files, limits, pack};

pub use jcj_project::folder::{Disk, Hash, Memory, OutsideChange, Saved, Sink, Source, Stamp, hash};

/// The folders of a project, besides its two root files.
const FOLDERS: [&str; 7] = ["text/", "bible/", "media/", "scripts/", "snapshots/", "conflicts/", "drafts/"];

/// Whether a path is one of a project's files: `binder.toml`, or a file under one of its folders
/// (`project.toml`, the manifest, always is).
fn belongs(path: &str) -> bool {
    path == "binder.toml" || FOLDERS.iter().any(|f| path.starts_with(f))
}

/// The layout of a project folder (`docs/format.md`), as the core's folder layer reads and
/// writes it.
pub const FORMAT: Format = Format {
    last: &["project.toml", "binder.toml"],
    belongs,
    conflicts: Some("conflicts"),
    conflict_stamp: wordsmith_manuscript_model::snapshot::stamp,
    limits: Limits {
        manifest: limits::MAX_PROJECT_FILE,
        name: 512,
        packed: pack::MAX_PACKED,
        entry: pack::MAX_ENTRY,
        entries: pack::MAX_ENTRIES,
        unpacked: pack::MAX_UNPACKED,
    },
    ..Format::new("project.toml", SCHEMA, pack::MIMETYPE, pack::EXTENSION)
};

/// Where a copy of `rel` is kept when a save would write over a change made elsewhere:
/// `conflicts/<stamp> <rel with slashes as dashes>`.
pub fn conflict_name(rel: &str, at: std::time::SystemTime) -> String {
    FORMAT.conflict_name(rel, at).unwrap_or_default()
}

/// Whether `rel` names a file the project's model holds (`project.toml`, `binder.toml`,
/// `text/<id>.md`, `bible/<id>.md`): the files whose changes made elsewhere the app takes in.
fn watched(rel: &str) -> bool {
    if rel == "project.toml" || rel == "binder.toml" {
        return true;
    }
    let Some((dir, name)) = rel.split_once('/') else { return false };
    (dir == "text" || dir == "bible") && name.strip_suffix(".md").is_some_and(|id| id.parse::<wordsmith_manuscript_model::ItemId>().is_ok())
}

/// A file of the project as text, read within `limit` bytes; `None` when it is missing.
fn read(src: &mut dyn Source, rel: &str, limit: u64) -> Result<Option<String>, Error> {
    match src.read(rel, limit) {
        Ok(None) => Ok(None),
        Ok(Some(bytes)) => String::from_utf8(bytes).map(Some).map_err(|_| Error::Damaged { file: rel.into(), why: "not UTF-8 text".into() }),
        Err(jcj_project::Error::TooLarge(_)) => Err(Error::TooLarge { what: rel.into(), limit }),
        Err(e) => Err(e.into()),
    }
}

/// Read a project from a source (a folder, or the files of a packed project in memory), telling
/// the source what each file renders to, so a save leaves alone a file the writer did not change.
pub fn load(src: &mut dyn Source) -> Result<Project, Error> {
    let text = read(src, "project.toml", limits::MAX_PROJECT_FILE)?.ok_or(Error::NotAProject)?;
    let settings = files::read_project(&text)?;
    src.rendered("project.toml", files::write_project(&settings)?.as_bytes());
    let text = read(src, "binder.toml", limits::MAX_BINDER_FILE)?.ok_or(Error::Damaged { file: "binder.toml".into(), why: "missing".into() })?;
    let binder = files::read_binder(&text)?;
    src.rendered("binder.toml", files::write_binder(&binder)?.as_bytes());
    let mut project = Project { settings, binder, ..Project::default() };
    let mut total = 0u64;
    let mut count = |text: &str| {
        total += text.len() as u64;
        if total > limits::MAX_PROJECT_TEXT {
            return Err(Error::TooLarge { what: "the project's text".into(), limit: limits::MAX_PROJECT_TEXT });
        }
        Ok(())
    };
    let rows: Vec<_> = project.binder.rows().into_iter().map(|r| (r.item.id, r.item.kind)).collect();
    for (id, kind) in rows {
        let rel = match kind {
            Kind::Text | Kind::Folder | Kind::Screenplay => format!("text/{id}.md"),
            Kind::Entry => format!("bible/{id}.md"),
            _ => continue,
        };
        let Some(text) = read(src, &rel, limits::MAX_TEXT_FILE)? else { continue };
        count(&text)?;
        if kind == Kind::Entry {
            let e = files::read_entry(&text, &rel)?;
            src.rendered(&rel, files::write_entry(&e)?.as_bytes());
            project.entries.insert(id, e);
        } else {
            let t = files::read_item(&text, &rel)?;
            // An empty text is not written: what it renders to is nothing.
            src.rendered(&rel, if t.is_empty() { Vec::new() } else { files::write_item(&t)?.into_bytes() }.as_slice());
            project.texts.insert(id, t);
        }
    }
    // Stored drafts (schema 3): each draft's manuscript tree and texts.
    for info in project.settings.drafts.clone() {
        if !files::draft_id_ok(&info.id) {
            return Err(Error::Damaged { file: "project.toml".into(), why: format!("`{}` is not a draft id", info.id) });
        }
        let rel = format!("drafts/{}/binder.toml", info.id);
        let Some(text) = read(src, &rel, limits::MAX_BINDER_FILE)? else { continue };
        let binder = files::read_binder(&text).map_err(|e| match e {
            Error::Damaged { why, .. } => Error::Damaged { file: rel.clone(), why },
            other => other,
        })?;
        if Area::ALL.iter().any(|a| *a != Area::Manuscript && !binder.area(*a).is_empty()) {
            return Err(Error::Damaged { file: rel, why: "a draft holds only a manuscript".into() });
        }
        src.rendered(&rel, files::write_binder(&binder)?.as_bytes());
        let mut body = wordsmith_manuscript_model::drafts::DraftBody { binder, ..Default::default() };
        let rows: Vec<_> = body.binder.rows().into_iter().filter(|r| r.item.kind.has_text()).map(|r| r.item.id).collect();
        for id in rows {
            let rel = format!("drafts/{}/text/{id}.md", info.id);
            let Some(text) = read(src, &rel, limits::MAX_TEXT_FILE)? else { continue };
            count(&text)?;
            let t = files::read_item(&text, &rel)?;
            src.rendered(&rel, if t.is_empty() { Vec::new() } else { files::write_item(&t)?.into_bytes() }.as_slice());
            body.texts.insert(id, t);
        }
        project.drafts.insert(info.id.clone(), body);
    }
    let ids: Vec<_> = project.binder.rows().into_iter().map(|r| r.item.id).collect();
    for id in ids {
        let dir = format!("snapshots/{id}");
        let names: Vec<String> =
            src.list(&dir)?.into_iter().filter(|n| n.strip_suffix(".md").is_some_and(wordsmith_manuscript_model::snapshot::is_stamp)).collect();
        if names.len() > wordsmith_manuscript_model::snapshot::MAX_PER_ITEM {
            return Err(Error::TooLarge { what: dir, limit: wordsmith_manuscript_model::snapshot::MAX_PER_ITEM as u64 });
        }
        for name in names {
            let rel = format!("{dir}/{name}");
            let Some(text) = read(src, &rel, limits::MAX_TEXT_FILE)? else { continue };
            count(&text)?;
            let taken = name.trim_end_matches(".md");
            let snap = files::read_snapshot(&text, taken, &rel)?;
            src.rendered(&rel, files::write_snapshot(&snap)?.as_bytes());
            project.snapshots.entry(id).or_default().push(snap);
        }
        if let Some(list) = project.snapshots.get_mut(&id) {
            list.sort_by_key(|s| wordsmith_manuscript_model::snapshot::order_key(&s.taken));
        }
    }
    Ok(project)
}

/// A project's files, as a save writes them.
pub(crate) fn rendered(project: &Project) -> Result<Files, Error> {
    Ok(files::render(project)?.into_iter().collect())
}

/// What opening a folder says when its `project.toml` is refused, in Manuscript's words: the
/// core's folder layer reads only that file before [`load`] reads the rest.
fn manifest_error(e: jcj_project::Error) -> Error {
    match e {
        jcj_project::Error::TooLarge(_) => Error::TooLarge { what: "project.toml".into(), limit: limits::MAX_PROJECT_FILE },
        jcj_project::Error::Damaged(why) => {
            Error::Damaged { file: "project.toml".into(), why: why.strip_prefix("the manifest: ").map_or(why.clone(), str::to_owned) }
        }
        e => e.into(),
    }
}

/// An open project folder: where it is and what its files held when read or last saved.
#[derive(Clone, Debug)]
pub struct Folder {
    inner: core::Folder,
}

impl Folder {
    /// The folder's path.
    pub fn root(&self) -> &Path {
        self.inner.root()
    }

    /// What is known about a file, when it was read or saved.
    pub fn stamp(&self, rel: &str) -> Option<Stamp> {
        self.inner.stamp(rel)
    }

    /// Open the project in `root`. Nothing is written.
    pub fn open(root: &Path) -> Result<(Folder, Project), Error> {
        let mut inner = core::Folder::open(root, &FORMAT).map_err(manifest_error)?;
        let project = load(&mut inner)?;
        Ok((Folder { inner }, project))
    }

    /// Make a new project folder at `root` (which must not hold a project yet) and save into it.
    pub fn create(root: &Path, project: &Project) -> Result<Folder, Error> {
        if root.join(FORMAT.manifest).exists() {
            return Err(Error::Io { file: FORMAT.manifest.into(), why: "a project already exists here".into() });
        }
        Ok(Folder { inner: core::Folder::create(root, &FORMAT, &rendered(project)?)? })
    }

    /// The files changed, added or removed outside the app since they were read or last saved:
    /// `project.toml`, `binder.toml` and the text and entry files (FIL-4).
    pub fn outside_changes(&self) -> Result<Vec<OutsideChange>, Error> {
        Ok(self.inner.outside_changes(&watched)?)
    }

    /// Record what a file holds now: on disk (`disk`), and what the model renders it to
    /// (`model`). `None` forgets the file, so the next save writes it.
    pub fn accept(&mut self, rel: &str, stamp: Option<Stamp>) {
        self.inner.accept(rel, stamp);
    }

    /// Save the project into its folder.
    pub fn save(&mut self, project: &Project) -> Result<Saved, Error> {
        Ok(self.inner.save(&rendered(project)?)?)
    }

    /// Save through a sink. On an error the files written so far stay written and the folder
    /// remembers them, so the next save picks up where this one stopped.
    pub fn save_to(&mut self, project: &Project, sink: &mut dyn Sink) -> Result<Saved, Error> {
        Ok(self.inner.save_to(&rendered(project)?, sink)?)
    }

    /// Save with recovery around it: the unsaved state goes to `.recovery/` first, the folder is
    /// saved, and the recovery copy is removed only after the save succeeded.
    pub fn save_safely(&mut self, project: &Project) -> Result<Saved, Error> {
        Ok(self.inner.save_safely(&rendered(project)?)?)
    }
}
