// SPDX-License-Identifier: MIT OR Apache-2.0
//! The files on disk are the judge: what a save, a packed project, a recovery copy, a backup and
//! an unpacked project write is compared, byte for byte (by its SHA-256), with
//! `tests/golden/project-files.txt`.
//!
//! The reference was written by Wordsmith Manuscript 0.9.0's own folder code, before the folder
//! layer became the core's (`jcj_project::folder`, core issue #45), so this test proves that the
//! move changed no byte a writer keeps. A change that alters the reference is deliberate: rerun
//! with `UPDATE_GOLDEN=1`, review the difference, and record it in `docs/golden.md`.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use sha2::{Digest, Sha256};
use wordsmith_manuscript_format::folder::Folder;
use wordsmith_manuscript_format::pack;
use wordsmith_manuscript_model::{Area, Item, ItemId, ItemText, Kind, Place, Project, Role, Snapshot, Text, drafts};

fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("on-disk-tests").join(name);
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn id(n: u64) -> ItemId {
    ItemId::from_counter(n)
}

fn sha(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().fold(String::new(), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
    })
}

/// Every file under `root`, by its path inside it, with `/` between the parts.
fn tree(root: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut out = BTreeMap::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else {
                let rel = path.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/");
                out.insert(rel, std::fs::read(&path).unwrap());
            }
        }
    }
    out
}

fn section(out: &mut String, title: &str, files: &BTreeMap<String, Vec<u8>>) {
    let _ = writeln!(out, "[{title}]");
    for (path, bytes) in files {
        let _ = writeln!(out, "{path} {}", sha(bytes));
    }
    out.push('\n');
}

fn one(out: &mut String, title: &str, name: &str, bytes: &[u8]) {
    let _ = writeln!(out, "[{title}]\n{name} {}\n", sha(bytes));
}

/// A project written for the test with every kind of file a project folder holds: texts with
/// notes and marks, a folder's text, a bible entry, a picture, a snapshot and a stored draft.
fn project() -> Project {
    let mut p = Project::new();
    p.settings.book.title = "The Lighthouse".into();
    p.settings.book.author = "A. Writer".into();
    let m = Place::end_of(Area::Manuscript);
    p.binder.insert(m, Item::with_id(id(1), Kind::Text, "Title Page").role(Role::FrontMatter)).unwrap();
    p.binder.insert(Place::end_of(Area::Manuscript), Item::with_id(id(2), Kind::Folder, "Arrival").role(Role::Chapter)).unwrap();
    for (n, title) in [(3, "Dusk"), (4, "Night"), (5, "Dawn")] {
        p.binder.insert(Place::inside(Area::Manuscript, id(2)), Item::with_id(id(n), Kind::Text, title).role(Role::Scene)).unwrap();
    }
    p.binder.insert(Place::end_of(Area::Bible), Item::with_id(id(6), Kind::Entry, "The Keeper")).unwrap();
    let mut picture = Item::with_id(id(7), Kind::Media, "Map");
    picture.file = Some("png".into());
    p.binder.insert(Place::end_of(Area::Research), picture).unwrap();
    p.binder.insert(Place::end_of(Area::Notes), Item::with_id(id(8), Kind::Text, "Plans")).unwrap();
    p.texts.insert(id(1), ItemText::with_body(Text::from_paragraphs(&["The Lighthouse", "A. Writer"])));
    p.texts.insert(id(2), ItemText::with_body(Text::from_paragraphs(&["The chapter's own words."])));
    p.texts.insert(id(3), ItemText::with_body(Text::from_paragraphs(&["The keeper climbed the rock.", "The lamp was cold."])));
    p.texts.insert(id(4), ItemText::with_body(Text::from_paragraphs(&["The sea was quiet."])));
    p.texts.insert(id(5), ItemText::with_body(Text::from_paragraphs(&["Morning came grey."])));
    p.texts.insert(id(8), ItemText::with_body(Text::from_paragraphs(&["Write the storm."])));
    p.snapshots.entry(id(3)).or_default().push(Snapshot {
        taken: "20261124T151230Z".into(),
        name: "First go".into(),
        text: Text::from_paragraphs(&["The keeper climbed."]),
    });
    drafts::start_new(&mut p, "Second draft", "first-draft", "20261125T090000Z").unwrap();
    p
}

