// SPDX-License-Identifier: MIT OR Apache-2.0
//! Text ↔ document: every construct, random texts, and formatting the dialect cannot hold.

use proptest::prelude::*;

use super::*;

fn round(t: Text) {
    let mut t = t;
    t.normalize();
    let doc = to_document(&t);
    assert_eq!(from_document(&doc), t);
}

fn span(s: &str) -> Inline {
    Inline::plain(s)
}

#[test]
fn every_construct_survives_the_document() {
    round(Text::new());
    round(Text::from_paragraphs(&[""]));
    round(Text {
        blocks: vec![
            Block::Heading { level: 2, content: vec![span("Epigraph")] },
            Block::Para(vec![
                span("A "),
                Inline::marked("bold", Marks::bold()),
                span(" and "),
                Inline::marked(
                    "very",
                    Marks {
                        italic: true,
                        small_caps: true,
                        strike: true,
                        lang: Some("fr".into()),
                        link: Some("wsm:x".into()),
                        comment: Some("c1".into()),
                        bold: false,
                    },
                ),
                Inline::NoteRef { label: "1".into(), marks: Marks::default() },
                span(" line\nbreak."),
            ]),
            Block::SceneBreak,
            Block::Quote(vec![vec![span("q1")], vec![span("q2")]]),
            Block::Quote(vec![vec![span("another quote")]]),
            Block::Fenced { kind: Fence::Verse, paras: vec![vec![span("a\nb")], vec![span("c")]] },
            Block::Fenced { kind: Fence::Aside(Some("letter".into())), paras: vec![] },
            Block::Fenced { kind: Fence::Aside(None), paras: vec![vec![]] },
            Block::Image { alt: "map".into(), src: "media/m.png".into(), role: ImageRole::Wide },
            Block::Raw { format: "html".into(), text: "<hr>".into() },
            Block::Para(vec![]),
        ],
        notes: vec![Note { label: "1".into(), paras: vec![vec![span("Source.")], vec![span("More.")]] }, Note { label: "2".into(), paras: vec![] }],
    });
}

#[test]
fn formatting_outside_the_dialect_is_dropped_but_words_stay() {
    let mut doc = to_document(&Text::from_paragraphs(&["Plain words here"]));
    if let Some(jcj_doc::Block::Para(p)) = doc.body.first_mut().map(std::sync::Arc::make_mut) {
        p.format(0, 5, &|c: &mut CharProps| {
            c.font = Some("Some Face".into());
            c.size = Some(30.0);
            c.italic = Some(true);
        })
        .unwrap();
    }
    let t = from_document(&doc);
    assert_eq!(t.blocks, vec![Block::Para(vec![Inline::marked("Plain", Marks::italic()), span(" words here")])]);
}

#[test]
fn words_typed_into_a_scene_break_are_kept() {
    let mut doc = to_document(&Text { blocks: vec![Block::SceneBreak], notes: vec![] });
    if let Some(jcj_doc::Block::Para(p)) = doc.body.first_mut().map(std::sync::Arc::make_mut) {
        p.insert_text(0, "typed", &CharProps::default()).unwrap();
    }
    assert_eq!(from_document(&doc).blocks, vec![Block::SceneBreak, Block::Para(vec![span("typed")])]);
}

fn arb_marks() -> impl Strategy<Value = Marks> {
    (
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        proptest::option::of("[a-z]{1,3}"),
        proptest::option::of("wsm:[a-z]{1,4}"),
        proptest::option::of("c[0-9]"),
    )
        .prop_map(|(italic, bold, small_caps, strike, lang, link, comment)| Marks { italic, bold, small_caps, strike, lang, link, comment })
}

fn arb_inlines() -> impl Strategy<Value = Inlines> {
    proptest::collection::vec(
        prop_oneof![
            6 => ("[a-zé \n—*]{0,8}", arb_marks()).prop_map(|(text, marks)| Inline::Span { text, marks }),
            1 => ("[0-9]{1,2}", arb_marks()).prop_map(|(label, marks)| Inline::NoteRef { label, marks }),
        ],
        0..5,
    )
}

fn arb_text() -> impl Strategy<Value = Text> {
    let block = prop_oneof![
        5 => arb_inlines().prop_map(Block::Para),
        1 => (1u8..4, arb_inlines()).prop_map(|(level, content)| Block::Heading { level, content }),
        1 => Just(Block::SceneBreak),
        1 => proptest::collection::vec(arb_inlines(), 1..3).prop_map(Block::Quote),
        1 => (any::<bool>(), proptest::collection::vec(arb_inlines(), 0..3)).prop_map(|(v, paras)| Block::Fenced { kind: if v { Fence::Verse } else { Fence::Aside(Some("note".into())) }, paras }),
        1 => "[a-z]{0,4}".prop_map(|text| Block::Raw { format: "html".into(), text }),
    ];
    (proptest::collection::vec(block, 0..8), proptest::collection::vec(("[0-9]{1,2}", proptest::collection::vec(arb_inlines(), 0..3)), 0..3))
        .prop_map(|(blocks, notes)| Text { blocks, notes: notes.into_iter().map(|(label, paras)| Note { label, paras }).collect() })
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 2000, ..ProptestConfig::default() })]

    #[test]
    fn random_texts_survive_the_document(t in arb_text()) {
        let mut t = t;
        t.normalize();
        prop_assert_eq!(from_document(&to_document(&t)), t);
    }
}
