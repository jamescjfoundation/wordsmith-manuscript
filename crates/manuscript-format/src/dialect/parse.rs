// SPDX-License-Identifier: MIT OR Apache-2.0
//! Reading blocks: the line-based part of the dialect.

use wordsmith_manuscript_model::{Block, Fence, ImageRole, Inlines, Note, Text};

use super::EMPTY_PARA;
use super::inline::{inlines, unescape};

/// Whether a line ends with a backslash that is not itself escaped (a line break follows).
fn continues(line: &str) -> bool {
    let n = line.len() - line.trim_end_matches('\\').len();
    n % 2 == 1
}

fn is_blank(line: &str) -> bool {
    line.trim().is_empty()
}

/// Read a body (no front matter). Never fails: text that is not dialect syntax is read as
/// paragraphs.
pub fn parse(src: &str) -> Text {
    let lines: Vec<&str> = src.split('\n').collect();
    // A final newline does not start another line.
    let lines = match lines.split_last() {
        Some((last, rest)) if last.is_empty() => rest.to_vec(),
        _ => lines,
    };
    let mut p = Parser { lines, i: 0 };
    let mut text = Text::new();
    while p.i < p.lines.len() {
        let line = p.line(p.i);
        if is_blank(line) {
            p.i += 1;
            continue;
        }
        if note_start(line).is_some() {
            text.notes = p.notes();
            break;
        }
        text.blocks.push(p.block());
    }
    text
}

struct Parser<'a> {
    lines: Vec<&'a str>,
    i: usize,
}

/// `[^label]:` at the start of a line: the label and the rest of the line.
fn note_start(line: &str) -> Option<(&str, &str)> {
    let rest = line.strip_prefix("[^")?;
    let n = rest.chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_').count();
    if n == 0 {
        return None;
    }
    let after = rest[n..].strip_prefix("]:")?;
    Some((&rest[..n], after.strip_prefix(' ').unwrap_or(after)))
}

fn heading(line: &str) -> Option<(u8, &str)> {
    let n = line.bytes().take_while(|b| *b == b'#').count();
    if !(1..=3).contains(&n) {
        return None;
    }
    let rest = &line[n..];
    if rest.is_empty() {
        return Some((n as u8, ""));
    }
    rest.strip_prefix(' ').map(|r| (n as u8, r))
}

fn image(line: &str) -> Option<Block> {
    let rest = line.strip_prefix("![")?;
    // The alt text ends at the first `]` not escaped.
    let bytes = rest.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes.get(i) {
            Some(b'\\') => i += 2,
            Some(b']') => break,
            _ => i += 1,
        }
    }
    if i >= bytes.len() || !rest.is_char_boundary(i) {
        return None;
    }
    let alt = unescape(&rest[..i]);
    let after = rest[i + 1..].strip_prefix('(')?;
    let bytes = after.as_bytes();
    let mut j = 0;
    while j < bytes.len() {
        match bytes.get(j) {
            Some(b'\\') => j += 2,
            Some(b')') => break,
            Some(b' ') => return None,
            _ => j += 1,
        }
    }
    if j >= bytes.len() || !after.is_char_boundary(j) {
        return None;
    }
    let src = unescape(&after[..j]);
    let tail = &after[j + 1..];
    let role = if tail.is_empty() {
        ImageRole::Inline
    } else {
        let r = tail.strip_prefix("{role=")?.strip_suffix('}')?;
        ImageRole::from_key(r)?
    };
    Some(Block::Image { alt, src, role })
}

