// SPDX-License-Identifier: MIT OR Apache-2.0
//! Writing text in the dialect. The output is canonical: [`super::parse`] reads it back to the
//! same (normalized) text.

use wordsmith_manuscript_model::{Block, Fence, Inline, Marks, Note, Text};

use super::{EMPTY_PARA, fence_for};

/// Write a text's body (no front matter). Empty text gives an empty string; otherwise the result
/// ends with one newline.
pub fn write(text: &Text) -> String {
    let mut text = text.clone();
    super::canonical(&mut text);
    let mut blocks: Vec<String> = text.blocks.iter().map(block).collect();
    blocks.extend(text.notes.iter().map(note));
    if blocks.is_empty() {
        return String::new();
    }
    let mut out = blocks.join("\n\n");
    out.push('\n');
    out
}

fn block(b: &Block) -> String {
    match b {
        Block::Para(c) => para(c, true),
        Block::Heading { level, content } => {
            let hashes = "#".repeat(usize::from(*level));
            if content.is_empty() { hashes } else { format!("{hashes} {}", para(content, false)) }
        }
        Block::SceneBreak => "***".to_owned(),
        Block::Quote(ps) => {
            let paras: Vec<String> = ps.iter().map(|p| prefix_lines(&para(p, true), "> ", "> ")).collect();
            paras.join("\n>\n")
        }
        Block::Fenced { kind, paras } => {
            let head = match kind {
                Fence::Verse => "::: verse".to_owned(),
                Fence::Aside(None) => "::: aside".to_owned(),
                Fence::Aside(Some(k)) => format!("::: aside {k}"),
            };
            let mut out = head;
            for p in paras {
                out.push('\n');
                out.push_str(&para(p, true));
                out.push('\n');
            }
            out.push_str(if paras.is_empty() { "\n:::" } else { ":::" });
            out
        }
        Block::Image { alt, src, role } => format!("![{}]({}){{role={}}}", escape_attr(alt), escape_dest(src), role.key()),
        Block::Raw { format, text } => {
            let fence = fence_for(text);
            format!("{fence}raw {format}\n{text}\n{fence}")
        }
    }
}

fn note(n: &Note) -> String {
    let mut out = format!("[^{}]:", n.label);
    for (i, p) in n.paras.iter().enumerate() {
        let body = para(p, false);
        if i == 0 {
            out.push(' ');
            out.push_str(&prefix_lines(&body, "", "    "));
        } else {
            out.push_str("\n\n");
            out.push_str(&prefix_lines(&body, "    ", "    "));
        }
    }
    out
}

/// Prefix the first physical line with `first` and every following line with `rest`.
fn prefix_lines(s: &str, first: &str, rest: &str) -> String {
    let mut out = String::with_capacity(s.len() + 8);
    for (i, line) in s.split('\n').enumerate() {
        if i > 0 {
            out.push('\n');
            out.push_str(rest);
        } else {
            out.push_str(first);
        }
        out.push_str(line);
    }
    out
}

/// One paragraph's content as dialect lines. `block_start` escapes what would otherwise start
/// another kind of block at the beginning of a line.
fn para(content: &[Inline], block_start: bool) -> String {
    if content.is_empty() {
        return EMPTY_PARA.to_owned();
    }
    let mut out = String::new();
    level(content, 0, &mut out);
    let mut out = edges(&out);
    // A paragraph starting with a line break would begin with a line holding only `\`, which is
    // the empty paragraph: write that first break as an entity instead.
    if let Some(rest) = out.strip_prefix("\\\n") {
        out = format!("&#10;{rest}");
    }
    if block_start { escape_block_start(out) } else { out }
}

/// Encode spaces and tabs at the very start and end of the paragraph, which Markdown would drop.
fn edges(s: &str) -> String {
    let lead = s.len() - s.trim_start_matches([' ', '\t']).len();
    let trail_start = s.trim_end_matches([' ', '\t']).len().max(lead);
    let mut out = String::with_capacity(s.len() + 8);
    for c in s[..lead].chars() {
        out.push_str(entity(c));
    }
    out.push_str(&s[lead..trail_start]);
    for c in s[trail_start..].chars() {
        out.push_str(entity(c));
    }
    out
}

fn entity(c: char) -> &'static str {
    if c == '\t' { "&#9;" } else { "&#32;" }
}

