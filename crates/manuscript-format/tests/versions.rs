// SPDX-License-Identifier: MIT OR Apache-2.0
//! Every schema reads (fixtures of schemas 1 and 2, written for the test), and drafts (schema 3)
//! save and read back.

use std::path::{Path, PathBuf};

use wordsmith_manuscript_format::dialect;
use wordsmith_manuscript_format::folder::Folder;
use wordsmith_manuscript_model::{ItemId, Text, drafts};

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

fn fixture(schema: u32, name: &str) -> PathBuf {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../../tests/fixtures/projects/schema-{schema}/The Lighthouse.wsm"));
    let dst = std::env::temp_dir().join(format!("wsm-versions-{name}-{}", std::process::id())).join("The Lighthouse.wsm");
    let _ = std::fs::remove_dir_all(dst.parent().unwrap());
    copy(&src, &dst);
    dst
}

fn id(n: u64) -> ItemId {
    ItemId::from_counter(n)
}

#[test]
fn schema_1_and_2_projects_open_and_save_as_3() {
    for schema in [1, 2] {
        let root = fixture(schema, &format!("s{schema}"));
        let (mut folder, p) = Folder::open(&root).unwrap();
        assert_eq!(p.settings.book.title, "The Lighthouse");
        assert_eq!(p.texts[&id(1)].meta.synopsis, "Arrival at the rock.");
        assert_eq!(p.texts[&id(2)].meta.comments["c1"].text, "Who is she?");
        assert_eq!(p.entries[&id(21)].fields["Age"], "34");
        let words = p.texts[&id(1)].body.plain_paragraphs();
        assert_eq!(words, ["The keeper climbed the rock at dusk.", "The sea was quiet."]);
        assert_eq!(dialect::write(&p.texts[&id(1)].body).contains("{rev=0}"), schema == 2);
        assert!(p.settings.drafts.is_empty());
        // Saved: the files that did not change are left as they were; the project says 3.
        let before = std::fs::read(root.join(format!("text/{}.md", id(2)))).unwrap();
        let mut changed = p.clone();
        changed.settings.book.subtitle = "A Story".into();
        folder.save(&changed).unwrap();
        assert!(std::fs::read_to_string(root.join("project.toml")).unwrap().starts_with("schema = 3"));
        assert_eq!(std::fs::read(root.join(format!("text/{}.md", id(2)))).unwrap(), before);
        let (_, back) = Folder::open(&root).unwrap();
        assert_eq!(back, changed);
        let _ = std::fs::remove_dir_all(root.parent().unwrap());
    }
}

#[test]
fn drafts_save_and_read_back() {
    let root = fixture(1, "drafts");
    let (mut folder, mut p) = Folder::open(&root).unwrap();
    drafts::start_new(&mut p, "Second Draft", "0190a5c2-b7d0-7c3e-8d2f-1a2b3c4d5e6f", "20261124T100000Z").unwrap();
    p.text_mut(id(1)).body = Text::from_paragraphs(&["The keeper ran up the rock."]);
    folder.save(&p).unwrap();
    let d = "drafts/0190a5c2-b7d0-7c3e-8d2f-1a2b3c4d5e6f";
    assert!(root.join(d).join("binder.toml").exists());
    assert!(root.join(d).join(format!("text/{}.md", id(1))).exists());
    let (_, back) = Folder::open(&root).unwrap();
    assert_eq!(back, p);
    assert_eq!(
        back.drafts.values().next().unwrap().texts[&id(1)].body.plain_paragraphs(),
        ["The keeper climbed the rock at dusk.", "The sea was quiet."]
    );
    // Deleting the draft removes its files at the next save.
    let mut gone = back.clone();
    drafts::delete(&mut gone, "0190a5c2-b7d0-7c3e-8d2f-1a2b3c4d5e6f").unwrap();
    let (mut folder, _) = Folder::open(&root).unwrap();
    folder.save(&gone).unwrap();
    assert!(!root.join(d).join("binder.toml").exists());
    // A hand-made bad draft id is refused.
    let t = std::fs::read_to_string(root.join("project.toml")).unwrap();
    std::fs::write(root.join("project.toml"), format!("{t}\n[[drafts]]\nid = \"../x\"\nname = \"Bad\"\n")).unwrap();
    assert!(Folder::open(&root).is_err());
    let _ = std::fs::remove_dir_all(root.parent().unwrap());
}
