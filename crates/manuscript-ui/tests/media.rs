// SPDX-License-Identifier: MIT OR Apache-2.0
//! Research media (BND-6): pictures and PDFs copied into the project and kept through save,
//! Save As and packing; web links kept as addresses; the viewer shows each.

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use jcj_shell::egui;
use serde_json::json;
use wordsmith_manuscript_model::{Area, Item, ItemId, ItemText, Kind, Place, Project, Role, Text};
use wordsmith_manuscript_ui::media::{self, MediaView};
use wordsmith_manuscript_ui::session::Session;

fn dir(name: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("wsm-media-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// A small picture made for the test.
fn png(path: &std::path::Path) {
    let img = image::RgbaImage::from_fn(40, 30, |x, y| image::Rgba([(x * 6) as u8, (y * 8) as u8, 120, 255]));
    img.save(path).unwrap();
}

/// A two-page PDF (title page and text) made for the test with the app's own standard manuscript export.
fn pdf(path: &std::path::Path) {
    let mut p = Project::new();
    let id = ItemId::from_counter(1);
    p.binder.insert(Place::end_of(Area::Manuscript), Item::with_id(id, Kind::Text, "Notes").role(Role::Scene)).unwrap();
    p.texts.insert(id, ItemText::with_body(Text::from_paragraphs(&["The lighthouse stood on the northern rock."])));
    let w = wordsmith_manuscript_io::standard::pdf(&p, &Default::default(), &Default::default()).unwrap();
    std::fs::write(path, w.bytes).unwrap();
}

#[test]
fn media_and_links_are_kept_through_saving() {
    let d = dir("keep");
    png(&d.join("harbour map.png"));
    pdf(&d.join("tide table.pdf"));
    std::fs::write(d.join("notes.txt"), "not media").unwrap();
    let mut s = Session::new(Project::new());
    let pic: ItemId = s.run("media.import", &json!({ "path": d.join("harbour map.png").to_str() })).unwrap()["id"].as_str().unwrap().parse().unwrap();
    let doc: ItemId = s.run("media.import", &json!({ "path": d.join("tide table.pdf").to_str() })).unwrap()["id"].as_str().unwrap().parse().unwrap();
    assert!(s.run("media.import", &json!({ "path": d.join("notes.txt").to_str() })).is_err());
    let link: ItemId = s.run("link.new", &json!({ "address": "https://example.org/tides" })).unwrap()["id"].as_str().unwrap().parse().unwrap();
    assert!(s.run("link.new", &json!({ "address": "file:///etc/passwd" })).is_err());
    assert_eq!(s.project.binder.get(link).unwrap().title, "example.org");
    assert_eq!(s.project.binder.get(pic).unwrap().title, "harbour map");
    assert_eq!(s.project.binder.area_of(pic), Some(Area::Research));
    let info = s.run("media.info", &json!({ "id": pic.as_text() })).unwrap();
    assert_eq!((info["width"].as_u64(), info["height"].as_u64()), (Some(40), Some(30)));
    assert_eq!(s.run("media.info", &json!({ "id": doc.as_text() })).unwrap()["pages"], 2, "a title page and the text");
    let original = s.media_bytes(pic).unwrap();
    // Saved: the files are in media/; reopened, they read back the same.
    let root = d.join("Project");
    s.save_as(&root).unwrap();
    assert!(s.media.is_empty());
    assert!(root.join(format!("media/{}.png", pic.as_text())).exists());
    let back = Session::open(&root).unwrap();
    assert_eq!(back.media_bytes(pic).unwrap(), original);
    assert_eq!(back.project.binder.get(link).unwrap().file.as_deref(), Some("https://example.org/tides"));
    // Save As elsewhere copies them; packing carries them.
    let mut back = back;
    let other = d.join("Copy");
    back.save_as(&other).unwrap();
    assert_eq!(back.media_bytes(doc).unwrap(), std::fs::read(d.join("tide table.pdf")).unwrap());
    let packed = d.join("Copy.wsmz");
    back.save_packed(&packed).unwrap();
    let unpacked = Session::open_packed(&packed, &d.join("Unpacked")).unwrap();
    assert_eq!(unpacked.media_bytes(pic).unwrap(), original);
    // Link address changes are checked.
    let mut s = unpacked;
    assert!(s.run("link.set", &json!({ "id": link.as_text(), "address": "not a link" })).is_err());
    s.run("link.set", &json!({ "id": link.as_text(), "address": "https://example.org/tides/2" })).unwrap();
    let _ = std::fs::remove_dir_all(&d);
}

#[test]
fn the_viewer_shows_pictures_pdfs_and_links() {
    let d = dir("view");
    png(&d.join("map.png"));
    pdf(&d.join("table.pdf"));
    let mut s = Session::new(Project::new());
    let ids: Vec<ItemId> = [
        s.run("media.import", &json!({ "path": d.join("map.png").to_str(), "title": "Harbour" })).unwrap(),
        s.run("media.import", &json!({ "path": d.join("table.pdf").to_str() })).unwrap(),
        s.run("link.new", &json!({ "address": "https://example.org/", "title": "Almanac" })).unwrap(),
    ]
    .iter()
    .map(|v| v["id"].as_str().unwrap().parse().unwrap())
    .collect();
    struct St {
        s: Session,
        v: MediaView,
        show: ItemId,
        styled: bool,
    }
    let mut h = Harness::builder().with_size(egui::vec2(700.0, 600.0)).build_ui_state(
        |ui, st: &mut St| {
            if !st.styled {
                wordsmith_manuscript_ui::brand::apply(ui.ctx(), jcj_shell::theme::Mode::Light);
                st.styled = true;
            }
            let _ = media::view(ui, &mut st.s, &mut st.v, st.show);
        },
        St { s, v: MediaView::default(), show: ids[0], styled: false },
    );
    h.run();
    assert!(h.query_by_label("40 × 30 pixels").is_some());
    h.state_mut().show = ids[1];
    h.run();
    assert!(h.query_by_label("PDF, 2 pages").is_some());
    assert!(h.query_by_label_contains("lighthouse").is_some(), "the PDF's text is shown");
    h.state_mut().show = ids[2];
    h.run();
    assert!(h.query_by_label("Open in Browser").is_some());
    let _ = std::fs::remove_dir_all(&d);
}
