// SPDX-License-Identifier: MIT OR Apache-2.0
//! Export through an external converter (EXP-7): for formats Manuscript does not write itself,
//! a document converter the writer installed as a separate program. Manuscript is never bundled
//! or linked with it: it writes the manuscript as DOCX, runs the program the writer named, with
//! the arguments the writer gave (`{input}`, `{output}` and `{format}` filled in), and reads the
//! result back to check every word when it is a format Manuscript reads.
//!
//! The program is run directly, never through a shell, so nothing in a path or title is
//! interpreted as a command; it is stopped after [`TIMEOUT`].

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use wordsmith_manuscript_model::Project;

use crate::compile::Options;
use crate::verify::Check;

/// How long a converter may run.
pub const TIMEOUT: Duration = Duration::from_secs(300);

/// A converter the writer installed.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Converter {
    /// The program (a path, or a name the system finds).
    pub program: String,
    /// Its arguments, separated by spaces; `{input}`, `{output}` and `{format}` are filled in. A
    /// placeholder is one argument even when the path holds spaces.
    pub args: String,
}

/// What a conversion made.
#[derive(Clone, Debug, PartialEq)]
pub struct Converted {
    /// The file written.
    pub path: PathBuf,
    /// Its size.
    pub bytes: u64,
    /// The word-for-word check, when the format can be read back.
    pub check: Option<Check>,
    /// What the converter said, cut short.
    pub said: String,
}

/// The arguments with the placeholders filled in.
pub fn arguments(template: &str, input: &Path, output: &Path, format: &str) -> Vec<String> {
    template
        .split_whitespace()
        .map(|a| a.replace("{input}", &input.to_string_lossy()).replace("{output}", &output.to_string_lossy()).replace("{format}", format))
        .collect()
}

/// Convert the manuscript to `output` (its extension names the format) with `converter`.
pub fn convert(p: &Project, o: &Options, converter: &Converter, output: &Path) -> Result<Converted, String> {
    if converter.program.trim().is_empty() {
        return Err("name the converter program first".into());
    }
    if !converter.args.contains("{input}") || !converter.args.contains("{output}") {
        return Err("the converter's arguments must name {input} and {output}".into());
    }
    let format = output.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
    if format.is_empty() {
        return Err("give the file an extension: it names the format".into());
    }
    // The manuscript as DOCX, checked first.
    let out = crate::export::export(p, o, crate::export::Target::Docx)?;
    if !out.check.passed() {
        return Err(format!("the manuscript could not be written for the converter ({})", out.check));
    }
    let dir = std::env::temp_dir().join(format!("wordsmith-manuscript-convert-{}", std::process::id()));
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let input = dir.join("manuscript.docx");
    std::fs::write(&input, &out.bytes).map_err(|e| e.to_string())?;
    let _ = std::fs::remove_file(output);
    let mut child = Command::new(converter.program.trim())
        .args(arguments(&converter.args, &input, output, &format))
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("the converter could not be started ({}): {e}", converter.program))?;
    let started = Instant::now();
    let status = loop {
        if let Some(s) = child.try_wait().map_err(|e| e.to_string())? {
            break s;
        }
        if started.elapsed() > TIMEOUT {
            let _ = child.kill();
            let _ = std::fs::remove_dir_all(&dir);
            return Err(format!("the converter did not finish within {} seconds and was stopped", TIMEOUT.as_secs()));
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    let output_text = child.wait_with_output().map(|o| String::from_utf8_lossy(&o.stderr).chars().take(400).collect::<String>()).unwrap_or_default();
    let _ = std::fs::remove_dir_all(&dir);
    if !status.success() {
        return Err(format!("the converter failed ({status}): {output_text}"));
    }
    let meta = std::fs::metadata(output).map_err(|_| "the converter finished but wrote no file".to_owned())?;
    if meta.len() == 0 {
        return Err("the converter wrote an empty file".into());
    }
    // Read back what Manuscript can read.
    let check = jcj_interchange::Format::from_name(&output.to_string_lossy())
        .filter(|f| f.can_import() || *f == jcj_interchange::Format::Epub)
        .and_then(|f| {
            let bytes = std::fs::read(output).ok()?;
            let found = crate::export::read_back(f, &bytes).ok()?;
            let mut expected = crate::compile::compile(p, o).words();
            expected.extend(crate::compile::compile(p, o).note_words());
            let (mut a, mut b) = (expected, found);
            a.sort();
            b.sort();
            Some(Check::of(&a, &b))
        });
    Ok(Converted { path: output.to_path_buf(), bytes: meta.len(), check, said: output_text })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholders_fill_whole_arguments() {
        let a = arguments("-f docx -t {format} -o {output} {input}", Path::new("/t/in put.docx"), Path::new("/w/My Book.odt"), "odt");
        assert_eq!(a, ["-f", "docx", "-t", "odt", "-o", "/w/My Book.odt", "/t/in put.docx"]);
    }
}
