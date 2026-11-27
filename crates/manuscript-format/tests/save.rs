// SPDX-License-Identifier: MIT OR Apache-2.0
//! Saving safely: crash injection, packing, damaged packed projects, recovery and backups.

use std::collections::BTreeMap;
use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use jcj_project::folder::{Format, Limits};
use wordsmith_manuscript_format::Error;
use wordsmith_manuscript_format::folder::{Disk, FORMAT, Folder, Sink};
use wordsmith_manuscript_format::pack;
use wordsmith_manuscript_model::{Area, Item, ItemId, ItemText, Kind, Place, Project, Role, Text};

fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("save-tests").join(name);
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn id(n: u64) -> ItemId {
    ItemId::from_counter(n)
}

fn old_project() -> Project {
    let mut p = Project::new();
    p.settings.book.title = "Tide".into();
    let ch = Item::with_id(id(1), Kind::Folder, "One").role(Role::Chapter);
    let kids = (2..6).map(|n| Item::with_id(id(n), Kind::Text, format!("Scene {n}")).role(Role::Scene)).collect();
    p.binder.insert(Place::end_of(Area::Manuscript), ch.with_children(kids)).unwrap();
    for n in 2..6 {
        p.texts.insert(id(n), ItemText::with_body(Text::from_paragraphs(&[format!("Old text of scene {n}.")])));
    }
    p
}

fn new_project() -> Project {
    let mut p = old_project();
    p.settings.book.title = "The Tide".into();
    for n in [2, 4] {
        p.texts.insert(id(n), ItemText::with_body(Text::from_paragraphs(&[format!("New text of scene {n}.")])));
    }
    p.binder.insert(Place::inside(Area::Manuscript, id(1)), Item::with_id(id(9), Kind::Text, "Added")).unwrap();
    p.texts.insert(id(9), ItemText::with_body(Text::from_paragraphs(&["A new scene."])));
    p.binder.move_to(id(5), Place { area: Area::Manuscript, parent: Some(id(1)), index: 0 }).unwrap();
    p.binder.trash(id(3)).unwrap();
    p.binder.empty_trash();
    p.prune();
    p
}

/// Writes through to disk until it has written `left` files, then fails as if the app stopped.
struct Crash {
    disk: Disk,
    left: usize,
}

impl Sink for Crash {
    fn write(&mut self, rel: &str, bytes: &[u8]) -> jcj_project::Result<()> {
        if self.left == 0 {
            return Err(jcj_project::Error::Invalid(format!("{rel}: stopped")));
        }
        self.left -= 1;
        self.disk.write(rel, bytes)
    }

    fn remove(&mut self, rel: &str) -> jcj_project::Result<()> {
        if self.left == 0 {
            return Err(jcj_project::Error::Invalid(format!("{rel}: stopped")));
        }
        self.left -= 1;
        self.disk.remove(rel)
    }
}

#[test]
fn a_save_stopped_after_any_file_reopens_whole_with_the_work_recoverable() {
    let old = old_project();
    let new = new_project();
    // How many writes and removals a full save makes.
    let full = {
        let dir = scratch("crash-count");
        let mut f = Folder::create(&dir, &old).unwrap();
        let s = f.save(&new).unwrap();
        s.written.len() + s.removed.len()
    };
    assert!(full >= 4);
    for k in 0..full {
        let dir = scratch(&format!("crash-{k}"));
        let mut folder = Folder::create(&dir, &old).unwrap();
        pack::write_recovery(&dir, &new).unwrap();
        let mut sink = Crash { disk: Disk::new(&dir), left: k };
        assert!(folder.save_to(&new, &mut sink).is_err(), "save {k} should stop");
        // The folder reopens, every file whole.
        let (_, reopened) = Folder::open(&dir).unwrap_or_else(|e| panic!("after {k} writes: {e}"));
        // Every text is either the last good save's or the new one's.
        for (item, text) in &reopened.texts {
            let o = old.texts.get(item);
            let n = new.texts.get(item);
            assert!(Some(text) == o || Some(text) == n, "after {k} writes, {item} holds neither version");
        }
        // The structure is the old binder or the new one.
        assert!(reopened.binder == old.binder || reopened.binder == new.binder, "after {k} writes the binder is mixed");
        // And the unsaved work is there to recover in full.
        assert_eq!(pack::recovered(&dir).as_ref(), Some(&new));
    }
}

