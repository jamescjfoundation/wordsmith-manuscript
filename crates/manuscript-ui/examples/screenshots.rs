// SPDX-License-Identifier: MIT OR Apache-2.0
//! The README's pictures, drawn by the app itself offscreen (`jcj_shell::snapshot`):
//! `cargo xtask screenshots` runs this with the folder to write them to.
//!
//! The sample project is the first six chapters of *Pride and Prejudice* (public domain,
//! `tests/fixtures/novels/pride-and-prejudice.txt`), each chapter split into two scenes. Chapter
//! and scene titles, synopses and bible entries are written for these pictures.

use std::path::{Path, PathBuf};

use jcj_shell::egui::{self, Vec2};
use jcj_shell::workspace::Layout;
use jcj_shell::{App, Cmd, Shell};
use serde_json::{Value, json};
use wordsmith_manuscript_model::{Area, Item, ItemId, ItemText, Kind, Place, Project, Role, Text};
use wordsmith_manuscript_ui::session::Session;
use wordsmith_manuscript_ui::{BINDER_PANEL, INSPECTOR_PANEL, Manuscript, ViewMode};

const SIZE: Vec2 = Vec2::new(1280.0, 820.0);
const PPP: f32 = 1.5;

/// Chapter title, then each scene's title, synopsis, label, status, point of view and story date.
type SceneInfo = (&'static str, &'static str, &'static str, &'static str, &'static str, &'static str);
const CHAPTERS: [(&str, [SceneInfo; 2]); 6] = [
    (
        "Netherfield Is Let",
        [
            (
                "News of a tenant",
                "Mrs. Bennet brings word that a rich young man has taken Netherfield Park.",
                "Idea",
                "Final draft",
                "Mrs. Bennet",
                "1811-09-29",
            ),
            (
                "A husband's teasing",
                "Mr. Bennet will not promise to call on the newcomer, and enjoys his wife's despair.",
                "Backstory",
                "Final draft",
                "Mr. Bennet",
                "1811-09-29",
            ),
        ],
    ),
    (
        "The Secret Call",
        [
            (
                "Trimming a hat",
                "Mr. Bennet lets slip that he has called on Mr. Bingley after all.",
                "Idea",
                "Revised draft",
                "Elizabeth",
                "1811-10-07",
            ),
            (
                "Joy at Longbourn",
                "The family rejoices and guesses when Mr. Bingley will return the visit.",
                "Backstory",
                "Revised draft",
                "Mrs. Bennet",
                "1811-10-07",
            ),
        ],
    ),
    (
        "The Assembly",
        [
            (
                "Mr. Bingley's party",
                "The Netherfield party arrives at the ball; Mr. Bingley is liked by everyone.",
                "Romance",
                "Revised draft",
                "Elizabeth",
                "1811-10-15",
            ),
            (
                "Not handsome enough",
                "Mr. Darcy refuses to dance with Elizabeth, and she overhears why.",
                "Romance",
                "First draft",
                "Elizabeth",
                "1811-10-15",
            ),
        ],
    ),
    (
        "Sisters Confide",
        [
            ("Jane's admission", "Alone with Elizabeth, Jane owns how much she admires Mr. Bingley.", "Romance", "First draft", "Jane", "1811-10-16"),
            (
                "The Bingley sisters",
                "Elizabeth weighs Mr. Bingley's proud sisters and his friendship with Mr. Darcy.",
                "Backstory",
                "First draft",
                "Elizabeth",
                "1811-10-16",
            ),
        ],
    ),
    (
        "Lucas Lodge",
        [
            ("The morning after", "The Lucases call to talk over the ball, and Mr. Darcy's pride.", "Idea", "To do", "Charlotte", "1811-10-17"),
            ("Pride and vanity", "Mary lectures on the difference between pride and vanity.", "Idea", "To do", "Mary", "1811-10-17"),
        ],
    ),
    (
        "Fine Eyes",
        [
            (
                "Charlotte's advice",
                "Charlotte warns that Jane hides her feelings too well to win Mr. Bingley.",
                "Romance",
                "To do",
                "Charlotte",
                "1811-10-24",
            ),
            ("A pair of fine eyes", "Mr. Darcy begins to admire Elizabeth, and Miss Bingley notices.", "Romance", "To do", "Mr. Darcy", "1811-10-24"),
        ],
    ),
];