/// The same project after a session of work: a text changed, a scene added, one removed, a
/// second snapshot.
fn worked(mut p: Project) -> Project {
    p.texts.insert(id(4), ItemText::with_body(Text::from_paragraphs(&["The sea was loud.", "Rain came."])));
    p.binder.insert(Place::inside(Area::Manuscript, id(2)), Item::with_id(id(9), Kind::Text, "Storm").role(Role::Scene)).unwrap();
    p.texts.insert(id(9), ItemText::with_body(Text::from_paragraphs(&["The storm broke."])));
    p.binder.trash(id(5)).unwrap();
    p.binder.empty_trash();
    p.prune();
    p.snapshots.entry(id(4)).or_default().push(Snapshot {
        taken: "20261126T101500Z".into(),
        name: "Before the rain".into(),
        text: Text::from_paragraphs(&["The sea was quiet."]),
    });
    p
}

fn copy(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for e in std::fs::read_dir(from).unwrap().flatten() {
        let p = e.path();
        if p.is_dir() {
            copy(&p, &to.join(e.file_name()));
        } else {
            std::fs::copy(&p, to.join(e.file_name())).unwrap();
        }
    }
}

/// What each way of writing a project puts on disk.
fn written() -> String {
    let mut out = String::new();
    let base = scratch("written");

    // A new project folder, with a picture the writer added.
    let root = base.join("The Lighthouse.wsm");
    let first = project();
    let mut folder = Folder::create(&root, &first).unwrap();
    std::fs::create_dir_all(root.join("media")).unwrap();
    std::fs::write(root.join(format!("media/{}.png", id(7))), [0x89, b'P', b'N', b'G', 1, 2, 3]).unwrap();
    section(&mut out, "a new project folder", &tree(&root));

    // Saved again after a session of work.
    let second = worked(first);
    folder.save(&second).unwrap();
    section(&mut out, "the folder after a save", &tree(&root));

    // A text file written by hand in another form is read and kept as it is by a save.
    let by_hand = root.join(format!("text/{}.md", id(9)));
    let text = std::fs::read_to_string(&by_hand).unwrap();
    std::fs::write(&by_hand, format!("{text}\n\n")).unwrap();
    let (mut reopened, read) = Folder::open(&root).unwrap();
    reopened.save(&read).unwrap();
    section(&mut out, "the folder after a save of a file written by hand", &tree(&root));

    // The packed project, from the folder and from memory.
    one(&mut out, "the folder packed", "The Lighthouse.wsmz", &pack::pack_folder(&root).unwrap());
    one(&mut out, "the project packed from memory", "The Lighthouse.wsmz", &pack::pack_project(&second).unwrap());
    let mut third = second.clone();
    third.texts.insert(id(3), ItemText::with_body(Text::from_paragraphs(&["Unsaved words."])));
    one(&mut out, "the project packed with its folder's pictures", "The Lighthouse.wsmz", &pack::pack_current(&third, Some(&root)).unwrap());

    // The recovery copy of unsaved work.
    pack::write_recovery(&root, &third).unwrap();
    let recovery = pack::recovery_path(&root);
    let name = recovery.strip_prefix(&root).unwrap().to_string_lossy().replace('\\', "/");
    one(&mut out, "the recovery copy", &name, &std::fs::read(&recovery).unwrap());
    pack::discard_recovery(&root).unwrap();

    // A backup, at a fixed time.
    let store = base.join("Backups");
    let at = SystemTime::UNIX_EPOCH + Duration::from_secs(1_795_000_000);
    let made = pack::backup(&root, &store, "The Lighthouse", 3, at).unwrap();
    let name = made.file_name().unwrap().to_string_lossy().into_owned();
    one(&mut out, "a backup", &name, &std::fs::read(&made).unwrap());

    // A packed project unpacked into a folder.
    let dest = base.join("Unpacked.wsm");
    pack::unpack_into(&pack::pack_folder(&root).unwrap(), &dest).unwrap();
    section(&mut out, "a packed project unpacked", &tree(&dest));

    // A project of schema 1, opened and saved: it is written as schema 3.
    let old = base.join("Old.wsm");
    copy(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/projects/schema-1/The Lighthouse.wsm"), &old);
    let (mut folder, read) = Folder::open(&old).unwrap();
    folder.save(&read).unwrap();
    section(&mut out, "a schema 1 project saved", &tree(&old));
    out
}

#[test]
fn the_files_on_disk_are_the_reference_bytes() {
    let got = written();
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden/project-files.txt");
    if std::env::var_os("UPDATE_GOLDEN").is_some() {
        std::fs::write(&path, &got).unwrap();
    }
    let want = std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("{} is missing: run with UPDATE_GOLDEN=1 and review it", path.display()));
    assert_eq!(got, want, "a file a project writes changed: review it and record it in docs/golden.md");
}