#[test]
fn a_good_save_removes_the_recovery_copy() {
    let dir = scratch("recovery");
    let mut folder = Folder::create(&dir, &old_project()).unwrap();
    pack::save_safely(&mut folder, &new_project()).unwrap();
    assert!(pack::recovered(&dir).is_none());
    assert!(!dir.join(".recovery").exists());
    assert_eq!(Folder::open(&dir).unwrap().1, new_project());
}

#[test]
fn packing_is_deterministic_and_round_trips() {
    let (a, b) = (scratch("pack-a"), scratch("pack-b"));
    Folder::create(&a, &new_project()).unwrap();
    Folder::create(&b, &new_project()).unwrap();
    std::fs::create_dir_all(a.join("media")).unwrap();
    std::fs::create_dir_all(b.join("media")).unwrap();
    std::fs::write(a.join("media/x.png"), b"not really a picture").unwrap();
    std::fs::write(b.join("media/x.png"), b"not really a picture").unwrap();
    pack::write_recovery(&a, &old_project()).unwrap();
    let pa = pack::pack_folder(&a).unwrap();
    let pb = pack::pack_folder(&b).unwrap();
    assert_eq!(pa, pb, "same project, same bytes (recovery left out)");
    assert_eq!(pack::pack_folder(&a).unwrap(), pa);
    assert_eq!(&pa[30..38], b"mimetype", "the mimetype entry is first");
    assert_eq!(pack::read_packed(&pa).unwrap(), new_project());
    let dest = scratch("unpacked");
    let (_, p) = pack::unpack_into(&pa, &dest).unwrap();
    assert_eq!(p, new_project());
    assert_eq!(std::fs::read(dest.join("media/x.png")).unwrap(), b"not really a picture");
    assert!(pack::unpack_into(&pa, &dest).is_err(), "never unpacks over a folder with files");
}

fn zip_of(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut z = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, data) in entries {
        z.start_file(*name, zip::write::SimpleFileOptions::default()).unwrap();
        z.write_all(data).unwrap();
    }
    z.finish().unwrap().into_inner()
}

#[test]
fn damaged_and_hostile_packed_projects_are_refused_cleanly() {
    let good = pack::pack_project(&new_project()).unwrap();
    assert_eq!(pack::read_packed(b"definitely not a zip").unwrap_err(), Error::NotAProject);
    assert!(pack::read_packed(&good[..good.len() / 2]).is_err());
    assert!(pack::read_packed(&[]).is_err());
    let mut flipped = good.clone();
    for b in flipped.iter_mut().skip(60).step_by(97) {
        *b ^= 0x5a;
    }
    assert!(pack::read_packed(&flipped).is_err());
    let mime = pack::MIMETYPE.as_bytes();
    let project = pack::unpack(&good).unwrap();
    let toml = project.get("project.toml").unwrap();
    let binder = project.get("binder.toml").unwrap();
    for evil in ["../escape.md", "/etc/passwd", "text/../../escape.md", "C:\\x", "text\\..\\x", "other.txt", "text//x.md"] {
        let z = zip_of(&[("mimetype", mime), ("project.toml", toml), ("binder.toml", binder), (evil, b"x")]);
        let dest = scratch("evil");
        assert!(pack::unpack_into(&z, &dest).is_err(), "{evil} must be refused");
        assert!(!dest.exists() || std::fs::read_dir(&dest).unwrap().next().is_none(), "nothing written for {evil}");
    }
    let wrong_mime = zip_of(&[("mimetype", b"application/zip"), ("project.toml", toml)]);
    assert_eq!(pack::read_packed(&wrong_mime).unwrap_err(), Error::NotAProject);
    let no_mime = zip_of(&[("project.toml", toml)]);
    assert_eq!(pack::read_packed(&no_mime).unwrap_err(), Error::NotAProject);
}