fn main() {
    let out = std::env::args().nth(1).map_or_else(|| PathBuf::from("docs/images"), PathBuf::from);
    if let Err(e) = run(&out) {
        eprintln!("screenshots: {e}");
        std::process::exit(1);
    }
}

fn id(n: u64) -> ItemId {
    ItemId::from_counter(n)
}

/// The first six chapters' paragraphs: lines joined, illustrations left out, `_italic_` marks
/// removed.
fn chapters(text: &str) -> Vec<Vec<String>> {
    let mut out: Vec<Vec<String>> = Vec::new();
    let mut in_illustration = false;
    for block in text.split("\n\n") {
        let para = block.lines().map(str::trim).filter(|l| !l.is_empty()).collect::<Vec<_>>().join(" ");
        if para.is_empty() {
            continue;
        }
        let heading = para.starts_with("CHAPTER ") || para.starts_with("Chapter I.]");
        if heading {
            in_illustration = false;
            if out.len() == CHAPTERS.len() {
                break;
            }
            out.push(Vec::new());
            continue;
        }
        if in_illustration || para.starts_with('[') {
            in_illustration = !para.contains(']');
            continue;
        }
        if let Some(ch) = out.last_mut() {
            ch.push(para.replace('_', ""));
        }
    }
    out
}

