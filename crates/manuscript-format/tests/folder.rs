// SPDX-License-Identifier: MIT OR Apache-2.0
//! The project folder: round trips, changed-files-only saving, stable bytes, limits and damage.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use proptest::prelude::*;
use wordsmith_manuscript_format::folder::Folder;
use wordsmith_manuscript_format::{Error, limits};
use wordsmith_manuscript_model::{Area, Block, Entry, Inline, Item, ItemId, ItemText, Kind, Marks, Place, Project, Role, Text};

fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("folder-tests").join(name);
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn files_of(dir: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut out = BTreeMap::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for e in std::fs::read_dir(&d).unwrap().flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else {
                out.insert(p.strip_prefix(dir).unwrap().to_string_lossy().replace('\\', "/"), std::fs::read(&p).unwrap());
            }
        }
    }
    out
}

fn id(n: u64) -> ItemId {
    ItemId::from_counter(n)
}

fn sample() -> Project {
    let mut p = Project::new();
    p.settings.book.title = "The Lighthouse".into();
    p.settings.book.author = "Test Author".into();
    let ch = Item::with_id(id(1), Kind::Folder, "Chapter One").role(Role::Chapter);
    let s1 = Item::with_id(id(2), Kind::Text, "Arrival").role(Role::Scene);
    let s2 = Item::with_id(id(3), Kind::Text, "Storm").role(Role::Scene);
    p.binder.insert(Place::end_of(Area::Manuscript), ch.with_children(vec![s1, s2])).unwrap();
    p.binder.insert(Place::end_of(Area::Bible), Item::with_id(id(4), Kind::Entry, "Mara")).unwrap();
    p.binder.insert(Place::end_of(Area::Notes), Item::with_id(id(5), Kind::Text, "Plan")).unwrap();
    let mut t = ItemText::with_body(Text {
        blocks: vec![
            Block::Para(vec![Inline::plain("The keeper climbed the "), Inline::marked("long", Marks::italic()), Inline::plain(" stair.")]),
            Block::SceneBreak,
            Block::Para(vec![Inline::plain("Morning.")]),
        ],
        notes: vec![],
    });
    t.meta.synopsis = "The keeper arrives.".into();
    p.texts.insert(id(2), t);
    p.texts.insert(id(3), ItemText::with_body(Text::from_paragraphs(&["Rain hammered the glass."])));
    p.texts.insert(id(5), ItemText::with_body(Text::from_paragraphs(&["A plan."])));
    let mut e = Entry::new("character", "Mara");
    e.fields.insert("Role".into(), "Keeper".into());
    p.entries.insert(id(4), e);
    p
}

#[test]
fn a_project_round_trips_through_its_folder() {
    let dir = scratch("round-trip");
    let p = sample();
    Folder::create(&dir, &p).unwrap();
    let (_, back) = Folder::open(&dir).unwrap();
    assert_eq!(back, p);
    assert!(dir.join("text").join(format!("{}.md", id(2))).exists());
    assert!(dir.join("bible").join(format!("{}.md", id(4))).exists());
}

#[test]
fn a_project_saved_twice_gives_the_same_bytes() {
    let (a, b) = (scratch("twice-a"), scratch("twice-b"));
    Folder::create(&a, &sample()).unwrap();
    Folder::create(&b, &sample()).unwrap();
    assert_eq!(files_of(&a), files_of(&b));
    let (mut f, p) = Folder::open(&a).unwrap();
    let saved = f.save(&p).unwrap();
    assert!(saved.written.is_empty(), "nothing changed, nothing written: {saved:?}");
    assert_eq!(files_of(&a), files_of(&b));
}

#[test]
fn a_change_to_one_scene_writes_one_file() {
    let dir = scratch("one-file");
    Folder::create(&dir, &sample()).unwrap();
    let (mut f, mut p) = Folder::open(&dir).unwrap();
    p.text_mut(id(3)).body = Text::from_paragraphs(&["Rain hammered the glass all night."]);
    let saved = f.save(&p).unwrap();
    assert_eq!(saved.written, vec![format!("text/{}.md", id(3))]);
    // Renaming changes the binder only.
    p.binder.get_mut(id(3)).unwrap().title = "The Storm".into();
    assert_eq!(f.save(&p).unwrap().written, vec!["binder.toml".to_owned()]);
}

