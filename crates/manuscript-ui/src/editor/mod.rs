// SPDX-License-Identifier: MIT OR Apache-2.0
//! The text editor: one item's text (or several items stitched together, [`crate::stitch`]) as
//! a `jcj_doc::Document` in a `jcj-engine` session, drawn paragraph by paragraph.
//!
//! Every edit goes through the engine's commands, so undo, the command line and agents see the
//! same steps. Only the paragraphs in view are laid out; laid-out paragraphs are kept until they
//! change, so long texts scroll and type smoothly.
//!
//! The editor offers only Manuscript's styles (ED-1): body, block quote, verse, aside, headings
//! for front matter pages and scene breaks; italic, bold, small caps, strikethrough and a
//! language. Pasted text arrives as plain text, so nothing the format cannot hold gets in.

mod draw;
mod input;
pub mod styles;

use std::collections::HashMap;
use std::sync::Arc;

use jcj_doc::{Document, Paragraph, Path, Pos, StoryRef};
use jcj_engine::{Selection, Session};
use jcj_shell::egui::{self, Galley};
use serde_json::{Value, json};
use wordsmith_manuscript_model::doc::{self as mdoc, BlockTag};
use wordsmith_manuscript_model::{ItemId, Text};

pub use draw::EditorLook;

/// A laid-out paragraph, kept while its revision and the width stay the same.
#[derive(Clone)]
struct Laid {
    rev: u64,
    width: f32,
    first: bool,
    galley: Arc<Galley>,
    /// Space above and below, and the left inset.
    above: f32,
    below: f32,
    inset: f32,
}

/// An open editor.
pub struct Editor {
    /// The engine session holding the document.
    pub session: Session,
    /// The item edited (for a stitched view, the first item).
    pub item: ItemId,
    /// The paragraphs' heights, by index, as last laid out (estimates for the rest).
    heights: Vec<f32>,
    cache: HashMap<usize, Laid>,
    /// A revision that changes with every edit.
    pub revision: u64,
    /// The revision last written back to the project.
    pub written: u64,
    /// The x position kept while moving up and down.
    goal_x: Option<f32>,
    /// Scroll so the caret shows on the next frame.
    pub reveal: bool,
    /// Keep the caret's line at a fixed height (typewriter scrolling, ED-3).
    pub typewriter: bool,
    /// Read-only (an item held by a stitched view elsewhere).
    pub read_only: bool,
    /// The name screen readers give the editor: the document's title (ACC-3).
    pub label: String,
    /// The color of each revision pass, by pass number (PRG-4).
    pub pass_colors: Vec<egui::Color32>,
    /// The last error, for the status bar.
    pub error: Option<String>,
    /// Where the caret was drawn last frame (screen), for typewriter scrolling and menus.
    pub caret_rect: Option<egui::Rect>,
    /// Smart typing rules (ED-5).
    pub smart: crate::typing::Smart,
    /// The last replacement smart typing made, so the next Backspace can undo it.
    pub last_smart: Option<crate::typing::Replaced>,
    /// Spelling marks (ED-11), when the app turned spelling on.
    pub spell: Option<crate::spelling::SpellCache>,
    /// The spelling menu: on which word, and where.
    pub menu: Option<(usize, std::ops::Range<usize>, egui::Pos2)>,
    /// The grammar hint menu: the paragraph, the hint, and where.
    pub hint_menu: Option<(usize, crate::spelling::Hint, egui::Pos2)>,
    /// A word the writer added to the project's words from the menu (the app runs the command).
    pub add_word: Option<String>,
    /// The link under the pointer and where (the app shows its preview).
    pub hover_link: Option<(String, egui::Pos2)>,
    /// A link the writer followed (command-click).
    pub follow: Option<String>,
}

/// The registry every editor session uses: the engine's commands and the core's features.
pub fn registry() -> Arc<jcj_engine::Registry> {
    Arc::new(jcj_engine::registry_with(jcj_engine::commands::merged(Vec::new())))
}

/// Type `text` at the selection; in revision mode it takes the current pass's mark, and out of
/// it text typed next to revised text is not revised (the core's typing rules for its marks).
pub fn typed(s: &mut Session, text: &str) -> Result<(), jcj_engine::CmdError> {
    with_marks(s, |s| jcj_engine::cmd::type_text(s, text))
}

