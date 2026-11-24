// SPDX-License-Identifier: MIT OR Apache-2.0
//! The standard manuscript layout, PDF and DOCX.

use wordsmith_manuscript_format::dialect;
use wordsmith_manuscript_io::compile::Options;
use wordsmith_manuscript_io::standard::{self, Standard};
use wordsmith_manuscript_model::{Area, Item, ItemId, ItemText, Kind, Place, Project, Role};

pub fn sample() -> Project {
    let mut p = Project::new();
    p.settings.book.title = "The Lighthouse Keeper".into();
    p.settings.book.short_title = "Lighthouse".into();
    p.settings.book.author = "Ada Q. Writer".into();
    p.settings.book.contact = vec!["12 Harbor Road".into(), "Port Town".into()];
    let mut n = 0;
    for c in 1..=3u64 {
        n += 1;
        let ch = Item::with_id(ItemId::from_counter(n), Kind::Folder, format!("Chapter {c}")).role(Role::Chapter);
        let cid = ch.id;
        p.binder.insert(Place::end_of(Area::Manuscript), ch).unwrap();
        for s in 1..=2u64 {
            n += 1;
            let sid = ItemId::from_counter(n);
            p.binder.insert(Place::inside(Area::Manuscript, cid), Item::with_id(sid, Kind::Text, format!("Scene {s}")).role(Role::Scene)).unwrap();
            let para =
                "The keeper climbed the long stair as the *light* turned over the bay and the gulls settled on the rail below the lamp room window.";
            let text = (0..12).map(|_| para).collect::<Vec<_>>().join("\n\n");
            p.texts.insert(sid, ItemText::with_body(dialect::parse(&text)));
        }
    }
    p
}

#[test]
fn the_pdf_holds_every_word_with_headers_and_a_title_page() {
    let p = sample();
    let s = Standard { created: "2000-01-01T00:00:00".into(), ..Standard::default() };
    let out = standard::pdf(&p, &Options { end_mark: true, ..Options::default() }, &s).unwrap();
    let pages = standard::pdf_pages(&out.bytes).unwrap();
    for (n, pg) in pages.iter().take(3).enumerate() {
        eprintln!("--- page {n}");
        for l in pg.iter().take(4) {
            eprintln!("{l}");
        }
    }
    assert!(out.check.passed(), "{}", out.check);
    assert!(out.pages > 6);
    assert!(pages[0][0].contains("about"), "the title page's first line has the count: {}", pages[0][0]);
    let squash = |s: &str| s.chars().filter(|c| !c.is_whitespace()).collect::<String>();
    assert_eq!(squash(&pages[1][0]), "Writer/LIGHTHOUSE/1", "the first page of text is page 1");
    assert_eq!(squash(&pages[2][0]), "Writer/LIGHTHOUSE/2");
}

#[test]
fn the_docx_holds_every_word() {
    let p = sample();
    let out = standard::docx(&p, &Options::default(), &Standard::default()).unwrap();
    assert!(out.check.passed(), "{}", out.check);
}