#[test]
fn hand_written_text_is_not_rewritten_unless_it_changes() {
    let dir = scratch("hand-written");
    Folder::create(&dir, &sample()).unwrap();
    let rel = format!("text/{}.md", id(5));
    let hand = "A plan,   with _underscores_ and a\r\nwindows line end.\r\n";
    std::fs::write(dir.join(&rel), hand).unwrap();
    let (mut f, p) = Folder::open(&dir).unwrap();
    assert!(f.save(&p).unwrap().written.is_empty());
    assert_eq!(std::fs::read_to_string(dir.join(&rel)).unwrap(), hand);
}

#[test]
fn emptied_trash_removes_files_after_the_binder() {
    let dir = scratch("trash");
    Folder::create(&dir, &sample()).unwrap();
    let (mut f, mut p) = Folder::open(&dir).unwrap();
    p.binder.trash(id(5)).unwrap();
    f.save(&p).unwrap();
    assert!(dir.join(format!("text/{}.md", id(5))).exists(), "the Trash keeps the file");
    p.binder.empty_trash();
    p.prune();
    let saved = f.save(&p).unwrap();
    assert_eq!(saved.written, vec!["binder.toml".to_owned()]);
    assert_eq!(saved.removed, vec![format!("text/{}.md", id(5))]);
    assert!(!dir.join(format!("text/{}.md", id(5))).exists());
}

#[test]
fn a_newer_project_is_refused() {
    let dir = scratch("newer");
    Folder::create(&dir, &sample()).unwrap();
    let path = dir.join("project.toml");
    let t = std::fs::read_to_string(&path).unwrap().replacen("schema = 2", "schema = 3", 1);
    std::fs::write(&path, t).unwrap();
    let err = Folder::open(&dir).unwrap_err();
    assert_eq!(err, Error::Newer { found: 3, supported: 2 });
}

#[test]
fn a_schema_1_project_opens_unchanged() {
    // Schema 2 only adds revision marks: a version 1 project reads as it is and is written as 2.
    let dir = scratch("schema1");
    Folder::create(&dir, &sample()).unwrap();
    for f in ["project.toml", "binder.toml"] {
        let path = dir.join(f);
        let t = std::fs::read_to_string(&path).unwrap().replacen("schema = 2", "schema = 1", 1);
        std::fs::write(&path, t).unwrap();
    }
    let (_, p) = Folder::open(&dir).unwrap();
    assert_eq!(p, sample());
}

#[test]
fn a_folder_without_a_project_is_not_a_project() {
    let dir = scratch("empty");
    std::fs::create_dir_all(&dir).unwrap();
    assert_eq!(Folder::open(&dir).unwrap_err(), Error::NotAProject);
}

#[test]
fn files_past_each_limit_are_refused() {
    let dir = scratch("limits");
    Folder::create(&dir, &sample()).unwrap();
    // project.toml
    let big = format!("schema = 1\n# {}\n", "x".repeat(limits::MAX_PROJECT_FILE as usize));
    std::fs::write(dir.join("project.toml"), big).unwrap();
    assert!(matches!(Folder::open(&dir), Err(Error::TooLarge { .. })));
    Folder::create(&scratch("limits"), &sample()).unwrap();
    // A text file.
    let rel = dir.join(format!("text/{}.md", id(2)));
    let f = std::fs::File::create(&rel).unwrap();
    f.set_len(limits::MAX_TEXT_FILE + 1).unwrap();
    assert!(matches!(Folder::open(&dir), Err(Error::TooLarge { .. })));
    // Front matter.
    let fm = format!("+++\nsynopsis = \"{}\"\n+++\n", "y".repeat(limits::MAX_FRONT_MATTER));
    std::fs::write(&rel, fm).unwrap();
    assert!(matches!(Folder::open(&dir), Err(Error::TooLarge { .. })));
    // Too many items.
    let mut binder = String::from("schema = 1\n");
    for n in 0..=limits::MAX_ITEMS as u64 {
        binder.push_str(&format!("[[item]]\narea = \"notes\"\nid = \"{}\"\n", ItemId::from_counter(n + 1000)));
    }
    std::fs::write(dir.join("binder.toml"), binder).unwrap();
    assert!(matches!(Folder::open(&dir), Err(Error::TooLarge { .. })));
}