impl<'a> Parser<'a> {
    fn line(&self, i: usize) -> &'a str {
        let l = self.lines.get(i).copied().unwrap_or("");
        l.strip_suffix('\r').unwrap_or(l)
    }

    fn raw_line(&self, i: usize) -> &'a str {
        self.lines.get(i).copied().unwrap_or("")
    }

    /// Read the lines of one paragraph starting at `self.i`, following line breaks. `strip` takes
    /// a line's prefix off (quotes, notes); it returns `None` when the line does not belong.
    fn para_lines(&mut self, strip: &dyn Fn(&'a str) -> Option<&'a str>) -> Option<Inlines> {
        let first = strip(self.line(self.i))?;
        self.i += 1;
        Some(self.para_from(first, strip))
    }

    /// A paragraph whose first line (already read, prefix removed) is `first`.
    fn para_from(&mut self, first: &str, strip: &dyn Fn(&'a str) -> Option<&'a str>) -> Inlines {
        if first == EMPTY_PARA {
            return Vec::new();
        }
        let mut joined = String::from(first);
        let mut last = first;
        while continues(last) && self.i < self.lines.len() {
            let Some(next) = strip(self.line(self.i)) else { break };
            joined.push('\n');
            joined.push_str(next);
            last = next;
            self.i += 1;
        }
        inlines(&joined)
    }

    fn block(&mut self) -> Block {
        let line = self.line(self.i);
        if line == "***" {
            self.i += 1;
            return Block::SceneBreak;
        }
        if let Some((level, first)) = heading(line) {
            self.i += 1;
            let content = if first.is_empty() { Vec::new() } else { self.para_from(first, &Some) };
            return Block::Heading { level, content };
        }
        if line.starts_with('>') {
            return self.quote();
        }
        if let Some(head) = line.strip_prefix(":::") {
            return self.fence(head.trim());
        }
        if line.starts_with("```") {
            let n = line.bytes().take_while(|b| *b == b'`').count();
            let info = line[n..].trim();
            if let Some(format) = info.strip_prefix("raw ").map(str::trim).filter(|f| !f.is_empty()) {
                return self.raw(&line[..n], format);
            }
        }
        if let Some(img) = image(line) {
            self.i += 1;
            return img;
        }
        Block::Para(self.para_lines(&Some).unwrap_or_default())
    }

    fn quote(&mut self) -> Block {
        let strip = |l: &'a str| l.strip_prefix("> ").or_else(|| l.strip_prefix('>'));
        let mut paras = Vec::new();
        while self.i < self.lines.len() {
            let Some(rest) = strip(self.line(self.i)) else { break };
            if is_blank(rest) {
                // A `>` line on its own separates the quote's paragraphs.
                self.i += 1;
                continue;
            }
            if let Some(p) = self.para_lines(&strip) {
                paras.push(p);
            }
        }
        if paras.is_empty() {
            paras.push(Vec::new());
        }
        Block::Quote(paras)
    }

    fn fence(&mut self, head: &str) -> Block {
        let kind = match head.split_once(char::is_whitespace).map_or((head, ""), |(a, b)| (a, b.trim())) {
            ("verse", _) => Fence::Verse,
            ("aside", "") => Fence::Aside(None),
            ("aside", k) => Fence::Aside(Some(k.to_owned())),
            ("", _) => Fence::Aside(None),
            _ => Fence::Aside(Some(head.to_owned())),
        };
        self.i += 1;
        let mut paras = Vec::new();
        while self.i < self.lines.len() {
            let line = self.line(self.i);
            if line.trim_end() == ":::" {
                self.i += 1;
                break;
            }
            if is_blank(line) {
                self.i += 1;
                continue;
            }
            if let Some(p) = self.para_lines(&|l| if l.trim_end() == ":::" { None } else { Some(l) }) {
                paras.push(p);
            }
        }
        Block::Fenced { kind, paras }
    }

    fn raw(&mut self, fence: &str, format: &str) -> Block {
        self.i += 1;
        let mut body: Vec<&str> = Vec::new();
        let mut closed = false;
        while self.i < self.lines.len() {
            let line = self.raw_line(self.i);
            self.i += 1;
            if line.strip_suffix('\r').unwrap_or(line) == fence {
                closed = true;
                break;
            }
            body.push(line);
        }
        let _ = closed;
        Block::Raw { format: format.to_owned(), text: body.join("\n") }
    }

    fn notes(&mut self) -> Vec<Note> {
        let mut notes: Vec<Note> = Vec::new();
        while self.i < self.lines.len() {
            let line = self.line(self.i);
            if is_blank(line) && line.len() < 4 {
                self.i += 1;
                continue;
            }
            if let Some((label, rest)) = note_start(line) {
                let label = label.to_owned();
                let mut paras = Vec::new();
                self.i += 1;
                if !(rest.is_empty() && !line.ends_with(' ')) {
                    // The first paragraph starts on the label's line; its later lines are indented.
                    paras.push(self.para_from(rest, &|l| Some(l.strip_prefix("    ").unwrap_or(l))));
                }
                // Further paragraphs: indented four spaces, after a blank line.
                loop {
                    let mut j = self.i;
                    while j < self.lines.len() && self.line(j).is_empty() {
                        j += 1;
                    }
                    if j == self.i || j >= self.lines.len() || !self.line(j).starts_with("    ") {
                        break;
                    }
                    self.i = j;
                    if let Some(p) = self.para_lines(&|l| l.strip_prefix("    ")) {
                        paras.push(p);
                    }
                }
                notes.push(Note { label, paras });
            } else {
                // Not a note: keep the words as a paragraph of the last note (or a new one).
                let p = self.para_lines(&Some).unwrap_or_default();
                match notes.last_mut() {
                    Some(n) => n.paras.push(p),
                    None => notes.push(Note { label: "n".into(), paras: vec![p] }),
                }
            }
        }
        notes
    }
}