/// [`typed`] for text that was not typed (pasted, replaced, put back).
pub fn pasted(s: &mut Session, text: &str) -> Result<(), jcj_engine::CmdError> {
    with_marks(s, |s| jcj_engine::cmd::paste_text(s, text))
}

fn with_marks(s: &mut Session, f: impl FnOnce(&mut Session) -> Result<(), jcj_engine::CmdError>) -> Result<(), jcj_engine::CmdError> {
    let props = jcj_features::editing::typing_props(s);
    let held = s.pending.replace(props);
    let out = f(s);
    s.pending = held;
    out
}

/// A position at byte `off` of top-level paragraph `i`.
pub fn pos(i: usize, off: usize) -> Pos {
    Pos::new(StoryRef::Body, Path::top(i), off)
}

/// The top-level paragraph index of a position.
pub fn para_index(p: &Pos) -> usize {
    p.path.0.first().map_or(0, |i| *i as usize)
}

impl Editor {
    /// An editor on an item's text.
    pub fn new(item: ItemId, text: &Text, registry: Arc<jcj_engine::Registry>) -> Editor {
        Editor::on_document(item, mdoc::to_document(text), registry)
    }

    /// An editor on a document already built.
    pub fn on_document(item: ItemId, doc: Document, registry: Arc<jcj_engine::Registry>) -> Editor {
        let mut session = jcj_engine::session_with(doc, registry);
        session.sel = Selection::caret(pos(0, 0));
        Editor {
            session,
            item,
            heights: Vec::new(),
            cache: HashMap::new(),
            revision: 0,
            written: 0,
            goal_x: None,
            reveal: false,
            typewriter: false,
            read_only: false,
            pass_colors: Vec::new(),
            label: String::new(),
            error: None,
            caret_rect: None,
            smart: crate::typing::Smart::default(),
            last_smart: None,
            spell: None,
            menu: None,
            hint_menu: None,
            add_word: None,
            hover_link: None,
            follow: None,
        }
    }

    /// The text the editor holds now.
    pub fn text(&self) -> Text {
        mdoc::from_document(&self.session.doc)
    }

    /// Whether there are edits not yet written back to the project.
    pub fn is_dirty(&self) -> bool {
        self.revision != self.written
    }

    /// The top-level paragraphs.
    pub fn paragraphs(&self) -> Vec<&Paragraph> {
        self.session.doc.body.iter().filter_map(|b| b.as_para()).collect()
    }

    /// Paragraph `i`.
    pub fn paragraph(&self, i: usize) -> Option<&Paragraph> {
        self.session.doc.body.get(i).and_then(|b| b.as_para())
    }

    /// Change paragraph `i` directly (style changes made outside the engine's commands).
    fn paragraph_mut(&mut self, i: usize) -> Option<&mut Paragraph> {
        match self.session.doc.body.get_mut(i).map(Arc::make_mut) {
            Some(jcj_doc::Block::Para(p)) => Some(p),
            _ => None,
        }
    }

    /// Keep the caret and the selection's ends out of dividers: an end on a divider moves to the
    /// start of the text after it.
    pub fn fix_caret(&mut self) {
        let n = self.paragraphs().len();
        let fix = |p: &Pos, ed: &Editor| -> Option<Pos> {
            let mut i = para_index(p);
            if !ed.paragraph(i).is_some_and(crate::stitch::is_divider) {
                return None;
            }
            while i < n && ed.paragraph(i).is_some_and(crate::stitch::is_divider) {
                i += 1;
            }
            Some(if i < n {
                pos(i, 0)
            } else {
                pos(para_index(p).saturating_sub(1), ed.paragraph(para_index(p).saturating_sub(1)).map_or(0, |q| q.text.len()))
            })
        };
        let (a, f) = (self.session.sel.anchor.clone(), self.session.sel.focus.clone());
        if let Some(na) = fix(&a, self) {
            self.session.sel.anchor = na;
        }
        if let Some(nf) = fix(&f, self) {
            self.session.sel.focus = nf;
        }
    }

