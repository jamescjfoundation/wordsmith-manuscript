// SPDX-License-Identifier: MIT OR Apache-2.0
//! Reading a paragraph's inline content.
//!
//! The paragraph's lines (joined with `\n`) are split into tokens, then built into spans with a
//! stack of open markers. When a marker is never closed (hand-written text), that marker is read
//! as plain text and the paragraph is built again, so no character is ever lost.

use std::collections::BTreeSet;

use wordsmith_manuscript_model::{Inline, Inlines, Marks};

use super::{MAX_NESTING, MAX_RETRIES};

#[derive(Clone, Debug, PartialEq)]
enum Tok {
    /// Literal text.
    Text(String),
    /// A run of `*`.
    Stars(usize),
    /// `~~`.
    Tildes,
    /// `[` opening a bracket span.
    Open,
    /// `]` followed by attributes; the source text is kept in case it must be read as text.
    Close(Attrs, String),
    /// `[^label]`.
    NoteRef(String),
}

#[derive(Clone, Debug, Default, PartialEq)]
struct Attrs {
    link: Option<String>,
    lang: Option<String>,
    small_caps: bool,
    comment: Option<String>,
    rev: Option<u32>,
}

fn is_label_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '-' || c == '_'
}

/// Decode `&#NNN;` at `s` (which starts with `&#`). Returns the character and bytes used.
pub(super) fn entity(s: &str) -> Option<(char, usize)> {
    let rest = s.strip_prefix("&#")?;
    let (digits, hex) = match rest.strip_prefix(['x', 'X']) {
        Some(h) => (h, true),
        None => (rest, false),
    };
    let len = digits.chars().take_while(|c| if hex { c.is_ascii_hexdigit() } else { c.is_ascii_digit() }).count();
    if len == 0 || len > 7 || !digits[len..].starts_with(';') {
        return None;
    }
    let n = u32::from_str_radix(&digits[..len], if hex { 16 } else { 10 }).ok()?;
    let c = char::from_u32(n).filter(|c| *c != '\0')?;
    Some((c, 2 + usize::from(hex) + len + 1))
}

/// Unescape a destination or attribute value: backslash escapes and entities.
pub(super) fn unescape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while let Some(c) = s[i..].chars().next() {
        if c == '\\'
            && let Some(n) = s[i + 1..].chars().next()
        {
            out.push(n);
            i += 1 + n.len_utf8();
            continue;
        }
        if c == '&'
            && let Some((e, used)) = entity(&s[i..])
        {
            out.push(e);
            i += used;
            continue;
        }
        out.push(c);
        i += c.len_utf8();
    }
    out
}

/// Read `(dest)` or `{attrs}` at the start of `s`. Returns the attributes and the bytes used.
fn attrs_after(s: &str) -> Option<(Attrs, usize)> {
    if let Some(rest) = s.strip_prefix('(') {
        let mut i = 0;
        let bytes = rest.as_bytes();
        while i < bytes.len() {
            match bytes.get(i) {
                Some(b'\\') => i += 2,
                Some(b')') => break,
                Some(b' ' | b'\n' | b'(') => return None,
                _ => i += 1,
            }
        }
        if i >= bytes.len() || !rest.is_char_boundary(i) {
            return None;
        }
        let dest = unescape(&rest[..i]);
        if dest.is_empty() {
            return None;
        }
        return Some((Attrs { link: Some(dest), ..Attrs::default() }, i + 2));
    }
    let rest = s.strip_prefix('{')?;
    let end = rest.find('}')?;
    let body = &rest[..end];
    if body.contains('\n') || body.trim().is_empty() {
        return None;
    }
    let mut a = Attrs::default();
    for part in body.split_whitespace() {
        if part == ".smallcaps" {
            a.small_caps = true;
        } else if part == ".comment" {
            a.comment.get_or_insert_with(String::new);
        } else if let Some(v) = part.strip_prefix("lang=") {
            a.lang = Some(v.to_owned()).filter(|v| !v.is_empty() && v.chars().all(is_label_char));
            a.lang.as_ref()?;
        } else if let Some(v) = part.strip_prefix("rev=") {
            if v.is_empty() || v.len() > 4 || !v.bytes().all(|b| b.is_ascii_digit()) || (v.len() > 1 && v.starts_with('0')) {
                return None;
            }
            a.rev = Some(v.parse().ok()?);
        } else {
            let v = part.strip_prefix("id=")?;
            if !v.chars().all(is_label_char) || v.is_empty() {
                return None;
            }
            a.comment = Some(v.to_owned());
        }
    }
    if a.comment.as_deref() == Some("") {
        return None;
    }
    Some((a, end + 2))
}

