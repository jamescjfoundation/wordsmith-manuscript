// SPDX-License-Identifier: MIT OR Apache-2.0
//! OPML import (IMP-8): an outline from an outliner or another writing tool, into the binder.
//!
//! Each `<outline>` is an item titled by its `text` (or `title`) attribute; its `_note` attribute
//! becomes the item's synopsis. An outline with children is a folder; a leaf is a document. A
//! title that names its role ("Part Two", "Book I", "Volume 3", "Act One", "Chapter 12") decides
//! it; otherwise the outline's shape does: leaves are scenes, their parents chapters, anything
//! above parts. Every word of every title and note is kept, checked word for word.
//!
//! The reader is bounded and careful: the file's size, the number of outlines and the depth are
//! capped, entities are decoded (named and numeric), and anything that is not an outline is
//! skipped. Damage is an error, never a panic.

use wordsmith_manuscript_model::{ItemMeta, ItemText, Kind, Role, Text};

use crate::mapping::{Plan, Planned, Report};
use crate::verify;

/// The most outlines read.
pub const MAX_OUTLINES: usize = 100_000;
/// The deepest outline read.
pub const MAX_DEPTH: usize = 32;

#[derive(Debug, Default)]
struct Node {
    title: String,
    note: String,
    children: Vec<Node>,
}

/// Whether bytes look like an OPML file.
pub fn is_opml(bytes: &[u8]) -> bool {
    let head = String::from_utf8_lossy(&bytes[..bytes.len().min(512)]).to_lowercase();
    head.contains("<opml")
}