fn escape_block_start(s: String) -> String {
    let first = s.chars().next();
    let needs = match first {
        Some('#' | '>' | '+' | '-' | '=' | '|') => true,
        Some(':') => s.starts_with(":::"),
        Some('`') => s.starts_with("```"),
        Some(c) if c.is_ascii_digit() => {
            let rest = s.trim_start_matches(|c: char| c.is_ascii_digit());
            (rest.starts_with(". ") || rest.starts_with(") ") || rest == "." || rest == ")") && s.len() - rest.len() <= 9
        }
        _ => false,
    };
    if !needs {
        return s;
    }
    if first.is_some_and(|c| c.is_ascii_digit()) {
        let digits = s.len() - s.trim_start_matches(|c: char| c.is_ascii_digit()).len();
        return format!("{}\\{}", &s[..digits], &s[digits..]);
    }
    format!("\\{s}")
}

/// The marks, outermost first. Each is written as a wrapper around everything inside it.
#[derive(Clone, Copy)]
enum Mark {
    Comment,
    Link,
    Lang,
    SmallCaps,
    Bold,
    Italic,
    Strike,
}

const ORDER: [Mark; 7] = [Mark::Comment, Mark::Link, Mark::Lang, Mark::SmallCaps, Mark::Bold, Mark::Italic, Mark::Strike];

/// The value of one mark, as a key for grouping (`None` when the mark is off).
fn value(m: &Marks, mark: Mark) -> Option<&str> {
    match mark {
        Mark::Comment => m.comment.as_deref(),
        Mark::Link => m.link.as_deref(),
        Mark::Lang => m.lang.as_deref(),
        Mark::SmallCaps => m.small_caps.then_some(""),
        Mark::Bold => m.bold.then_some(""),
        Mark::Italic => m.italic.then_some(""),
        Mark::Strike => m.strike.then_some(""),
    }
}

fn level(items: &[Inline], depth: usize, out: &mut String) {
    let Some(mark) = ORDER.get(depth).copied() else {
        for i in items {
            leaf(i, out);
        }
        return;
    };
    let mut start = 0;
    while start < items.len() {
        let v = items.get(start).and_then(|i| value(i.marks(), mark));
        let mut end = start + 1;
        while end < items.len() && items.get(end).and_then(|i| value(i.marks(), mark)) == v {
            end += 1;
        }
        let group = items.get(start..end).unwrap_or_default();
        match v {
            None => level(group, depth + 1, out),
            Some(v) => {
                let (open, close) = wrapper(mark, v);
                out.push_str(&open);
                level(group, depth + 1, out);
                out.push_str(&close);
            }
        }
        start = end;
    }
}

fn wrapper(mark: Mark, v: &str) -> (String, String) {
    match mark {
        Mark::Comment => ("[".into(), format!("]{{.comment id={v}}}")),
        Mark::Link => ("[".into(), format!("]({})", escape_dest(v))),
        Mark::Lang => ("[".into(), format!("]{{lang={v}}}")),
        Mark::SmallCaps => ("[".into(), "]{.smallcaps}".into()),
        Mark::Bold => ("**".into(), "**".into()),
        Mark::Italic => ("*".into(), "*".into()),
        Mark::Strike => ("~~".into(), "~~".into()),
    }
}

fn leaf(i: &Inline, out: &mut String) {
    match i {
        Inline::Span { text, .. } => escape_text(text, out),
        Inline::NoteRef { label, .. } => {
            out.push_str("[^");
            out.push_str(label);
            out.push(']');
        }
    }
}

/// Escape span text: dialect punctuation gets a backslash, a line break is a backslash at the end
/// of the line, and a carriage return is an entity.
fn escape_text(text: &str, out: &mut String) {
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' | '*' | '[' | ']' | '~' | '_' | '<' | '`' => {
                out.push('\\');
                out.push(c);
            }
            '&' if chars.peek() == Some(&'#') => out.push_str("\\&"),
            // `[^` would start a note reference.
            '^' if out.ends_with('[') => out.push_str("\\^"),
            '\n' => out.push_str("\\\n"),
            '\r' => out.push_str("&#13;"),
            c => out.push(c),
        }
    }
}

/// Escape image alt text: one line, no closing bracket.
fn escape_attr(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        match c {
            '\\' | '[' | ']' => {
                out.push('\\');
                out.push(c);
            }
            '&' => out.push_str("\\&"),
            '\n' => out.push_str("&#10;"),
            '\r' => out.push_str("&#13;"),
            c => out.push(c),
        }
    }
    out
}

/// Escape a link or image destination: no spaces, parentheses or line breaks unescaped.
fn escape_dest(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        match c {
            '\\' | '(' | ')' | ' ' => {
                out.push('\\');
                out.push(c);
            }
            '\n' => out.push_str("&#10;"),
            '\r' => out.push_str("&#13;"),
            '&' => out.push_str("\\&"),
            c => out.push(c),
        }
    }
    out
}