fn tokenize(s: &str) -> Vec<Tok> {
    let mut toks = Vec::new();
    let mut text = String::new();
    let flush = |text: &mut String, toks: &mut Vec<Tok>| {
        if !text.is_empty() {
            toks.push(Tok::Text(std::mem::take(text)));
        }
    };
    let mut i = 0;
    while let Some(c) = s[i..].chars().next() {
        let rest = &s[i..];
        match c {
            '\\' => match rest[1..].chars().next() {
                Some(n) if n.is_ascii_punctuation() || n == '\n' => {
                    text.push(n);
                    i += 1 + n.len_utf8();
                }
                _ => {
                    text.push('\\');
                    i += 1;
                }
            },
            '&' => match entity(rest) {
                Some((e, used)) => {
                    text.push(e);
                    i += used;
                }
                None => {
                    text.push('&');
                    i += 1;
                }
            },
            '*' => {
                flush(&mut text, &mut toks);
                let n = rest.bytes().take_while(|b| *b == b'*').count();
                toks.push(Tok::Stars(n));
                i += n;
            }
            '~' if rest.starts_with("~~") => {
                flush(&mut text, &mut toks);
                toks.push(Tok::Tildes);
                i += 2;
            }
            '[' => {
                flush(&mut text, &mut toks);
                if let Some(label) = rest.strip_prefix("[^") {
                    let n = label.chars().take_while(|c| is_label_char(*c)).count();
                    if n > 0 && label[n..].starts_with(']') {
                        toks.push(Tok::NoteRef(label[..n].to_owned()));
                        i += 2 + n + 1;
                        continue;
                    }
                }
                toks.push(Tok::Open);
                i += 1;
            }
            ']' => {
                flush(&mut text, &mut toks);
                match attrs_after(&rest[1..]) {
                    Some((a, used)) => {
                        toks.push(Tok::Close(a, rest[..1 + used].to_owned()));
                        i += 1 + used;
                    }
                    None => {
                        toks.push(Tok::Text("]".into()));
                        i += 1;
                    }
                }
            }
            c => {
                text.push(c);
                i += c.len_utf8();
            }
        }
    }
    flush(&mut text, &mut toks);
    toks
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Kind {
    Bold,
    Italic,
    Strike,
    Bracket,
}

struct Frame {
    kind: Kind,
    /// The token that opened it, and for a star run the star's offset in the run.
    opener: (usize, usize),
    content: Inlines,
}

/// Where building failed: the opener to read as text next time.
type Opener = (usize, usize);

fn apply(content: &mut Inlines, f: &dyn Fn(&mut Marks)) {
    for i in content.iter_mut() {
        match i {
            Inline::Span { marks, .. } | Inline::NoteRef { marks, .. } => f(marks),
        }
    }
}

fn push_text(stack: &mut [Frame], root: &mut Inlines, s: &str) {
    let target = stack.last_mut().map_or(root, |f| &mut f.content);
    if let Some(Inline::Span { text, marks }) = target.last_mut()
        && marks.is_plain()
    {
        text.push_str(s);
        return;
    }
    target.push(Inline::plain(s));
}

fn close(stack: &mut Vec<Frame>, root: &mut Inlines, attrs: Option<&Attrs>) {
    let Some(mut frame) = stack.pop() else { return };
    match (frame.kind, attrs) {
        (Kind::Bold, _) => apply(&mut frame.content, &|m| m.bold = true),
        (Kind::Italic, _) => apply(&mut frame.content, &|m| m.italic = true),
        (Kind::Strike, _) => apply(&mut frame.content, &|m| m.strike = true),
        (Kind::Bracket, Some(a)) => apply(&mut frame.content, &|m| {
            if a.small_caps {
                m.small_caps = true;
            }
            if m.lang.is_none() {
                m.lang.clone_from(&a.lang);
            }
            if m.link.is_none() {
                m.link.clone_from(&a.link);
            }
            if m.comment.is_none() {
                m.comment.clone_from(&a.comment);
            }
            if m.rev.is_none() {
                m.rev = a.rev;
            }
        }),
        (Kind::Bracket, None) => {}
    }
    let target = stack.last_mut().map_or(root, |f| &mut f.content);
    target.append(&mut frame.content);
}

fn open(stack: &mut Vec<Frame>, kind: Kind, opener: Opener) -> Result<(), Opener> {
    if stack.len() >= MAX_NESTING {
        return Err(opener);
    }
    stack.push(Frame { kind, opener, content: Vec::new() });
    Ok(())
}

fn build(toks: &[Tok], literal: &BTreeSet<Opener>) -> Result<Inlines, Opener> {
    let mut root: Inlines = Vec::new();
    let mut stack: Vec<Frame> = Vec::new();
    for (ti, tok) in toks.iter().enumerate() {
        match tok {
            Tok::Text(s) => push_text(&mut stack, &mut root, s),
            Tok::NoteRef(label) => {
                let target = stack.last_mut().map_or(&mut root, |f| &mut f.content);
                target.push(Inline::NoteRef { label: label.clone(), marks: Marks::default() });
            }
            Tok::Stars(n) => {
                let mut k = 0;
                while k < *n {
                    if literal.contains(&(ti, k)) {
                        push_text(&mut stack, &mut root, "*");
                        k += 1;
                        continue;
                    }
                    let top = stack.last().map(|f| f.kind);
                    if top == Some(Kind::Italic) {
                        close(&mut stack, &mut root, None);
                        k += 1;
                    } else if top == Some(Kind::Bold) && n - k >= 2 && !literal.contains(&(ti, k + 1)) {
                        close(&mut stack, &mut root, None);
                        k += 2;
                    } else if n - k >= 2 && top != Some(Kind::Bold) && !literal.contains(&(ti, k + 1)) {
                        open(&mut stack, Kind::Bold, (ti, k))?;
                        k += 2;
                    } else if stack.iter().any(|f| f.kind == Kind::Italic) {
                        // An italic is open below another marker: this star cannot close it.
                        return Err(stack.last().map_or((ti, k), |f| f.opener));
                    } else {
                        open(&mut stack, Kind::Italic, (ti, k))?;
                        k += 1;
                    }
                }
            }
            Tok::Tildes => {
                if literal.contains(&(ti, 0)) {
                    push_text(&mut stack, &mut root, "~~");
                } else if stack.last().map(|f| f.kind) == Some(Kind::Strike) {
                    close(&mut stack, &mut root, None);
                } else if stack.iter().any(|f| f.kind == Kind::Strike) {
                    return Err(stack.last().map_or((ti, 0), |f| f.opener));
                } else {
                    open(&mut stack, Kind::Strike, (ti, 0))?;
                }
            }
            Tok::Open => {
                if literal.contains(&(ti, 0)) {
                    push_text(&mut stack, &mut root, "[");
                } else {
                    open(&mut stack, Kind::Bracket, (ti, 0))?;
                }
            }
            Tok::Close(attrs, src) => {
                let top = stack.last().map(|f| f.kind);
                if literal.contains(&(ti, 0)) || !stack.iter().any(|f| f.kind == Kind::Bracket) {
                    push_text(&mut stack, &mut root, src);
                } else if top == Some(Kind::Bracket) {
                    close(&mut stack, &mut root, Some(attrs));
                } else {
                    // An emphasis opened inside the brackets was never closed.
                    return Err(stack.last().map_or((ti, 0), |f| f.opener));
                }
            }
        }
    }
    if let Some(f) = stack.last() {
        return Err(f.opener);
    }
    Ok(root)
}

/// The tokens' source text, every marker read as plain text.
fn all_literal(toks: &[Tok]) -> Inlines {
    let mut s = String::new();
    let mut out = Vec::new();
    for t in toks {
        match t {
            Tok::Text(x) => s.push_str(x),
            Tok::Stars(n) => s.push_str(&"*".repeat(*n)),
            Tok::Tildes => s.push_str("~~"),
            Tok::Open => s.push('['),
            Tok::Close(_, src) => s.push_str(src),
            Tok::NoteRef(l) => {
                if !s.is_empty() {
                    out.push(Inline::plain(std::mem::take(&mut s)));
                }
                out.push(Inline::NoteRef { label: l.clone(), marks: Marks::default() });
            }
        }
    }
    if !s.is_empty() {
        out.push(Inline::plain(s));
    }
    out
}

/// Read one paragraph's content (its lines joined with `\n`).
pub fn inlines(s: &str) -> Inlines {
    let toks = tokenize(s);
    let mut literal = BTreeSet::new();
    for _ in 0..MAX_RETRIES {
        match build(&toks, &literal) {
            Ok(mut c) => {
                wordsmith_manuscript_model::text::normalize_inlines(&mut c);
                return c;
            }
            Err(opener) => {
                if !literal.insert(opener) {
                    break;
                }
            }
        }
    }
    let mut c = all_literal(&toks);
    wordsmith_manuscript_model::text::normalize_inlines(&mut c);
    c
}