    /// Run an engine command; a change bumps the revision.
    pub fn run(&mut self, id: &str, params: &Value) -> Result<Value, String> {
        let mutates = self.session.registry.get(id).is_some_and(|c| c.mutates);
        if self.read_only && mutates {
            return Err("this text is being edited in a stitched view".into());
        }
        if mutates {
            self.fix_caret();
        }
        let before = self.session.doc.body.clone();
        let guard = crate::stitch::is_stitched(self).then(|| crate::stitch::dividers(&self.session.doc));
        let out = self.session.run(id, params).map_err(|e| format!("{e:?}"))?;
        if let Some(dividers) = guard
            && crate::stitch::dividers(&self.session.doc) != dividers
        {
            // The edit would have removed a divider: undo it and edit each item's part instead.
            self.session.run("edit.undo", &json!({})).map_err(|e| format!("{e:?}"))?;
            let text = match id {
                "text.insert" => params.get("text").and_then(Value::as_str).map(str::to_owned),
                _ => None,
            };
            self.across_dividers(text.as_deref(), false)?;
            return Ok(json!({}));
        }
        if !same_blocks(&before, &self.session.doc.body) {
            self.changed();
        }
        self.reveal = true;
        Ok(out)
    }

    /// Run a direct session edit (typing, pasting) with the dividers of a stitched view kept.
    pub(crate) fn guarded(
        &mut self,
        insert: &str,
        paste: bool,
        f: &dyn Fn(&mut Session, &str) -> Result<(), jcj_engine::CmdError>,
    ) -> Result<(), String> {
        self.fix_caret();
        let guard = crate::stitch::is_stitched(self)
            .then(|| (crate::stitch::dividers(&self.session.doc), self.session.doc.clone(), self.session.sel.clone()));
        // Typing joins the open typing step; a paste is a step of its own.
        self.session.checkpoint(if paste { "Paste" } else { "Typing" });
        f(&mut self.session, insert).map_err(|e| format!("{e:?}"))?;
        if let Some((dividers, doc, sel)) = guard
            && crate::stitch::dividers(&self.session.doc) != dividers
        {
            self.session.doc = doc;
            self.session.sel = sel;
            self.session.close_typing();
            self.across_dividers(Some(insert), paste)?;
        }
        self.changed();
        Ok(())
    }

    /// Delete the selection item by item (never a divider), then type or paste `insert` at its
    /// start. With nothing selected (Backspace at an item's start, Delete at its end) nothing
    /// happens: items are never merged.
    fn across_dividers(&mut self, insert: Option<&str>, paste: bool) -> Result<(), String> {
        if self.session.sel.is_collapsed() {
            return Ok(());
        }
        let (a, b) = self.session.sel.ordered();
        let (ia, ib) = (para_index(&a), para_index(&b));
        // The parts of the selection between dividers, last first.
        let mut parts: Vec<(Pos, Pos)> = Vec::new();
        let mut start: Option<Pos> = Some(a.clone());
        for i in ia..=ib {
            let divider = self.paragraph(i).is_some_and(crate::stitch::is_divider);
            if divider {
                if let Some(st) = start.take()
                    && i > 0
                {
                    let end = pos(i - 1, self.paragraph(i - 1).map_or(0, |p| p.text.len()));
                    if st < end {
                        parts.push((st, end));
                    }
                }
                continue;
            }
            if start.is_none() {
                start = Some(pos(i, 0));
            }
        }
        if let Some(st) = start
            && st < b
        {
            parts.push((st, b.clone()));
        }
        self.session.checkpoint("Delete");
        for (from, to) in parts.iter().rev() {
            self.session.sel = Selection { anchor: from.clone(), focus: to.clone() };
            self.session.join_next_undo();
            self.session.run("text.delete", &json!({})).map_err(|e| format!("{e:?}"))?;
        }
        let first = parts.first().map_or(a, |(f, _)| f.clone());
        self.session.sel = Selection::caret(first);
        if let Some(t) = insert {
            self.session.join_next_undo();
            if paste {
                for (k, line) in t.split('\n').enumerate() {
                    if k > 0 {
                        self.session.join_next_undo();
                        self.session.run("text.newParagraph", &json!({})).map_err(|e| format!("{e:?}"))?;
                    }
                    if !line.is_empty() {
                        self.session.join_next_undo();
                        pasted(&mut self.session, line).map_err(|e| format!("{e:?}"))?;
                    }
                }
            } else {
                typed(&mut self.session, t).map_err(|e| format!("{e:?}"))?;
            }
        }
        self.changed();
        self.reveal = true;
        Ok(())
    }