#[test]
fn every_packing_limit_is_enforced() {
    // Manuscript's bounds are the format's, and the core's reader holds a packed project to them.
    let lim = FORMAT.limits;
    assert_eq!((lim.packed, lim.entry, lim.entries, lim.unpacked), (pack::MAX_PACKED, pack::MAX_ENTRY, pack::MAX_ENTRIES, pack::MAX_UNPACKED));
    let good = pack::pack_project(&new_project()).unwrap();
    let over = |l: Limits| {
        let format = Format { limits: l, ..FORMAT };
        matches!(jcj_project::folder::unpack(&format, &good), Err(jcj_project::Error::TooLarge(_)))
    };
    assert!(over(Limits { packed: 100, ..lim }));
    assert!(over(Limits { entries: 2, ..lim }));
    assert!(over(Limits { entry: 10, ..lim }));
    assert!(over(Limits { unpacked: 50, ..lim }));
    // A highly compressible entry is cut off at the limit, whatever its header says.
    let big = vec![b'a'; 4096];
    let z = zip_of(&[("mimetype", pack::MIMETYPE.as_bytes()), ("text/x.md", &big)]);
    let format = Format { limits: Limits { entry: 1000, ..lim }, ..FORMAT };
    assert!(matches!(jcj_project::folder::unpack(&format, &z), Err(jcj_project::Error::TooLarge(_))));
    // Over a limit, the error says so in a sentence.
    let said = pack::read_packed(&zip_of(&[("mimetype", pack::MIMETYPE.as_bytes()), ("../x", b"x")])).unwrap_err().to_string();
    assert!(said.contains("not the project's"), "{said}");
}

#[test]
fn backups_rotate_keeping_the_newest() {
    let dir = scratch("backed");
    Folder::create(&dir, &old_project()).unwrap();
    let store = scratch("backups");
    let t0 = SystemTime::UNIX_EPOCH + Duration::from_secs(1_800_000_000);
    let mut made = Vec::new();
    for n in 0..5 {
        made.push(pack::backup(&dir, &store, "Tide", 3, t0 + Duration::from_secs(n * 60)).unwrap());
    }
    let kept = pack::backups(&store, "Tide");
    assert_eq!(kept.len(), 3);
    assert_eq!(kept[0], made[4]);
    assert_eq!(pack::read_packed(&std::fs::read(&kept[0]).unwrap()).unwrap(), old_project());
    let _: BTreeMap<String, Vec<u8>> = pack::unpack(&std::fs::read(&kept[2]).unwrap()).unwrap();
}

#[test]
fn a_second_backup_in_one_second_is_kept_beside_the_first() {
    let dir = scratch("backed-twice");
    Folder::create(&dir, &old_project()).unwrap();
    let store = scratch("backups-twice");
    let at = SystemTime::UNIX_EPOCH + Duration::from_secs(1_800_000_000);
    let first = pack::backup(&dir, &store, "Tide", 3, at).unwrap();
    let second = pack::backup(&dir, &store, "Tide", 3, at).unwrap();
    assert_ne!(first, second, "an existing backup is never replaced");
    assert_eq!(second.file_name().unwrap().to_string_lossy(), "Tide 2027-01-15 08.00.00 UTC 2.wsmz");
    // Newest first, and a file named by hand is not one of the project's backups.
    std::fs::write(store.join("Tide copy.wsmz"), b"mine").unwrap();
    assert_eq!(pack::backups(&store, "Tide"), [second, first]);
}