/// Decode XML character references and the five named entities.
pub fn unescape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(at) = rest.find('&') {
        out.push_str(&rest[..at]);
        let after = &rest[at..];
        let Some(end) = after.find(';').filter(|e| *e <= 12) else {
            out.push('&');
            rest = &after[1..];
            continue;
        };
        let name = &after[1..end];
        let c = match name {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            n if n.starts_with("#x") || n.starts_with("#X") => u32::from_str_radix(&n[2..], 16).ok().and_then(char::from_u32),
            n if n.starts_with('#') => n[1..].parse().ok().and_then(char::from_u32),
            _ => None,
        };
        match c {
            Some(c) => {
                out.push(c);
                rest = &after[end + 1..];
            }
            None => {
                out.push('&');
                rest = &after[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// The value of attribute `name` in a tag's text.
fn attr(tag: &str, name: &str) -> Option<String> {
    let mut rest = tag;
    while let Some(at) = rest.find(name) {
        let before_ok = at == 0 || rest[..at].ends_with(|c: char| c.is_whitespace());
        let after = rest[at + name.len()..].trim_start();
        if before_ok && let Some(v) = after.strip_prefix('=') {
            let v = v.trim_start();
            let q = v.chars().next()?;
            if q == '"' || q == '\'' {
                let body = &v[1..];
                return body.find(q).map(|end| unescape(&body[..end]));
            }
        }
        rest = &rest[at + name.len()..];
    }
    None
}

fn parse(xml: &str) -> Result<Vec<Node>, String> {
    let body = xml.find("<body").map(|i| &xml[i..]).ok_or("the file has no outline body")?;
    let mut stack: Vec<Node> = vec![Node::default()];
    let mut count = 0usize;
    let mut rest = body;
    while let Some(at) = rest.find('<') {
        let after = &rest[at + 1..];
        // Comments, declarations and character data blocks are skipped whole.
        if let Some(c) = after.strip_prefix("!--") {
            let end = c.find("-->").ok_or("a comment is not closed")?;
            rest = &c[end + 3..];
            continue;
        }
        let end = find_tag_end(after).ok_or("a tag is not closed")?;
        let tag = &after[..end];
        rest = &after[end + 1..];
        if let Some(name) = tag.strip_prefix('/') {
            if name.trim() == "outline" {
                let done = stack.pop().ok_or("the outline is damaged")?;
                stack.last_mut().ok_or("the outline is damaged")?.children.push(done);
            } else if name.trim() == "body" {
                break;
            }
            continue;
        }
        let is_outline = tag.starts_with("outline") && tag[7..].starts_with(|c: char| c.is_whitespace() || c == '/' || c == '>') || tag == "outline";
        if !is_outline {
            continue;
        }
        count += 1;
        if count > MAX_OUTLINES {
            return Err(format!("the outline has more than {MAX_OUTLINES} items"));
        }
        let node = Node {
            title: attr(tag, "text").or_else(|| attr(tag, "title")).unwrap_or_default(),
            note: attr(tag, "_note").unwrap_or_default(),
            children: Vec::new(),
        };
        if tag.trim_end().ends_with('/') {
            stack.last_mut().ok_or("the outline is damaged")?.children.push(node);
        } else {
            if stack.len() > MAX_DEPTH {
                return Err(format!("the outline is deeper than {MAX_DEPTH} levels"));
            }
            stack.push(node);
        }
    }
    // Outlines left open at the end are closed (a cut-off file keeps what it has).
    while stack.len() > 1 {
        let done = stack.pop().ok_or("the outline is damaged")?;
        stack.last_mut().ok_or("the outline is damaged")?.children.push(done);
    }
    Ok(stack.pop().map(|n| n.children).unwrap_or_default())
}

/// The end of a tag, past quoted attribute values.
fn find_tag_end(s: &str) -> Option<usize> {
    let mut quote: Option<char> = None;
    for (i, c) in s.char_indices() {
        match (quote, c) {
            (Some(q), c) if c == q => quote = None,
            (Some(_), _) => {}
            (None, '"' | '\'') => quote = Some(c),
            (None, '>') => return Some(i),
            _ => {}
        }
    }
    None
}

fn height(n: &Node) -> usize {
    n.children.iter().map(|c| height(c) + 1).max().unwrap_or(0)
}

/// The role a title names: "Part Two", "Book I", "Volume 3", "Act One" are parts, "Chapter 12"
/// a chapter.
fn named_role(title: &str) -> Option<Role> {
    let t = title.trim().to_lowercase();
    let first = t.split(|c: char| c.is_whitespace() || c == '.' || c == ':').next().unwrap_or("");
    let numbered = t.len() > first.len();
    match first {
        "part" | "book" | "volume" | "act" if numbered => Some(Role::Part),
        "chapter" if numbered => Some(Role::Chapter),
        _ => None,
    }
}

fn flatten(n: Node, depth: usize, out: &mut Vec<Planned>) {
    let h = height(&n);
    let kind = if n.children.is_empty() { Kind::Text } else { Kind::Folder };
    let role = named_role(&n.title).unwrap_or(match h {
        0 => Role::Scene,
        1 => Role::Chapter,
        _ => Role::Part,
    });
    let title = n.title.trim().to_owned();
    let from_source = !title.is_empty();
    let meta = ItemMeta { synopsis: n.note.replace("\r\n", "\n"), ..ItemMeta::default() };
    out.push(Planned {
        title: if from_source { title } else { "Untitled".into() },
        title_from_source: from_source,
        role,
        kind,
        depth,
        text: ItemText { meta, body: Text::new() },
    });
    for c in n.children {
        flatten(c, depth + 1, out);
    }
}

fn words_of(nodes: &[Node], out: &mut Vec<String>) {
    for n in nodes {
        out.extend(verify::words(&n.title));
        out.extend(verify::words(&n.note));
        words_of(&n.children, out);
    }
}

/// An OPML file's outline as a binder plan, checked word for word.
pub fn plan(bytes: &[u8]) -> Result<Plan, String> {
    let xml = std::str::from_utf8(bytes).map_err(|_| "the outline is not UTF-8 text".to_owned())?;
    let nodes = parse(xml)?;
    if nodes.is_empty() {
        return Err("the outline is empty".into());
    }
    let mut source = Vec::new();
    words_of(&nodes, &mut source);
    let mut items = Vec::new();
    for n in nodes {
        flatten(n, 0, &mut items);
    }
    let mut planned = Vec::new();
    for it in &items {
        if it.title_from_source {
            planned.extend(verify::words(&it.title));
        }
        planned.extend(verify::words(&it.text.meta.synopsis));
    }
    let mut report = Report::default();
    for it in &items {
        *report.made.entry(it.role.title().to_owned()).or_default() += 1;
    }
    report.check = Some(verify::Check::of(&source, &planned));
    Ok(Plan { items, report })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<opml version="2.0"><head><title>Plan</title></head>
<body>
  <outline text="Act One">
    <outline text="The Harbour" _note="Mara arrives &amp; finds the lamp out.">
      <outline text="Arrival" _note="Rain. The ferry is late."/>
      <outline text="The Keeper&#39;s House"/>
    </outline>
  </outline>
  <!-- a comment <outline text="not this"/> -->
  <outline text="Loose idea"/>
</body></opml>"#;

    #[test]
    fn outlines_become_items_with_synopses() {
        let p = plan(SAMPLE.as_bytes()).unwrap();
        let got: Vec<(usize, &str, Role, Kind)> = p.items.iter().map(|i| (i.depth, i.title.as_str(), i.role, i.kind)).collect();
        assert_eq!(
            got,
            vec![
                (0, "Act One", Role::Part, Kind::Folder),
                (1, "The Harbour", Role::Chapter, Kind::Folder),
                (2, "Arrival", Role::Scene, Kind::Text),
                (2, "The Keeper's House", Role::Scene, Kind::Text),
                (0, "Loose idea", Role::Scene, Kind::Text),
            ]
        );
        assert_eq!(p.items[1].text.meta.synopsis, "Mara arrives & finds the lamp out.");
        assert!(p.report.check.as_ref().is_some_and(crate::verify::Check::passed));
    }

    #[test]
    fn damaged_outlines_are_errors_or_kept_as_far_as_they_go() {
        assert!(plan(b"<opml><body></body></opml>").is_err());
        assert!(plan(b"not xml at all").is_err());
        let cut = plan(b"<opml><body><outline text=\"A\"><outline text=\"B\"/>").unwrap();
        assert_eq!(cut.items.len(), 2);
        assert_eq!(unescape("a &lt;b&gt; &#x41;&#66; & c &bogus;"), "a <b> AB & c &bogus;");
    }
}
