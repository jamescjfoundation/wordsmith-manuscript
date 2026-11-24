// SPDX-License-Identifier: MIT OR Apache-2.0
//! Round trips for every construct of the dialect, property tests, and hostile input.

use proptest::prelude::*;
use wordsmith_manuscript_model::{Block, Fence, ImageRole, Inline, Marks, Note, Text};

use super::{canonical, parse, write};

fn span(t: &str) -> Inline {
    Inline::plain(t)
}

fn marked(t: &str, f: impl Fn(&mut Marks)) -> Inline {
    let mut m = Marks::default();
    f(&mut m);
    Inline::marked(t, m)
}

/// model → text → model is exact, and the text it gives is the text expected.
fn check(text: Text, expected: &str) {
    let mut t = text;
    canonical(&mut t);
    let out = write(&t);
    assert_eq!(out, expected, "written form");
    assert_eq!(parse(&out), t, "read back");
    assert_eq!(write(&parse(expected)), expected, "unchanged text stays byte-identical");
}

fn paras(blocks: Vec<Block>) -> Text {
    Text { blocks, notes: Vec::new() }
}

#[test]
fn paragraphs_and_emphasis() {
    check(
        paras(vec![
            Block::Para(vec![span("It was "), marked("very", |m| m.italic = true), span(" "), marked("dark", |m| m.bold = true), span(".")]),
            Block::Para(vec![span("Second.")]),
        ]),
        "It was *very* **dark**.\n\nSecond.\n",
    );
}

#[test]
fn nested_and_adjacent_emphasis() {
    check(paras(vec![Block::Para(vec![marked("a", |m| m.italic = true), marked("b", |m| m.bold = true)])]), "*a***b**\n");
    check(paras(vec![Block::Para(vec![marked("a", |m| m.bold = true), marked("b", |m| m.italic = true)])]), "**a***b*\n");
    check(
        paras(vec![Block::Para(vec![
            marked("a", |m| m.italic = true),
            marked("b", |m| {
                m.italic = true;
                m.bold = true
            }),
            marked("c", |m| m.italic = true),
        ])]),
        "*a****b****c*\n",
    );
    check(
        paras(vec![Block::Para(vec![
            marked("x", |m| {
                m.italic = true;
                m.bold = true
            }),
            marked(" y", |m| m.bold = true),
        ])]),
        "***x* y**\n",
    );
}

#[test]
fn small_caps_strike_language_links_comments() {
    check(
        paras(vec![Block::Para(vec![
            marked("BC", |m| m.small_caps = true),
            span(" "),
            marked("gone", |m| m.strike = true),
            span(" "),
            marked("merci", |m| m.lang = Some("fr".into())),
            span(" "),
            marked("Anna", |m| m.link = Some("wsm:0190a5c2-b7d0-7c3e-8d2f-1a2b3c4d5e6f".into())),
            span(" "),
            marked("this", |m| m.comment = Some("c1".into())),
        ])]),
        "[BC]{.smallcaps} ~~gone~~ [merci]{lang=fr} [Anna](wsm:0190a5c2-b7d0-7c3e-8d2f-1a2b3c4d5e6f) [this]{.comment id=c1}\n",
    );
}

#[test]
fn marks_nest_in_a_fixed_order() {
    check(
        paras(vec![Block::Para(vec![marked("both", |m| {
            m.italic = true;
            m.link = Some("wsm:x".into());
            m.lang = Some("de".into());
        })])]),
        "[[*both*]{lang=de}](wsm:x)\n",
    );
}

#[test]
fn scene_breaks_headings_quotes() {
    check(
        paras(vec![
            Block::Heading { level: 1, content: vec![span("Dedication")] },
            Block::Para(vec![span("For the river.")]),
            Block::SceneBreak,
            Block::Quote(vec![vec![span("First.")], vec![span("Second.")]]),
            Block::Heading { level: 3, content: vec![] },
        ]),
        "# Dedication\n\nFor the river.\n\n***\n\n> First.\n>\n> Second.\n\n###\n",
    );
}

