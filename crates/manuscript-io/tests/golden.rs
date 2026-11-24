// SPDX-License-Identifier: MIT OR Apache-2.0
//! Golden files (plan section 4, rule 7): the standard manuscript's PDF text and page breaks, and
//! its DOCX structure, compared with reviewed files in `tests/golden/`. A change that alters them
//! is deliberate: rerun with `UPDATE_GOLDEN=1`, review the difference, and record it in
//! `docs/golden.md`.

use std::path::{Path, PathBuf};

use wordsmith_manuscript_io::compile::Options;
use wordsmith_manuscript_io::standard::{self, Standard};

#[path = "standard.rs"]
#[allow(dead_code)]
mod fixture;

fn golden(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden").join(name)
}

fn compare(name: &str, got: &str) {
    let path = golden(name);
    if std::env::var_os("UPDATE_GOLDEN").is_some() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, got).unwrap();
        return;
    }
    let want = std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("{} is missing: run with UPDATE_GOLDEN=1 and review it", path.display()));
    if want != got {
        let line = want.lines().zip(got.lines()).position(|(a, b)| a != b).unwrap_or(want.lines().count().min(got.lines().count()));
        panic!("{name} differs from the reviewed file at line {}: expected {:?}, got {:?}", line + 1, want.lines().nth(line), got.lines().nth(line));
    }
}

fn settings() -> Standard {
    Standard { created: "2000-01-01T00:00:00".into(), ..Standard::default() }
}

#[test]
fn standard_manuscript_pdf_text_and_page_breaks() {
    let p = fixture::sample();
    let out = standard::pdf(&p, &Options { end_mark: true, ..Options::default() }, &settings()).unwrap();
    assert!(out.check.passed(), "{}", out.check);
    let mut text = String::new();
    for (n, page) in standard::pdf_pages(&out.bytes).unwrap().iter().enumerate() {
        text.push_str(&format!("=== page {}\n", n + 1));
        for l in page {
            text.push_str(l.trim_end());
            text.push('\n');
        }
    }
    compare("standard-manuscript.pdf.txt", &text);
}

#[test]
fn standard_manuscript_docx_structure() {
    let p = fixture::sample();
    let out = standard::docx(&p, &Options { end_mark: true, ..Options::default() }, &settings()).unwrap();
    assert!(out.check.passed(), "{}", out.check);
    let doc = jcj_interchange::import(jcj_interchange::Format::Docx, &out.bytes).unwrap();
    let mut text = format!(
        "page {}x{} margins {} {} {} {}\n",
        doc.last_section.page_w,
        doc.last_section.page_h,
        doc.last_section.margin_top,
        doc.last_section.margin_right,
        doc.last_section.margin_bottom,
        doc.last_section.margin_left
    );
    for b in &doc.body {
        if let Some(p) = b.as_para() {
            let pp = &p.props;
            let words: Vec<&str> = p.text.split_whitespace().take(6).collect();
            text.push_str(&format!(
                "{:?} align={:?} indent={:?} before={:?} spacing={:?} break={:?} section={} | {}\n",
                pp.style,
                pp.align,
                pp.indent_first,
                pp.space_before,
                pp.line_spacing,
                pp.page_break_before,
                p.section.is_some(),
                words.join(" ")
            ));
        }
    }
    compare("standard-manuscript.docx.txt", &text);
}