fn sample() -> Result<Manuscript, String> {
    let novel = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/novels/pride-and-prejudice.txt");
    let text = std::fs::read_to_string(&novel).map_err(|e| format!("{}: {e}", novel.display()))?;
    let paras = chapters(&text);
    if paras.len() != CHAPTERS.len() {
        return Err(format!("found {} chapters, not {}", paras.len(), CHAPTERS.len()));
    }
    let mut p = Project::new();
    p.settings.book.title = "Pride and Prejudice".into();
    p.settings.book.author = "Jane Austen".into();
    p.settings.book.surname = "Austen".into();
    let mut n = 1000;
    let mut scenes = Vec::new();
    for ((title, info), body) in CHAPTERS.iter().zip(&paras) {
        n += 1;
        let half = body.len() / 2;
        let mut children = Vec::new();
        for (k, part) in [&body[..half], &body[half..]].into_iter().enumerate() {
            n += 1;
            let sid = id(n);
            children.push(Item::with_id(sid, Kind::Text, info[k].0).role(Role::Scene));
            p.texts.insert(sid, ItemText::with_body(Text::from_paragraphs(part)));
            scenes.push((sid, info[k]));
        }
        p.binder
            .insert(Place::end_of(Area::Manuscript), Item::with_id(id(n - 2), Kind::Folder, *title).role(Role::Chapter).with_children(children))
            .map_err(|e| e.to_string())?;
    }
    let mut s = Session::new(p);
    let run = |s: &mut Session, cmd: &str, v: Value| -> Result<Value, String> { s.run(cmd, &v).map_err(|e| format!("{cmd}: {e}")) };
    run(&mut s, "project.setFields", json!({ "fields": [{ "name": "Point of view" }, { "name": "Scene date", "kind": "date" }] }))?;
    for (sid, (_, synopsis, label, status, pov, date)) in &scenes {
        let i = sid.as_text();
        run(&mut s, "item.setSynopsis", json!({ "id": i, "synopsis": synopsis }))?;
        run(&mut s, "item.setLabel", json!({ "id": i, "label": label }))?;
        run(&mut s, "item.setStatus", json!({ "id": i, "status": status }))?;
        run(&mut s, "item.setField", json!({ "id": i, "field": "Point of view", "value": pov }))?;
        run(&mut s, "item.setField", json!({ "id": i, "field": "Scene date", "value": date }))?;
        run(&mut s, "item.setTarget", json!({ "id": i, "words": 1200 }))?;
    }
    let entry = |s: &mut Session, kind: &str, name: &str| -> Result<String, String> {
        let got = s.run("bible.new", &json!({ "kind": kind, "name": name })).map_err(|e| format!("bible.new: {e}"))?;
        got["id"].as_str().map(str::to_owned).ok_or_else(|| "bible.new gave no id".to_owned())
    };
    let lizzy = entry(&mut s, "character", "Elizabeth Bennet")?;
    let darcy = entry(&mut s, "character", "Mr. Darcy")?;
    let jane = entry(&mut s, "character", "Jane Bennet")?;
    let bingley = entry(&mut s, "character", "Mr. Bingley")?;
    entry(&mut s, "place", "Longbourn")?;
    let netherfield = entry(&mut s, "place", "Netherfield Park")?;
    let first = entry(&mut s, "thread", "First impressions")?;
    let match_ = entry(&mut s, "thread", "Jane and Mr. Bingley")?;
    run(&mut s, "entry.setField", json!({ "id": lizzy, "field": "Role in story", "value": "Heroine" }))?;
    run(&mut s, "entry.setField", json!({ "id": lizzy, "field": "Age", "value": "Twenty" }))?;
    run(&mut s, "entry.setField", json!({ "id": lizzy, "field": "Personality", "value": "Quick, playful, and too sure of her first judgments." }))?;
    run(&mut s, "entry.setField", json!({ "id": lizzy, "field": "Wants", "value": "To marry only for real affection." }))?;
    run(&mut s, "entry.setField", json!({ "id": lizzy, "field": "Flaw", "value": "Prejudice: she decides about people at first sight." }))?;
    run(
        &mut s,
        "entry.setBody",
        json!({ "id": lizzy, "text": "The second of the five Bennet daughters and her father's favourite. She laughs at follies, including her own, but is slow to see when she has been wrong." }),
    )?;
    run(&mut s, "entry.relate", json!({ "id": lizzy, "relation": "sibling of", "to": jane })).ok();
    for (k, (sid, _)) in scenes.iter().enumerate() {
        let i = sid.as_text();
        if k >= 4 {
            run(&mut s, "item.linkEntry", json!({ "id": i, "entry": lizzy }))?;
        }
        if matches!(k, 5 | 7 | 9 | 11) {
            run(&mut s, "item.linkEntry", json!({ "id": i, "entry": darcy }))?;
            run(&mut s, "item.linkEntry", json!({ "id": i, "entry": first }))?;
        }
        if matches!(k, 4 | 6 | 10) {
            run(&mut s, "item.linkEntry", json!({ "id": i, "entry": jane }))?;
            run(&mut s, "item.linkEntry", json!({ "id": i, "entry": bingley }))?;
            run(&mut s, "item.linkEntry", json!({ "id": i, "entry": match_ }))?;
        }
        if k < 2 {
            run(&mut s, "item.linkEntry", json!({ "id": i, "entry": netherfield }))?;
        }
    }
    let chapter_synopses = [
        "A rich young man takes Netherfield Park, and Mrs. Bennet wants him for a daughter.",
        "Mr. Bennet has called on Mr. Bingley in secret, and the family is delighted.",
        "At the assembly Mr. Bingley charms everyone; Mr. Darcy offends Elizabeth.",
        "Jane and Elizabeth talk over the ball and the Netherfield party.",
        "The neighbours dissect the ball and Mr. Darcy's pride.",
        "Jane's reserve worries Charlotte; Mr. Darcy finds Elizabeth's eyes fine.",
    ];
    for (k, synopsis) in chapter_synopses.iter().enumerate() {
        let i = id(1001 + 3 * k as u64).as_text();
        run(&mut s, "item.setSynopsis", json!({ "id": i, "synopsis": synopsis }))?;
        run(&mut s, "item.setLabel", json!({ "id": i, "label": CHAPTERS[k].1[1].2 }))?;
        run(&mut s, "item.setStatus", json!({ "id": i, "status": CHAPTERS[k].1[1].3 }))?;
    }
    run(&mut s, "item.addKeyword", json!({ "id": scenes[5].0.as_text(), "keyword": "ball" }))?;
    run(&mut s, "item.addKeyword", json!({ "id": scenes[4].0.as_text(), "keyword": "ball" }))?;
    run(
        &mut s,
        "targets.set",
        json!({ "manuscript": 120_000, "deadline": "2027-06-30", "writing_days": ["mon", "tue", "wed", "thu", "fri"], "session": 1000 }),
    )?;
    let mut app = Manuscript::with_session(s);
    app.binder.selected = Some(scenes[5].0);
    app.open_item(scenes[5].0);
    Ok(app)
}