#[test]
fn verse_and_asides_keep_line_breaks() {
    check(
        paras(vec![
            Block::Fenced { kind: Fence::Verse, paras: vec![vec![span("Line one\nLine two")], vec![span("Stanza two")]] },
            Block::Fenced { kind: Fence::Aside(Some("letter".into())), paras: vec![vec![span("Dear Sir,")]] },
            Block::Fenced { kind: Fence::Aside(None), paras: vec![] },
        ]),
        "::: verse\nLine one\\\nLine two\n\nStanza two\n:::\n\n::: aside letter\nDear Sir,\n:::\n\n::: aside\n:::\n",
    );
}

#[test]
fn images_and_raw_blocks() {
    check(
        paras(vec![
            Block::Image { alt: "A map [old]".into(), src: "media/0190a5c2.png".into(), role: ImageRole::Wide },
            Block::Raw { format: "html".into(), text: "<hr>\n```inner```".into() },
            Block::Raw { format: "latex".into(), text: String::new() },
        ]),
        "![A map \\[old\\]](media/0190a5c2.png){role=wide}\n\n````raw html\n<hr>\n```inner```\n````\n\n```raw latex\n\n```\n",
    );
}

#[test]
fn notes() {
    check(
        Text {
            blocks: vec![Block::Para(vec![span("A claim"), Inline::NoteRef { label: "1".into(), marks: Marks::default() }, span(".")])],
            notes: vec![
                Note { label: "1".into(), paras: vec![vec![span("The source.")], vec![span("More.")]] },
                Note { label: "two".into(), paras: vec![] },
                Note { label: "e".into(), paras: vec![vec![]] },
            ],
        },
        "A claim[^1].\n\n[^1]: The source.\n\n    More.\n\n[^two]:\n\n[^e]: \\\n",
    );
}

#[test]
fn escapes_keep_every_character() {
    let tricky = "*not* [a](b) ~~x~~ _u_ \\ <b> &#32; `c`";
    check(paras(vec![Block::Para(vec![span(tricky)])]), "\\*not\\* \\[a\\](b) \\~\\~x\\~\\~ \\_u\\_ \\\\ \\<b> \\&#32; \\`c\\`\n");
}

#[test]
fn block_starts_are_escaped() {
    for (text, written) in [
        ("# not a heading", "\\# not a heading\n"),
        ("> not a quote", "\\> not a quote\n"),
        ("::: not a fence", "\\::: not a fence\n"),
        ("1. not a list", "1\\. not a list\n"),
        ("- dash", "\\- dash\n"),
        ("***", "\\*\\*\\*\n"),
        ("+++", "\\+++\n"),
    ] {
        check(paras(vec![Block::Para(vec![span(text)])]), written);
    }
}

#[test]
fn edges_empty_paragraphs_and_breaks() {
    check(paras(vec![Block::Para(vec![span("  indented\t")])]), "&#32;&#32;indented&#9;\n");
    check(paras(vec![Block::Para(vec![]), Block::Para(vec![span("x")])]), "\\\n\nx\n");
    check(paras(vec![Block::Para(vec![span("end\n")])]), "end\\\n\n");
    check(paras(vec![Block::Para(vec![span("a\n\nb")])]), "a\\\n\\\nb\n");
    check(paras(vec![Block::Quote(vec![vec![span("q\n")], vec![]])]), "> q\\\n> \n>\n> \\\n");
    check(paras(vec![Block::Para(vec![span("cr\rhere")])]), "cr&#13;here\n");
    check(paras(vec![Block::Para(vec![span("\n\nb")])]), "&#10;\\\nb\n");
}

#[test]
fn the_empty_text_is_empty() {
    check(Text::new(), "");
}

#[test]
fn hand_written_text_keeps_unmatched_markers() {
    let src = "An *open marker and a ] and [brackets] and **bold";
    let t = parse(src);
    let plain = wordsmith_manuscript_model::text::plain(match &t.blocks[0] {
        Block::Para(c) => c,
        _ => panic!("a paragraph"),
    });
    // Every character but the markers read as emphasis is kept, in order.
    assert_eq!(plain.replace('*', ""), src.replace('*', ""));
    assert!(plain.contains("] and [brackets]"));
    let t = parse("*a [b* c]\n");
    assert_eq!(t.blocks, vec![Block::Para(vec![marked("a [b", |m| m.italic = true), span(" c]")])]);
    let t = parse("**never closed\n");
    assert_eq!(t.blocks, vec![Block::Para(vec![span("**never closed")])]);
}

