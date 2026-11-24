// SPDX-License-Identifier: MIT OR Apache-2.0
//! Screenplays inside a project (SCR-1, plan section 5.7): reading a Wordsmith Script file
//! (`.wss`) for showing it, read-only, and counting its words.
//!
//! Manuscript has no screenplay rules and never writes a script: Wordsmith Script edits the file
//! in place. This reader, Manuscript's own, takes what it needs from Script's published format
//! (its `docs/format.md`): the title from the settings, then each paragraph's element and text,
//! with the text's marks (bold, italic, underline toggles, `{…}` marks, `\` escapes) taken off.
//! It is bounded, and a damaged file is an error, never a panic.

/// The largest script read.
pub const MAX_FILE: usize = 64 << 20;
/// The most paragraphs read.
pub const MAX_PARAGRAPHS: usize = 500_000;

/// One paragraph of a screenplay: its element (`SceneHeading`, `Action`, `Character`,
/// `Dialogue`, `Parenthetical`, `Transition`, or another the script defines) and its plain text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScriptParagraph {
    /// The element's id.
    pub element: String,
    /// The text without marks; manual line breaks as `\n`.
    pub text: String,
}

/// A screenplay as Manuscript shows it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Screenplay {
    /// The format version the file declares.
    pub version: u32,
    /// Its title, if the settings give one.
    pub title: String,
    /// The paragraphs in order.
    pub paragraphs: Vec<ScriptParagraph>,
}

impl Screenplay {
    /// Its words, counted as everywhere in Manuscript.
    pub fn words(&self) -> usize {
        self.paragraphs.iter().map(|p| crate::verify::words(&p.text).len()).sum()
    }

    /// How many scenes (scene headings) it has.
    pub fn scenes(&self) -> usize {
        self.paragraphs.iter().filter(|p| p.element == "SceneHeading").count()
    }
}

/// Whether bytes look like a Wordsmith Script file.
pub fn is_script(bytes: &[u8]) -> bool {
    bytes.starts_with(b"Wordsmith Script ")
}

/// The text of one line with its marks taken off.
fn plain(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                if let Some(n) = chars.next() {
                    out.push(n);
                }
            }
            '*' | '_' => {}
            '{' => {
                // A mark ({n12}, {/n}, {r2}, {p7}, {sD,K}, ...): skipped to its end.
                for n in chars.by_ref() {
                    if n == '}' {
                        break;
                    }
                }
            }
            c => out.push(c),
        }
    }
    out
}

/// Read a script file.
pub fn read(bytes: &[u8]) -> Result<Screenplay, String> {
    if bytes.len() > MAX_FILE {
        return Err(format!("the script is larger than {} MB", MAX_FILE >> 20));
    }
    let text = std::str::from_utf8(bytes).map_err(|_| "the script is not UTF-8 text".to_owned())?;
    let mut lines = text.split('\n').map(|l| l.strip_suffix('\r').unwrap_or(l));
    let first = lines.next().unwrap_or("");
    let version: u32 = first.strip_prefix("Wordsmith Script ").and_then(|v| v.trim().parse().ok()).ok_or("this is not a Wordsmith Script file")?;
    let mut out = Screenplay { version, ..Screenplay::default() };
    // Settings: one JSON object between the two markers.
    let mut settings = String::new();
    let mut in_script = false;
    for line in lines.by_ref() {
        if line == "=== script ===" {
            in_script = true;
            break;
        }
        if line != "=== settings ===" {
            settings.push_str(line);
            settings.push('\n');
        }
    }
    if !in_script {
        return Err("the script has no script part".into());
    }
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&settings) {
        out.title = v.pointer("/core/title").and_then(serde_json::Value::as_str).unwrap_or("").to_owned();
    }
    let mut current: Option<ScriptParagraph> = None;
    for line in lines {
        match &mut current {
            None => {
                if let Some(head) = line.strip_prefix('@') {
                    let element = head.split_whitespace().next().unwrap_or("General").to_owned();
                    current = Some(ScriptParagraph { element, text: String::new() });
                }
            }
            Some(p) => {
                if line.is_empty() {
                    if out.paragraphs.len() >= MAX_PARAGRAPHS {
                        return Err(format!("the script has more than {MAX_PARAGRAPHS} paragraphs"));
                    }
                    if let Some(done) = current.take() {
                        out.paragraphs.push(done);
                    }
                } else {
                    if !p.text.is_empty() {
                        p.text.push('\n');
                    }
                    if line != "\\" {
                        p.text.push_str(&plain(line));
                    }
                }
            }
        }
    }
    if let Some(done) = current {
        out.paragraphs.push(done);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "Wordsmith Script 5\n=== settings ===\n{\n  \"core\": { \"title\": \"Harbour Lights\" }\n}\n=== script ===\n@SceneHeading #1\nEXT. HARBOUR - NIGHT\n\n@Action\nRain *hammers* the {n3}pier{/n}. A \\*star\\* shines.\n\n@Character\nMARA\n\n@Dialogue\nWe go **now**.\nBefore the tide.\n\n@Transition\nCUT TO:\n\n";

    #[test]
    fn a_script_reads_as_elements_and_plain_text() {
        let s = read(SAMPLE.as_bytes()).unwrap();
        assert_eq!(s.version, 5);
        assert_eq!(s.title, "Harbour Lights");
        let got: Vec<(&str, &str)> = s.paragraphs.iter().map(|p| (p.element.as_str(), p.text.as_str())).collect();
        assert_eq!(
            got,
            [
                ("SceneHeading", "EXT. HARBOUR - NIGHT"),
                ("Action", "Rain hammers the pier. A *star* shines."),
                ("Character", "MARA"),
                ("Dialogue", "We go now.\nBefore the tide."),
                ("Transition", "CUT TO:"),
            ]
        );
        assert_eq!(s.scenes(), 1);
        assert_eq!(s.words(), 19);
    }

    #[test]
    fn other_files_are_refused() {
        assert!(read(b"not a script").is_err());
        assert!(read(b"Wordsmith Script 5\n=== settings ===\n{}\n").is_err());
        assert!(!is_script(b"# Markdown"));
    }
}
