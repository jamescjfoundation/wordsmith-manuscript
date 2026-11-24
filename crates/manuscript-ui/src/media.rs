// SPDX-License-Identifier: MIT OR Apache-2.0
//! Research media (BND-6): images, PDFs and web links kept in the Research area and shown in the
//! editor area. Image and PDF files are copied into the project (`media/<id>.<ext>`); a link
//! keeps its address. Nothing is fetched from the web: a link opens in the browser when asked.

use std::collections::HashMap;
use std::path::Path;

use jcj_shell::egui::{self, RichText, Ui};
use serde_json::{Value, json};
use wordsmith_manuscript_model::{Area, Item, ItemId, Kind, Place};

use crate::commands::{Spec, id_param, opt_str, str_param};
use crate::session::{Outcome, Session};

/// The largest media file kept in a project.
pub const MAX_BYTES: u64 = 64 << 20;
/// The image and document types a project keeps.
pub const TYPES: [&str; 7] = ["png", "jpg", "jpeg", "gif", "webp", "pdf", "wss"];
/// The largest picture shown, in pixels on a side.
const MAX_SIDE: u32 = 12_000;

/// The media commands.
pub fn specs() -> Vec<Spec> {
    let spec = |id, label, params, about, mutates| Spec { id, label, key: "", params, about, mutates };
    vec![
        spec(
            "media.import",
            "Add Image or PDF",
            r#"{"path": string, "title"?: string, "inside"?: id}"#,
            "Copy an image (PNG, JPEG, GIF, WebP) or a PDF into the project's Research (or inside a folder there).",
            true,
        ),
        spec(
            "link.new",
            "Add Web Link",
            r#"{"address": string, "title"?: string, "inside"?: id}"#,
            "Keep a web address in Research (nothing is fetched).",
            true,
        ),
        spec("link.set", "Change Web Link", r#"{"id": id, "address": string}"#, "Change a link's address.", true),
        spec("media.info", "Media Info", r#"{"id": id}"#, "What a media or link item is: kind, size, picture size or pages, address.", false),
    ]
}

fn place(s: &Session, v: &Value) -> Result<Place, String> {
    match opt_str(v, "inside") {
        None => Ok(Place::end_of(Area::Research)),
        Some(_) => {
            let parent = id_param(v, "inside")?;
            let item = s.project.binder.get(parent).ok_or_else(|| format!("no binder item {parent}"))?;
            if !item.kind.can_hold_children() {
                return Err(format!("“{}” cannot hold other items", item.title));
            }
            let area = s.project.binder.area_of(parent).ok_or("the folder is not in the binder")?;
            Ok(Place::inside(area, parent))
        }
    }
}

/// Whether an address is a web address this app opens.
pub fn is_web(address: &str) -> bool {
    let a = address.trim();
    (a.starts_with("https://") || a.starts_with("http://")) && a.len() > 8 && !a.contains(char::is_whitespace)
}

fn host(address: &str) -> String {
    address.split("://").nth(1).and_then(|r| r.split('/').next()).unwrap_or(address).to_owned()
}

/// Decode a picture, bounded in size.
pub fn decode(bytes: &[u8]) -> Result<egui::ColorImage, String> {
    let mut reader = image::ImageReader::new(std::io::Cursor::new(bytes)).with_guessed_format().map_err(|e| e.to_string())?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_SIDE);
    limits.max_image_height = Some(MAX_SIDE);
    limits.max_alloc = Some(1 << 30);
    reader.limits(limits);
    let img = reader.decode().map_err(|e| format!("the picture could not be read: {e}"))?.to_rgba8();
    let size = [img.width() as usize, img.height() as usize];
    Ok(egui::ColorImage::from_rgba_unmultiplied(size, img.as_raw()))
}

/// Run a media command, or `None` if `id` is not one.
pub fn run(s: &mut Session, id: &str, v: &Value) -> Option<Outcome> {
    Some(match id {
        "media.import" => (|| {
            let path = Path::new(str_param(v, "path")?);
            let ext = path.extension().and_then(|e| e.to_str()).map(str::to_lowercase).unwrap_or_default();
            if !TYPES.contains(&ext.as_str()) {
                return Err(format!("{}: only pictures (PNG, JPEG, GIF, WebP) and PDFs can be added", path.display()));
            }
            let len = std::fs::metadata(path).map_err(|e| format!("{}: {e}", path.display()))?.len();
            if len > MAX_BYTES {
                return Err(format!("{} is larger than {} MB", path.display(), MAX_BYTES >> 20));
            }
            let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
            if ext == "wss" {
                wordsmith_manuscript_io::screenplay::read(&bytes).map_err(|e| format!("{}: {e}", path.display()))?;
            } else if ext == "pdf" {
                if !bytes.starts_with(b"%PDF") {
                    return Err(format!("{} is not a PDF", path.display()));
                }
            } else {
                decode(&bytes)?;
            }
            let title = opt_str(v, "title")
                .map(str::trim)
                .filter(|t| !t.is_empty())
                .map(str::to_owned)
                .unwrap_or_else(|| path.file_stem().and_then(|n| n.to_str()).unwrap_or("Picture").to_owned());
            // A screenplay goes into the Manuscript (or where asked) as a screenplay item, its
            // file kept as Wordsmith Script wrote it (SCR-1).
            let at = if ext == "wss" && opt_str(v, "inside").is_none() { Place::end_of(Area::Manuscript) } else { place(s, v)? };
            let mut item = if ext == "wss" { Item::new(Kind::Screenplay, title) } else { Item::new(Kind::Media, title) };
            item.file = if ext == "wss" { None } else { Some(if ext == "jpeg" { "jpg".into() } else { ext }) };
            let new = item.id;
            s.change("Add Media", &[], &[], |p| p.binder.insert(at, item).map(|_| ()).map_err(|e| e.to_string()))?;
            s.media.insert(new, bytes);
            Ok(json!({ "id": new.as_text() }))
        })(),
        "link.new" => (|| {
            let address = str_param(v, "address")?.trim().to_owned();
            if !is_web(&address) {
                return Err(format!("`{address}` is not a web address (https://...)"));
            }
            let title = opt_str(v, "title").map(str::trim).filter(|t| !t.is_empty()).map_or_else(|| host(&address), str::to_owned);
            let at = place(s, v)?;
            let mut item = Item::new(Kind::Link, title);
            item.file = Some(address);
            let new = item.id;
            s.change("Add Web Link", &[], &[], |p| p.binder.insert(at, item).map(|_| ()).map_err(|e| e.to_string()))?;
            Ok(json!({ "id": new.as_text() }))
        })(),
        "link.set" => (|| {
            let item = id_param(v, "id")?;
            let address = str_param(v, "address")?.trim().to_owned();
            if !is_web(&address) {
                return Err(format!("`{address}` is not a web address (https://...)"));
            }
            if s.project.binder.get(item).is_none_or(|i| i.kind != Kind::Link) {
                return Err("this item is not a web link".into());
            }
            s.change("Change Web Link", &[], &[], |p| {
                if let Some(i) = p.binder.get_mut(item) {
                    i.file = Some(address);
                }
                Ok(())
            })?;
            Ok(json!({}))
        })(),
        "media.info" => (|| {
            let item = id_param(v, "id")?;
            let it = s.project.binder.get(item).ok_or_else(|| format!("no binder item {item}"))?.clone();
            match it.kind {
                Kind::Link => Ok(json!({ "kind": "link", "address": it.file })),
                Kind::Media => {
                    let bytes = s.media_bytes(item)?;
                    if it.file.as_deref() == Some("pdf") {
                        let pages = wordsmith_manuscript_io::standard::pdf_pages(&bytes).map(|p| p.len()).unwrap_or(0);
                        Ok(json!({ "kind": "pdf", "bytes": bytes.len(), "pages": pages }))
                    } else {
                        let img = decode(&bytes)?;
                        Ok(json!({ "kind": "image", "bytes": bytes.len(), "width": img.size[0], "height": img.size[1] }))
                    }
                }
                Kind::Screenplay => {
                    let sp = wordsmith_manuscript_io::screenplay::read(&s.media_bytes(item)?)?;
                    Ok(
                        json!({ "kind": "screenplay", "title": sp.title, "words": sp.words(), "scenes": sp.scenes(), "paragraphs": sp.paragraphs.len() }),
                    )
                }
                _ => Err("this item is not a media file or a link".into()),
            }
        })(),
        _ => return None,
    })
}

/// What the viewer has loaded, by item.
#[derive(Default)]
pub struct MediaView {
    pictures: HashMap<ItemId, Result<egui::TextureHandle, String>>,
    pdfs: HashMap<ItemId, Result<Vec<Vec<String>>, String>>,
    scripts: HashMap<ItemId, (Option<std::time::SystemTime>, Result<wordsmith_manuscript_io::screenplay::Screenplay, String>)>,
    address: String,
    editing: Option<ItemId>,
}

impl MediaView {
    /// Forget what was loaded (after the project changes).
    pub fn clear(&mut self) {
        self.pictures.clear();
        self.pdfs.clear();
        self.scripts.clear();
    }
}

/// Draw a media or link item in the editor area. Returns an error for the status bar.
pub fn view(ui: &mut Ui, s: &mut Session, mv: &mut MediaView, id: ItemId) -> Option<String> {
    let item = s.project.binder.get(id)?.clone();
    let mut error = None;
    ui.add_space(8.0);
    ui.heading(&item.title);
    match item.kind {
        Kind::Link => {
            let address = item.file.clone().unwrap_or_default();
            if mv.editing != Some(id) {
                mv.address.clone_from(&address);
                mv.editing = Some(id);
            }
            ui.horizontal(|ui| {
                let l = ui.label(crate::i18n::tr("Address"));
                let r = ui.add(egui::TextEdit::singleline(&mut mv.address).desired_width(ui.available_width() - 140.0)).labelled_by(l.id);
                if r.lost_focus() && mv.address.trim() != address {
                    match s.run("link.set", &json!({ "id": id.as_text(), "address": mv.address })) {
                        Ok(_) => {}
                        Err(e) => {
                            mv.address.clone_from(&address);
                            error = Some(e);
                        }
                    }
                }
                if ui.button("Open in Browser").clicked() && is_web(&address) {
                    ui.ctx().open_url(egui::OpenUrl::new_tab(&address));
                }
            });
            ui.label(RichText::new("The page is not fetched or kept; this link only remembers where it is.").weak());
        }
        Kind::Screenplay => {
            // Read again whenever Wordsmith Script has saved the file.
            let file = s.root().zip(s.media_path(id)).map(|(r, p)| r.join(p));
            let modified = file.as_ref().and_then(|f| std::fs::metadata(f).ok()).and_then(|m| m.modified().ok());
            let fresh = mv.scripts.get(&id).is_none_or(|(m, _)| *m != modified);
            if fresh {
                let read = s.media_bytes(id).and_then(|b| wordsmith_manuscript_io::screenplay::read(&b));
                mv.scripts.insert(id, (modified, read));
            }
            match mv.scripts.get(&id).map(|(_, r)| r) {
                Some(Ok(sp)) => {
                    ui.horizontal(|ui| {
                        ui.label(format!(
                            "Screenplay, {} words, {} scene{}",
                            crate::binder::format_words(sp.words()),
                            sp.scenes(),
                            if sp.scenes() == 1 { "" } else { "s" }
                        ));
                        open_file_button(ui, s, id);
                    });
                    ui.label(RichText::new("Read-only here: Wordsmith Script edits it, and the changes show here when it saves.").weak());
                    ui.separator();
                    egui::ScrollArea::vertical().id_salt(("manuscript-script", id.as_text())).auto_shrink([false, false]).show(ui, |ui| {
                        let w = ui.available_width().min(620.0);
                        for p in &sp.paragraphs {
                            let (indent, text) = match p.element.as_str() {
                                "SceneHeading" => (0.0, RichText::new(p.text.to_uppercase()).strong()),
                                "Character" => (w * 0.35, RichText::new(p.text.to_uppercase())),
                                "Dialogue" => (w * 0.2, RichText::new(&p.text)),
                                "Parenthetical" => (w * 0.28, RichText::new(&p.text).italics()),
                                "Transition" => (w * 0.7, RichText::new(p.text.to_uppercase())),
                                _ => (0.0, RichText::new(&p.text)),
                            };
                            ui.horizontal_wrapped(|ui| {
                                ui.add_space(indent);
                                ui.set_max_width(w);
                                ui.label(text);
                            });
                            if !matches!(p.element.as_str(), "Character" | "Parenthetical") {
                                ui.add_space(6.0);
                            }
                        }
                    });
                }
                Some(Err(e)) => {
                    ui.label(format!("The screenplay could not be read: {e}"));
                }
                None => {}
            }
        }
        Kind::Media if item.file.as_deref() == Some("pdf") => {
            let pages = mv.pdfs.entry(id).or_insert_with(|| s.media_bytes(id).and_then(|b| wordsmith_manuscript_io::standard::pdf_pages(&b)));
            match pages {
                Ok(pages) => {
                    ui.horizontal(|ui| {
                        ui.label(format!("PDF, {} page{}", pages.len(), if pages.len() == 1 { "" } else { "s" }));
                        open_file_button(ui, s, id);
                    });
                    ui.separator();
                    egui::ScrollArea::vertical().id_salt(("manuscript-pdf", id.as_text())).auto_shrink([false, false]).show(ui, |ui| {
                        for (n, lines) in pages.iter().enumerate() {
                            ui.label(RichText::new(format!("Page {}", n + 1)).strong());
                            if lines.iter().all(|l| l.trim().is_empty()) {
                                ui.label(RichText::new("(no text on this page)").weak());
                            }
                            for l in lines {
                                ui.label(l);
                            }
                            ui.add_space(8.0);
                        }
                    });
                }
                Err(e) => {
                    ui.label(format!("The PDF's text could not be read: {e}"));
                    open_file_button(ui, s, id);
                }
            }
        }
        Kind::Media => {
            let tex = mv.pictures.entry(id).or_insert_with(|| {
                s.media_bytes(id)
                    .and_then(|b| decode(&b))
                    .map(|img| ui.ctx().load_texture(format!("manuscript-media-{}", id.as_text()), img, egui::TextureOptions::LINEAR))
            });
            match tex {
                Ok(t) => {
                    let size = t.size_vec2();
                    ui.label(RichText::new(format!("{} × {} pixels", size.x, size.y)).weak());
                    egui::ScrollArea::both().id_salt(("manuscript-picture", id.as_text())).auto_shrink([false, false]).show(ui, |ui| {
                        let scale = (ui.available_width() / size.x).min(1.0);
                        let r = ui.add(egui::Image::new((t.id(), size * scale)));
                        let title = item.title.clone();
                        r.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Image, true, &title));
                    });
                }
                Err(e) => {
                    ui.label(format!("The picture could not be shown: {e}"));
                }
            }
        }
        _ => return None,
    }
    error
}

fn open_file_button(ui: &mut Ui, s: &Session, id: ItemId) {
    let file = s.root().zip(s.media_path(id)).map(|(r, p)| r.join(p));
    let saved = file.as_ref().is_some_and(|f| f.exists());
    if ui.add_enabled(saved, egui::Button::new("Open in Another App")).on_disabled_hover_text("Save the project first").clicked()
        && let Some(f) = file
    {
        ui.ctx().open_url(egui::OpenUrl::new_tab(format!("file://{}", f.display())));
    }
}

/// Whether an item is shown by the media viewer.
pub fn shows(item: &Item) -> bool {
    matches!(item.kind, Kind::Media | Kind::Link | Kind::Screenplay)
}
