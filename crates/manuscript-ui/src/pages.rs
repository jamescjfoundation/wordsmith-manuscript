// SPDX-License-Identifier: MIT OR Apache-2.0
//! The page view (ED-8): the text as standard manuscript pages, with the page count. It shows
//! the selected item with what is under it, or the whole manuscript (with its title page), laid
//! out exactly as the standard manuscript PDF export lays it out. The layout is made in the
//! background when the text changes; only the pages in view are drawn.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use jcj_shell::egui::{self, RichText, Ui};
use serde_json::json;
use wordsmith_manuscript_io::compile::Options;
use wordsmith_manuscript_io::standard::{self, Paper, Standard};
use wordsmith_manuscript_model::ItemId;

use crate::session::Session;

/// What a layout was made for: the project revision, the item (or the whole manuscript) and
/// the settings.
#[derive(Clone, Debug, PartialEq)]
struct Key {
    revision: u64,
    item: Option<ItemId>,
    standard: String,
}

struct Laid {
    key: Key,
    pag: jcj_layout::paginate::Paginated,
    pages: usize,
    paper: Paper,
}

/// The page view's state.
#[derive(Default)]
pub struct PageView {
    /// Show the whole manuscript (else the selected item and what is under it).
    pub whole: bool,
    laid: Arc<Mutex<Option<Laid>>>,
    building: Arc<Mutex<Option<Key>>>,
    textures: HashMap<(usize, u32), egui::TextureHandle>,
    shown_key: Option<Key>,
}

impl PageView {
    /// The page count of the layout shown, once it is made.
    pub fn pages(&self) -> Option<usize> {
        self.laid.lock().ok().and_then(|l| l.as_ref().map(|l| l.pages))
    }

    /// Make the layout now (tests, agents): waits for it.
    pub fn build_now(&mut self, s: &Session, selected: Option<ItemId>) {
        let (key, p, o, st) = self.wanted(s, selected);
        let (_, pag) = standard::paginate(&p, &o, &st);
        let pages = standard::page_count(&pag);
        if let Ok(mut l) = self.laid.lock() {
            *l = Some(Laid { key, pag, pages, paper: st.paper });
        }
    }

    fn wanted(&self, s: &Session, selected: Option<ItemId>) -> (Key, wordsmith_manuscript_model::Project, Options, Standard) {
        let item = if self.whole { None } else { selected };
        let mut st = crate::exports::standard_of(s, &json!({})).unwrap_or_default();
        let mut o = Options::default();
        if let Some(i) = item {
            o.items = vec![i];
            st.title_page = false;
        }
        let key = Key { revision: s.revision(), item, standard: format!("{st:?}") };
        (key, s.project.clone(), o, st)
    }

    fn refresh(&mut self, s: &Session, selected: Option<ItemId>) {
        let (key, p, o, st) = self.wanted(s, selected);
        let have = self.laid.lock().ok().and_then(|l| l.as_ref().map(|l| l.key.clone()));
        let busy = self.building.lock().ok().and_then(|b| b.clone());
        if have.as_ref() == Some(&key) || busy.as_ref() == Some(&key) {
            return;
        }
        if let Ok(mut b) = self.building.lock() {
            *b = Some(key.clone());
        }
        let slot = self.laid.clone();
        let building = self.building.clone();
        std::thread::spawn(move || {
            let (_, pag) = standard::paginate(&p, &o, &st);
            let pages = standard::page_count(&pag);
            if let Ok(mut l) = slot.lock() {
                *l = Some(Laid { key: key.clone(), pag, pages, paper: st.paper });
            }
            if let Ok(mut b) = building.lock()
                && b.as_ref() == Some(&key)
            {
                *b = None;
            }
        });
    }
}

/// The page commands.
pub fn specs() -> Vec<crate::commands::Spec> {
    vec![crate::commands::Spec {
        id: "pages.count",
        label: "Count Pages",
        key: "",
        params: r#"{"id"?: id}"#,
        about: "How many standard manuscript pages an item (with what is under it) or the whole manuscript (with its title page) makes.",
        mutates: false,
    }]
}

