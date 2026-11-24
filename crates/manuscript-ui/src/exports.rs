// SPDX-License-Identifier: MIT OR Apache-2.0
//! Export commands (SPEC 16): write the compiled manuscript to a file, checked word for word;
//! options saved as named presets in the project. Export never changes the project's text.

use std::path::Path;

use serde_json::{Value, json};
use wordsmith_manuscript_io::compile::Options;
use wordsmith_manuscript_io::export::{self, Target};
use wordsmith_manuscript_io::standard;

use crate::commands::{Spec, opt_str, str_param};
use crate::session::{Outcome, Session};

/// The export commands.
pub fn specs() -> Vec<Spec> {
    vec![
        Spec {
            id: "export.convert",
            label: "Export Through a Converter",
            key: "",
            params: r#"{"path": string, "program": string, "args": string, "options"?: {...}, "preset"?: string}"#,
            about: "Write the manuscript in another format through a document converter installed separately: it is given the manuscript as DOCX ({input}) and writes `path` ({output}; {format} is its extension). Checked word for word when the result can be read back.",
            mutates: false,
        },
        Spec {
            id: "export.file",
            label: "Export",
            key: "Mod+E",
            params: r#"{"path": string, "format"?: "docx"|"odt"|"rtf"|"markdown"|"text"|"html"|"epub"|"manuscript-docx"|"manuscript-pdf"|"book", "options"?: {...}, "preset"?: string, "standard"?: {"paper"?: "letter"|"a4", "face"?: "serif"|"mono", "title_page"?: bool, "underline_italic"?: bool}}"#,
            about: "Write the manuscript to a file (format from the extension when not given), checked word for word.",
            mutates: false,
        },
        Spec { id: "export.presets", label: "Export Presets", key: "", params: "{}", about: "The project's saved export presets.", mutates: false },
        Spec {
            id: "export.savePreset",
            label: "Save Export Preset",
            key: "",
            params: r#"{"name": string, "options": {...}}"#,
            about: "Keep export options under a name in the project.",
            mutates: true,
        },
    ]
}

/// The options for an export: given ones, else a named preset, else the project's last ones, else
/// the defaults.
pub fn options_of(s: &Session, v: &Value) -> Result<Options, String> {
    if let Some(o) = v.get("options") {
        return serde_json::from_value(o.clone()).map_err(|e| format!("options: {e}"));
    }
    let presets = s.project.settings.extra.get("export_presets");
    if let Some(name) = opt_str(v, "preset") {
        let p = presets.and_then(|p| p.get(name)).ok_or_else(|| format!("no export preset `{name}`"))?;
        return serde_json::from_value(p.clone()).map_err(|e| format!("preset {name}: {e}"));
    }
    Ok(Options::default())
}

fn write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let dir = path.parent().filter(|d| !d.as_os_str().is_empty()).unwrap_or(Path::new("."));
    let name = path.file_name().ok_or("the path needs a file name")?.to_string_lossy().into_owned();
    wordsmith_manuscript_format::folder::Sink::write(&mut wordsmith_manuscript_format::folder::Disk::new(dir), &name, bytes)
        .map_err(|e| e.to_string())
}