fn panels(app: &Manuscript, shell: &mut Shell, open: &[&str]) {
    shell.layout = Layout::with_open(&app.panels(), open);
}

fn view(app: &mut Manuscript, shell: &mut Shell, cmd: &str) -> Result<(), String> {
    app.run(&Cmd::new(cmd), shell).map_err(|e| format!("{cmd}: {e}"))
}

fn shot(shell: &mut Shell, app: &mut Manuscript, out: &Path, name: &str) -> Result<(), String> {
    let ctx = egui::Context::default();
    wordsmith_manuscript_ui::brand::BRAND.apply(&ctx, shell.mode);
    let pic = jcj_shell::snapshot::render(&ctx, SIZE, PPP, Vec::new(), 6, |ui| shell.ui(ui, app));
    let path = out.join(format!("{name}.png"));
    std::fs::write(&path, pic.to_png()).map_err(|e| format!("{}: {e}", path.display()))?;
    println!("{}", path.display());
    Ok(())
}

fn run(out: &Path) -> Result<(), String> {
    std::fs::create_dir_all(out).map_err(|e| format!("{}: {e}", out.display()))?;
    let mut app = sample()?;
    let mut shell = Shell::new(wordsmith_manuscript_ui::NAME, env!("CARGO_PKG_VERSION"), wordsmith_manuscript_ui::brand::BRAND.tokens());
    let (a, s) = (&mut app, &mut shell);
    panels(a, s, &[BINDER_PANEL, INSPECTOR_PANEL]);
    s.chrome.tab = "write".into();
    shot(s, a, out, "editor")?;
    s.chrome.tab = "view".into();
    a.view_root = None;
    view(a, s, "view.corkboard")?;
    shot(s, a, out, "corkboard")?;
    view(a, s, "view.outline")?;
    panels(a, s, &[BINDER_PANEL]);
    shot(s, a, out, "outline")?;
    view(a, s, "view.pages")?;
    shot(s, a, out, "pages")?;
    // The bible: Elizabeth's sheet.
    let lizzy = a
        .session
        .as_ref()
        .and_then(|s| s.project.entries.iter().find(|(_, e)| e.name == "Elizabeth Bennet").map(|(k, _)| *k))
        .ok_or("no entry for Elizabeth")?;
    a.mode = ViewMode::Editor;
    a.binder.selected = Some(lizzy);
    a.open_item(lizzy);
    s.chrome.tab = "bible".into();
    panels(a, s, &[BINDER_PANEL]);
    shot(s, a, out, "bible")?;
    // Dark, with the whole manuscript as one text.
    s.chrome.tab = "write".into();
    s.mode = jcj_shell::theme::Mode::Dark;
    view(a, s, "view.stitchManuscript")?;
    panels(a, s, &[BINDER_PANEL]);
    shot(s, a, out, "stitched-dark")?;
    Ok(())
}