/// Run a page command, or `None` if `id` is not one.
pub fn run(s: &mut Session, id: &str, v: &serde_json::Value) -> Option<crate::session::Outcome> {
    if id != "pages.count" {
        return None;
    }
    Some((|| {
        let item = match v.get("id") {
            Some(_) => Some(crate::commands::id_param(v, "id")?),
            None => None,
        };
        let mut pv = PageView { whole: item.is_none(), ..PageView::default() };
        pv.build_now(s, item);
        Ok(json!({ "pages": pv.pages().unwrap_or(0) }))
    })())
}

/// Draw the page view.
pub fn view(ui: &mut Ui, s: &Session, v: &mut PageView, selected: Option<ItemId>) {
    v.refresh(s, selected);
    let busy = v.building.lock().ok().is_some_and(|b| b.is_some());
    if busy {
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(100));
    }
    let guard = v.laid.lock();
    let Ok(guard) = guard else { return };
    ui.horizontal(|ui| {
        ui.heading(jcj_shell::i18n::tr("Pages"));
        let title = if v.whole {
            "the whole manuscript".to_owned()
        } else {
            selected.and_then(|i| s.project.binder.get(i)).map_or_else(|| "the whole manuscript".to_owned(), |i| format!("“{}”", i.title))
        };
        match guard.as_ref() {
            Some(l) => {
                ui.label(format!(
                    "{} page{} of {title}, standard manuscript, {}",
                    l.pages,
                    if l.pages == 1 { "" } else { "s" },
                    match l.paper {
                        Paper::Letter => "Letter",
                        Paper::A4 => "A4",
                    }
                ));
            }
            None => {
                ui.label(jcj_shell::i18n::tr("Laying out the pages…"));
            }
        }
        if busy && guard.is_some() {
            ui.label(RichText::new("(updating)").weak());
        }
    });
    let mut whole = v.whole;
    ui.checkbox(&mut whole, "Whole manuscript");
    let Some(l) = guard.as_ref() else { return };
    if v.shown_key.as_ref() != Some(&l.key) {
        v.textures.clear();
        v.shown_key = Some(l.key.clone());
    }
    let (pw, ph) = l.paper.size();
    let ppp = ui.ctx().pixels_per_point();
    egui::ScrollArea::vertical().id_salt("manuscript-pages").auto_shrink([false, false]).show_viewport(ui, |ui, viewport| {
        let width = (ui.available_width() - 32.0).clamp(200.0, 820.0);
        let height = width * ph / pw;
        let gap = 16.0;
        let total = l.pages as f32 * (height + gap);
        let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), total), egui::Sense::hover());
        let first = ((viewport.min.y / (height + gap)).floor().max(0.0)) as usize;
        let last = ((viewport.max.y / (height + gap)).ceil() as usize).min(l.pages);
        let px = (width * ppp).round() as u32;
        for n in first..last {
            let top = rect.min.y + n as f32 * (height + gap);
            let r = egui::Rect::from_min_size(egui::pos2(rect.center().x - width / 2.0, top), egui::vec2(width, height));
            ui.painter().rect_filled(r.translate(egui::vec2(2.0, 3.0)), 2.0, egui::Color32::from_black_alpha(40));
            let tex = v.textures.entry((n, px)).or_insert_with(|| {
                let scale = px as f32 / pw;
                let (w, h, rgba) = standard::page_pixels(&l.pag, n, scale).unwrap_or((1, 1, vec![255; 4]));
                let img = egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba);
                ui.ctx().load_texture(format!("manuscript-page-{n}"), img, egui::TextureOptions::LINEAR)
            });
            ui.painter().rect_filled(r, 0.0, egui::Color32::WHITE);
            ui.painter().image(tex.id(), r, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
            let resp = ui.interact(r, ui.scope_id().with(("page", n)), egui::Sense::hover());
            resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Image, true, format!("Page {} of {}", n + 1, l.pages)));
        }
    });
    drop(guard);
    if whole != v.whole {
        v.whole = whole;
    }
}
