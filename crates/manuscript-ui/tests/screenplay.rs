// SPDX-License-Identifier: MIT OR Apache-2.0
//! Screenplays in a project (SCR-1): the script file is kept exactly as Wordsmith Script wrote
//! it, shown read-only with its counts, given a synopsis like any item, re-read when Script saves
//! it, and left out of manuscript exports.

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use jcj_shell::egui;
use serde_json::json;
use wordsmith_manuscript_model::{Area, ItemId, Kind, Project};
use wordsmith_manuscript_ui::media::{self, MediaView};
use wordsmith_manuscript_ui::session::Session;

/// A short script written for the test.
const SCRIPT: &str = "Wordsmith Script 5\n=== settings ===\n{\n  \"core\": {\n    \"title\": \"Harbour Lights\"\n  }\n}\n=== script ===\n@SceneHeading\nEXT. HARBOUR - NIGHT\n\n@Action\nRain *hammers* the pier.\n\n@Character\nMARA\n\n@Dialogue\nWe go now.\n\n";

#[test]
fn a_screenplay_is_kept_shown_and_left_out_of_exports() {
    let base = std::env::temp_dir().join(format!("wsm-screenplay-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(&base).unwrap();
    let file = base.join("Harbour Lights.wss");
    std::fs::write(&file, SCRIPT).unwrap();
    std::fs::write(base.join("not a script.wss"), "hello").unwrap();
    let mut s = Session::new(Project::new());
    let id: ItemId = s.run("media.import", &json!({ "path": file.to_str() })).unwrap()["id"].as_str().unwrap().parse().unwrap();
    assert!(s.run("media.import", &json!({ "path": base.join("not a script.wss").to_str() })).is_err());
    let item = s.project.binder.get(id).unwrap();
    assert_eq!((item.kind, s.project.binder.area_of(id)), (Kind::Screenplay, Some(Area::Manuscript)));
    let info = s.run("media.info", &json!({ "id": id.as_text() })).unwrap();
    assert_eq!((info["words"].as_u64(), info["scenes"].as_u64()), (Some(11), Some(1)));
    s.run("item.setSynopsis", &json!({ "id": id.as_text(), "synopsis": "The film of chapter one." })).unwrap();
    let root = base.join("Book.wsm");
    s.save_as(&root).unwrap();
    let kept = root.join(format!("scripts/{id}.wss"));
    assert_eq!(std::fs::read_to_string(&kept).unwrap(), SCRIPT, "kept byte for byte");
    let mut back = Session::open(&root).unwrap();
    assert_eq!(back.project.text(id).unwrap().meta.synopsis, "The film of chapter one.");
    // Wordsmith Script saves the file in place; Manuscript reads it again.
    std::fs::write(&kept, SCRIPT.replace("We go now.", "We go now, before the tide turns.")).unwrap();
    assert_eq!(back.run("media.info", &json!({ "id": id.as_text() })).unwrap()["words"], 15);
    // Not part of the manuscript's exports.
    let out = wordsmith_manuscript_io::export::export(&back.project, &Default::default(), wordsmith_manuscript_io::export::Target::Text).unwrap();
    assert!(!String::from_utf8_lossy(&out.bytes).contains("HARBOUR"));
    // The viewer shows it.
    struct St {
        s: Session,
        v: MediaView,
        id: ItemId,
        styled: bool,
    }
    let mut h = Harness::builder().with_size(egui::vec2(800.0, 600.0)).build_ui_state(
        |ui, st: &mut St| {
            if !st.styled {
                wordsmith_manuscript_ui::brand::apply(ui.ctx(), jcj_shell::theme::Mode::Light);
                st.styled = true;
            }
            let _ = media::view(ui, &mut st.s, &mut st.v, st.id);
        },
        St { s: back, v: MediaView::default(), id, styled: false },
    );
    h.run();
    assert!(h.query_by_label("Screenplay, 15 words, 1 scene").is_some());
    assert!(h.query_by_label("EXT. HARBOUR - NIGHT").is_some());
    let _ = std::fs::remove_dir_all(&base);
}
