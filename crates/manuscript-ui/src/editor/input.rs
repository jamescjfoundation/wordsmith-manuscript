// SPDX-License-Identifier: MIT OR Apache-2.0
//! Keyboard input for the editor: typing, Return, deleting, moving the caret, the clipboard.
//!
//! Keys bound to commands (Mod+B, Mod+Z, ...) are taken by the shell first and arrive as
//! commands; what reaches the editor is typing and plain editing keys.

use jcj_shell::egui::text::CCursor;
use jcj_shell::egui::{self, Event, Key, Ui, vec2};
use serde_json::json;

use super::draw::{byte_offset, char_index};
use super::{Editor, EditorLook, para_index};

impl Editor {
    pub(super) fn input(&mut self, ui: &mut Ui, look: &EditorLook) {
        let events = ui.input(|i| i.events.clone());
        let mac = ui.ctx().os() == egui::os::OperatingSystem::Mac;
        for e in events {
            let r = match e {
                Event::Text(t) => {
                    self.goal_x = None;
                    self.type_text(&t)
                }
                Event::Paste(t) => {
                    self.goal_x = None;
                    self.paste(&t)
                }
                Event::Copy => {
                    let t = self.selected_text();
                    if !t.is_empty() {
                        ui.ctx().copy_text(t);
                    }
                    Ok(())
                }
                Event::Cut => {
                    let t = self.selected_text();
                    if !t.is_empty() {
                        ui.ctx().copy_text(t);
                        self.run("text.delete", &json!({})).map(|_| ())
                    } else {
                        Ok(())
                    }
                }
                Event::Key { key, pressed: true, modifiers, .. } => self.key(key, modifiers, mac, look),
                _ => Ok(()),
            };
            if let Err(e) = r {
                self.error = Some(e);
            }
        }
    }

    /// Paste plain text: line breaks become paragraphs; no formatting comes in.
    pub fn paste(&mut self, text: &str) -> Result<(), String> {
        if self.read_only {
            return Err("this text is being edited in a stitched view".into());
        }
        let text = text.replace("\r\n", "\n").replace('\r', "\n");
        // Each line becomes a paragraph, as one undo step.
        self.guarded(&text, true, &|sess, t| {
            for (k, line) in t.split('\n').enumerate() {
                if k > 0 {
                    sess.join_next_undo();
                    sess.run("text.newParagraph", &json!({}))?;
                }
                if !line.is_empty() {
                    if k > 0 {
                        sess.join_next_undo();
                    }
                    jcj_engine::cmd::paste_text(sess, line)?;
                }
            }
            Ok(())
        })?;
        self.changed();
        self.reveal = true;
        Ok(())
    }

    fn key(&mut self, key: Key, m: egui::Modifiers, mac: bool, _look: &EditorLook) -> Result<(), String> {
        let extend = json!({ "extend": m.shift });
        let word = if mac { m.alt } else { m.ctrl };
        let vertical = matches!(key, Key::ArrowUp | Key::ArrowDown | Key::PageUp | Key::PageDown);
        if !vertical {
            self.goal_x = None;
        }
        match key {
            Key::Enter if m.shift => self.run("text.lineBreak", &json!({})).map(|_| ()),
            Key::Enter => self.new_paragraph(),
            Key::Backspace => {
                if let Some(r) = self.last_smart.take() {
                    return crate::typing::undo_smart(self, &r);
                }
                self.run(if word { "text.deleteWordBack" } else { "text.backspace" }, &json!({})).map(|_| ())
            }
            Key::Delete => self.run(if word { "text.deleteWordForward" } else { "text.delete" }, &json!({})).map(|_| ()),
            Key::ArrowLeft => self.run(if word { "caret.wordLeft" } else { "caret.left" }, &extend).map(|_| ()),
            Key::ArrowRight => self.run(if word { "caret.wordRight" } else { "caret.right" }, &extend).map(|_| ()),
            Key::Home if m.command => self.run("caret.docStart", &extend).map(|_| ()),
            Key::End if m.command => self.run("caret.docEnd", &extend).map(|_| ()),
            Key::Home => self.line_edge(false, m.shift),
            Key::End => self.line_edge(true, m.shift),
            Key::ArrowUp => self.vertical(-1, m.shift),
            Key::ArrowDown => self.vertical(1, m.shift),
            Key::PageUp => (0..20).try_for_each(|_| self.vertical(-1, m.shift)),
            Key::PageDown => (0..20).try_for_each(|_| self.vertical(1, m.shift)),
            Key::Tab => Ok(()),
            _ => Ok(()),
        }
        .map(|()| {
            self.last_smart = None;
        })
    }

    /// Home and End: the start or end of the caret's line on screen.
    fn line_edge(&mut self, end: bool, extend: bool) -> Result<(), String> {
        let Some((i, galley, at)) = self.caret_galley() else {
            return self.run(if end { "caret.end" } else { "caret.home" }, &json!({ "extend": extend })).map(|_| ());
        };
        let Some(row) = galley.rows.iter().find(|r| r.rect().min.y <= at.center().y && at.center().y <= r.rect().max.y) else { return Ok(()) };
        let x = if end { row.rect().max.x + 1000.0 } else { row.rect().min.x - 1000.0 };
        let c = galley.cursor_from_pos(vec2(x, row.rect().center().y));
        let text = self.paragraph(i).map(|p| p.text.clone()).unwrap_or_default();
        self.set_caret(i, byte_offset(&text, c.index.0), extend);
        Ok(())
    }

    /// Up and Down: the line above or below, keeping the x position.
    fn vertical(&mut self, dir: i32, extend: bool) -> Result<(), String> {
        let n = self.paragraphs().len();
        let Some((i, galley, at)) = self.caret_galley() else {
            return self.run(if dir < 0 { "caret.paraUp" } else { "caret.paraDown" }, &json!({ "extend": extend })).map(|_| ());
        };
        let x = *self.goal_x.get_or_insert(at.min.x);
        let row_h = at.height().max(4.0);
        let target_y = at.center().y + dir as f32 * row_h;
        if target_y >= 0.0 && target_y <= galley.size().y {
            let c = galley.cursor_from_pos(vec2(x, target_y));
            let text = self.paragraph(i).map(|p| p.text.clone()).unwrap_or_default();
            let goal = self.goal_x;
            self.set_caret(i, byte_offset(&text, c.index.0), extend);
            self.goal_x = goal;
            return Ok(());
        }
        // Into the next or previous paragraph (skipping dividers).
        let mut j = i as i64 + i64::from(dir);
        while j >= 0 && (j as usize) < n && self.paragraph(j as usize).is_some_and(crate::stitch::is_divider) {
            j += i64::from(dir);
        }
        if j < 0 || j as usize >= n {
            let off = if dir < 0 { 0 } else { self.paragraph(i).map_or(0, |p| p.text.len()) };
            let goal = self.goal_x;
            self.set_caret(i, off, extend);
            self.goal_x = goal;
            return Ok(());
        }
        let j = j as usize;
        let text = self.paragraph(j).map(|p| p.text.clone()).unwrap_or_default();
        let off = match self.galley(j) {
            Some(g) => {
                let y = if dir < 0 { g.size().y - 1.0 } else { 1.0 };
                byte_offset(&text, g.cursor_from_pos(vec2(x, y)).index.0)
            }
            None => byte_offset(&text, char_index(&text, text.len()).min(CCursor::new(0).index.0)),
        };
        let goal = self.goal_x;
        self.set_caret(j, off, extend);
        self.goal_x = goal;
        let _ = para_index;
        Ok(())
    }
}