#[test]
fn too_deep_a_binder_and_bad_text_are_damage() {
    let dir = scratch("deep");
    Folder::create(&dir, &sample()).unwrap();
    let mut binder = String::from("schema = 1\n");
    for n in 0..40u64 {
        let parent = if n == 0 { String::new() } else { format!("parent = \"{}\"\n", ItemId::from_counter(n + 99)) };
        binder.push_str(&format!("[[item]]\narea = \"notes\"\n{parent}id = \"{}\"\nkind = \"folder\"\n", ItemId::from_counter(n + 100)));
    }
    std::fs::write(dir.join("binder.toml"), binder).unwrap();
    assert!(matches!(Folder::open(&dir), Err(Error::Damaged { .. })));
    Folder::create(&scratch("deep"), &sample()).unwrap();
    std::fs::write(dir.join(format!("text/{}.md", id(2))), [0xff, 0xfe, 0x00, b'a']).unwrap();
    assert!(matches!(Folder::open(&dir), Err(Error::Damaged { .. })));
    std::fs::write(dir.join("binder.toml"), "schema = 1\n[[item]\n").unwrap();
    assert!(matches!(Folder::open(&dir), Err(Error::Damaged { .. })));
}

fn arb_project() -> impl Strategy<Value = Project> {
    let item = (0u8..4, "[A-Za-z ]{0,10}", any::<bool>(), proptest::option::of("[a-z]{1,4}"));
    (proptest::collection::vec((item, 0usize..6, "[a-z .*\n]{0,30}", "[a-z]{0,8}"), 1..25), "[A-Za-z ]{1,12}").prop_map(|(items, title)| {
        let mut p = Project::new();
        p.settings.book.title = title;
        let mut ids = Vec::new();
        for (n, ((kind, t, include, label), parent, body, synopsis)) in items.into_iter().enumerate() {
            let kind = match kind {
                0 => Kind::Folder,
                1 | 2 => Kind::Text,
                _ => Kind::Entry,
            };
            let i = ItemId::from_counter(n as u64 + 1);
            let mut item = Item::with_id(i, kind, t);
            item.include = include;
            item.label = label;
            let area = if kind == Kind::Entry { Area::Bible } else { Area::Manuscript };
            let parent = ids.iter().filter(|(_, a, k): &&(ItemId, Area, Kind)| *a == area && *k != Kind::Entry).nth(parent).map(|(x, _, _)| *x);
            let place = parent.map_or(Place::end_of(area), |pa| Place::inside(area, pa));
            if p.binder.insert(place, item).is_err() {
                continue;
            }
            ids.push((i, area, kind));
            if kind == Kind::Entry {
                let mut e = Entry::new("place", &synopsis);
                e.body = wordsmith_manuscript_format::dialect::parse(&body);
                p.entries.insert(i, e);
            } else {
                let mut t = ItemText::with_body(wordsmith_manuscript_format::dialect::parse(&body));
                t.meta.synopsis = synopsis;
                if !t.is_empty() {
                    p.texts.insert(i, t);
                }
            }
        }
        p
    })
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 48, ..ProptestConfig::default() })]

    #[test]
    fn random_projects_round_trip_and_save_stably(p in arb_project()) {
        let dir = scratch("prop");
        Folder::create(&dir, &p).unwrap();
        let (mut f, back) = Folder::open(&dir).unwrap();
        prop_assert_eq!(&back, &p);
        let before = files_of(&dir);
        prop_assert!(f.save(&back).unwrap().written.is_empty());
        prop_assert_eq!(files_of(&dir), before);
    }
}

#[test]
fn both_templates_make_valid_projects_that_round_trip() {
    for t in wordsmith_manuscript_model::templates::TEMPLATES {
        let p = wordsmith_manuscript_model::templates::build(t.id, "Test", &mut ItemId::new).unwrap();
        let dir = scratch(&format!("template-{}", t.id));
        Folder::create(&dir, &p).unwrap();
        let (mut f, back) = Folder::open(&dir).unwrap();
        assert_eq!(back, p);
        assert!(f.save(&back).unwrap().written.is_empty());
    }
}

#[test]
fn snapshots_are_kept_beside_the_text_and_round_trip() {
    let dir = scratch("snapshots");
    let mut p = sample();
    let t0 = std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_800_000_000);
    let a = p.take_snapshot(id(2), "First pass", t0).unwrap();
    let b = p.take_snapshot(id(2), "Same second", t0).unwrap();
    assert_eq!(b, format!("{a}-2"));
    Folder::create(&dir, &p).unwrap();
    assert!(dir.join(format!("snapshots/{}/{a}.md", id(2))).exists());
    let (mut f, back) = Folder::open(&dir).unwrap();
    assert_eq!(back, p);
    assert!(f.save(&back).unwrap().written.is_empty());
    // A file in the snapshots folder that is not a stamp is ignored, never read as a snapshot.
    std::fs::write(dir.join(format!("snapshots/{}/notes.md", id(2))), "x").unwrap();
    assert_eq!(Folder::open(&dir).unwrap().1, p);
}