/// Run an export command, or `None` if `id` is not one.
pub fn run(s: &mut Session, id: &str, v: &Value) -> Option<Outcome> {
    Some(match id {
        "export.file" => (|| {
            let path = Path::new(str_param(v, "path")?).to_path_buf();
            let options = options_of(s, v)?;
            let format = opt_str(v, "format")
                .map(str::to_owned)
                .unwrap_or_else(|| path.extension().map(|e| e.to_string_lossy().into_owned()).unwrap_or_default());
            if let Some(out) = special(s, &format, &path, &options, v) {
                return out;
            }
            let target = Target::from_extension(&format)
                .or_else(|| serde_json::from_value(json!(format)).ok())
                .ok_or_else(|| format!("`{format}` is not a format Manuscript exports"))?;
            // The draft chosen for export (DRF-1).
            let out = export::export(&wordsmith_manuscript_model::drafts::exported(&s.project), &options, target)?;
            if !out.check.passed() {
                return Err(format!("the export was not written: its words do not match the manuscript ({})", out.check));
            }
            write(&path, &out.bytes)?;
            Ok(json!({ "path": path.to_string_lossy(), "bytes": out.bytes.len(), "check": out.check.to_string(), "left_out": out.left_out }))
        })(),
        "export.convert" => (|| {
            let path = Path::new(str_param(v, "path")?).to_path_buf();
            let options = options_of(s, v)?;
            let conv =
                wordsmith_manuscript_io::external::Converter { program: str_param(v, "program")?.to_owned(), args: str_param(v, "args")?.to_owned() };
            let out = wordsmith_manuscript_io::external::convert(&wordsmith_manuscript_model::drafts::exported(&s.project), &options, &conv, &path)?;
            if let Some(c) = &out.check
                && !c.passed()
            {
                return Err(format!("the converter's file does not hold the manuscript's words ({c}); it was written to {}", path.display()));
            }
            Ok(json!({
                "path": out.path.to_string_lossy(),
                "bytes": out.bytes,
                "check": out.check.as_ref().map_or_else(|| "not checked: Manuscript cannot read this format back".to_owned(), ToString::to_string),
            }))
        })(),
        "export.presets" => Ok(s.project.settings.extra.get("export_presets").cloned().unwrap_or_else(|| json!({}))),
        "export.savePreset" => (|| {
            let name = str_param(v, "name")?.trim().to_owned();
            if name.is_empty() {
                return Err("a preset needs a name".into());
            }
            let o: Options =
                serde_json::from_value(v.get("options").cloned().ok_or("`options` is required")?).map_err(|e| format!("options: {e}"))?;
            let value = serde_json::to_value(o).map_err(|e| e.to_string())?;
            s.change("Save Export Preset", &[], &[], |p| {
                let presets = p.settings.extra.entry("export_presets".to_owned()).or_insert_with(|| json!({}));
                presets[name.as_str()] = value;
                Ok(())
            })?;
            Ok(json!({}))
        })(),
        _ => return None,
    })
}

/// The standard manuscript settings: given (`standard`), else the project's, else the defaults.
pub(crate) fn standard_of(s: &Session, v: &Value) -> Result<standard::Standard, String> {
    match v.get("standard").or_else(|| s.project.settings.extra.get("manuscript_format")) {
        Some(x) => serde_json::from_value(x.clone()).map_err(|e| format!("standard manuscript settings: {e}")),
        None => Ok(standard::Standard::default()),
    }
}

/// Exports with their own writers (the standard manuscript layout, the book package): `None` for
/// formats handled by [`export::export`].
pub fn special(s: &mut Session, format: &str, path: &Path, o: &Options, v: &Value) -> Option<Outcome> {
    let run = |s: &mut Session| -> Outcome {
        let std = standard_of(s, v)?;
        let out = match format {
            "manuscript-pdf" | "pdf" => standard::pdf(&wordsmith_manuscript_model::drafts::exported(&s.project), o, &std)?,
            _ => standard::docx(&wordsmith_manuscript_model::drafts::exported(&s.project), o, &std)?,
        };
        if !out.check.passed() {
            return Err(format!("the export was not written: its words do not match the manuscript ({})", out.check));
        }
        write(path, &out.bytes)?;
        Ok(json!({ "path": path.to_string_lossy(), "bytes": out.bytes.len(), "pages": out.pages, "check": out.check.to_string() }))
    };
    match format {
        "manuscript-docx" | "manuscript-pdf" | "pdf" => Some(run(s)),
        "book" | "wsmz" => Some(book_package(s, path)),
        _ => None,
    }
}

/// The book package (EXP-5): the project packed as `.wsmz` for Wordsmith Book, checked by reading
/// it back through the book view; the metadata Book needs and the writer has not set is listed.
fn book_package(s: &mut Session, path: &Path) -> Outcome {
    use wordsmith_manuscript_format::book;
    let bytes = wordsmith_manuscript_format::pack::pack_current(&s.project, s.root()).map_err(|e| e.to_string())?;
    let mine = book::from_project(&s.project);
    let theirs = book::open_packed(&bytes).map_err(|e| e.to_string())?;
    let check = wordsmith_manuscript_io::verify::Check::of(&book::words(&mine), &book::words(&theirs));
    if !check.passed() || book::outline(&mine) != book::outline(&theirs) {
        return Err(format!("the book package was not written: it does not read back as the project ({check})"));
    }
    let mut path = path.to_path_buf();
    if path.extension().is_none_or(|e| e != "wsmz") {
        path.set_extension("wsmz");
    }
    write(&path, &bytes)?;
    Ok(json!({
        "path": path.to_string_lossy(),
        "bytes": bytes.len(),
        "check": check.to_string(),
        "book_view": book::VERSION,
        "sections": book::counts(&theirs),
        "missing_metadata": book::missing_metadata(&theirs),
    }))
}