#[test]
fn crlf_files_read_as_lf() {
    assert_eq!(parse("One\r\n\r\n***\r\n\r\nTwo\r\n"), parse("One\n\n***\n\nTwo\n"));
}

#[test]
fn hostile_nesting_and_many_markers_do_not_hang() {
    let deep = format!("{}x{}", "[".repeat(10_000), "]{.smallcaps}".repeat(10_000));
    let t = parse(&deep);
    assert_eq!(t.blocks.len(), 1);
    let stars = "*a ".repeat(20_000);
    let t = parse(&stars);
    assert_eq!(super::super::dialect::parse(&write(&t)), t);
}

fn arb_text_piece() -> impl Strategy<Value = String> {
    "[a-z é—“”*_\\[\\]~\\\\<&#`:>+\\-1.!^{}()\n\r\t]{0,12}"
}

fn arb_marks() -> impl Strategy<Value = Marks> {
    (
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        any::<bool>(),
        proptest::option::of("[a-z]{1,3}"),
        proptest::option::of("[a-z: ()]{1,6}"),
        proptest::option::of("c[0-9]"),
    )
        .prop_map(|(italic, bold, small_caps, strike, lang, link, comment)| Marks { italic, bold, small_caps, strike, lang, link, comment })
}

fn arb_inlines() -> impl Strategy<Value = Vec<Inline>> {
    proptest::collection::vec(
        prop_oneof![
            8 => (arb_text_piece(), arb_marks()).prop_map(|(t, m)| Inline::Span { text: t, marks: m }),
            1 => ("[0-9a-z]{1,3}", arb_marks()).prop_map(|(l, m)| Inline::NoteRef { label: l, marks: m }),
        ],
        0..6,
    )
}

fn arb_block() -> impl Strategy<Value = Block> {
    prop_oneof![
        6 => arb_inlines().prop_map(Block::Para),
        1 => (1u8..4, arb_inlines()).prop_map(|(level, content)| Block::Heading { level, content }),
        1 => Just(Block::SceneBreak),
        1 => proptest::collection::vec(arb_inlines(), 0..3).prop_map(Block::Quote),
        1 => (any::<bool>(), proptest::option::of("[a-z ]{0,6}"), proptest::collection::vec(arb_inlines(), 0..3))
            .prop_map(|(v, k, paras)| Block::Fenced { kind: if v { Fence::Verse } else { Fence::Aside(k) }, paras }),
        1 => ("[a-z \\]\\[\n]{0,8}", "[a-z/.() ]{1,10}").prop_map(|(alt, src)| Block::Image { alt, src, role: ImageRole::Page }),
        1 => ("[a-z]{1,5}", "[a-z`\n\r ]{0,12}").prop_map(|(format, text)| Block::Raw { format, text }),
    ]
}

fn arb_text() -> impl Strategy<Value = Text> {
    (
        proptest::collection::vec(arb_block(), 0..6),
        proptest::collection::vec(
            ("[0-9a-z]{1,3}", proptest::collection::vec(arb_inlines(), 0..3)).prop_map(|(label, paras)| Note { label, paras }),
            0..3,
        ),
    )
        .prop_map(|(blocks, notes)| Text { blocks, notes })
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 2000, ..ProptestConfig::default() })]

    #[test]
    fn model_to_text_to_model_is_exact(t in arb_text()) {
        let mut t = t;
        canonical(&mut t);
        let out = write(&t);
        prop_assert_eq!(parse(&out), t);
    }

    #[test]
    fn written_text_reads_and_writes_back_byte_identical(t in arb_text()) {
        let out = write(&t);
        prop_assert_eq!(write(&parse(&out)), out);
    }

    #[test]
    fn any_input_parses_and_settles(s in "[a-z *_\\[\\]~\\\\<&#`:>!^{}()=.\n\r\t]{0,80}") {
        let t = parse(&s);
        let out = write(&t);
        prop_assert_eq!(write(&parse(&out)), out);
    }
}