    /// Note a change made to the document.
    pub fn changed(&mut self) {
        self.revision += 1;
    }

    /// Type text at the caret as the writer types it (with smart typing).
    pub fn type_text(&mut self, s: &str) -> Result<(), String> {
        if self.read_only {
            return Err("this text is being edited in a stitched view".into());
        }
        let i = para_index(&self.session.sel.focus);
        if self.paragraph(i).is_some_and(|p| !mdoc::is_text(&mdoc::tag_of(p))) {
            // Typing on a scene break, picture or raw block starts a body paragraph after it.
            self.run("caret.end", &json!({}))?;
            self.new_paragraph()?;
        }
        let replaced = crate::typing::type_smart(self, s)?;
        self.last_smart = replaced;
        self.changed();
        self.reveal = true;
        Ok(())
    }

    /// Return: a new paragraph. After a heading, scene break, picture or raw block the new
    /// paragraph is a body paragraph; Return on an empty quote, verse or aside paragraph ends
    /// the block.
    pub fn new_paragraph(&mut self) -> Result<(), String> {
        let i = para_index(&self.session.sel.focus);
        let tag = self.paragraph(i).map(mdoc::tag_of).unwrap_or(BlockTag::Body);
        let empty = self.paragraph(i).is_some_and(|p| p.text.is_empty());
        let in_block = matches!(tag, BlockTag::Quote { .. } | BlockTag::Verse { .. } | BlockTag::Aside { .. });
        if in_block && empty {
            styles::set_block(self, i, &BlockTag::Body);
            return Ok(());
        }
        self.run("text.newParagraph", &json!({}))?;
        let j = para_index(&self.session.sel.focus);
        let next = match tag {
            BlockTag::Heading { .. } | BlockTag::SceneBreak | BlockTag::Image { .. } | BlockTag::Raw { .. } | BlockTag::EmptyFence { .. } => {
                Some(BlockTag::Body)
            }
            _ => None,
        };
        if let Some(t) = next {
            styles::set_block(self, j, &t);
        }
        Ok(())
    }

    /// Move the caret to paragraph `i`, byte `off`.
    pub fn set_caret(&mut self, i: usize, off: usize, extend: bool) {
        let p = pos(i, off);
        if extend {
            self.session.sel.focus = p;
        } else {
            self.session.sel = Selection::caret(p);
        }
        self.reveal = true;
    }

    /// The selected paragraphs' indexes (the caret's alone when nothing is selected).
    pub fn selected_paragraphs(&self) -> std::ops::RangeInclusive<usize> {
        let (a, b) = self.session.sel.ordered();
        para_index(&a)..=para_index(&b)
    }

    /// The plain text of the selection.
    pub fn selected_text(&self) -> String {
        let (a, b) = self.session.sel.ordered();
        let mut out = String::new();
        for i in para_index(&a)..=para_index(&b) {
            let Some(p) = self.paragraph(i) else { continue };
            let from = if i == para_index(&a) { a.off.min(p.text.len()) } else { 0 };
            let to = if i == para_index(&b) { b.off.min(p.text.len()) } else { p.text.len() };
            if i > para_index(&a) {
                out.push('\n');
            }
            out.push_str(p.text.get(from..to).unwrap_or(""));
        }
        out
    }

    /// Words in the document and in the selection.
    pub fn counts(&self) -> (usize, usize) {
        let all = self.paragraphs().iter().filter(|p| !crate::stitch::is_divider(p)).map(|p| jcj_doc::count_words(&p.text)).sum();
        let sel = if self.session.sel.is_collapsed() { 0 } else { jcj_doc::count_words(&self.selected_text()) };
        (all, sel)
    }

    /// Forget laid-out paragraphs (after the look changed).
    pub fn relayout(&mut self) {
        self.cache.clear();
    }

    /// Run a Manuscript editor command (`write.*`, see [`styles`]) or an engine command.
    pub fn command(&mut self, id: &str, params: &Value) -> Result<(), String> {
        if let Some(r) = styles::run(self, id, params) {
            return r;
        }
        self.run(id, params).map(|_| ())
    }
}

/// Whether two block lists hold the very same blocks (nothing was replaced).
fn same_blocks(a: &[Arc<jcj_doc::Block>], b: &[Arc<jcj_doc::Block>]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| Arc::ptr_eq(x, y))
}
