// SPDX-License-Identifier: MIT OR Apache-2.0
//! Export commands (SPEC 16): write the compiled manuscript to a file, checked word for word;
//! options saved as named presets in the project. Export never changes the project's text.

use std::path::Path;

use serde_json::{Value, json};
use wordsmith_manuscript_io::compile::Options;
use wordsmith_manuscript_io::export::{self, Target};

use crate::commands::{Spec, opt_str, str_param};
use crate::session::{Outcome, Session};

/// The export commands.
pub fn specs() -> Vec<Spec> {
    vec![
        Spec {
            id: "export.file",
            label: "Export",
            key: "Mod+E",
            params: r#"{"path": string, "format"?: "docx"|"odt"|"rtf"|"markdown"|"text"|"html"|"manuscript-docx"|"manuscript-pdf"|"book", "options"?: {...}, "preset"?: string}"#,
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
            if let Some(out) = crate::exports::special(s, &format, &path, &options) {
                return out;
            }
            let target = Target::from_extension(&format)
                .or_else(|| serde_json::from_value(json!(format)).ok())
                .ok_or_else(|| format!("`{format}` is not a format Manuscript exports"))?;
            let out = export::export(&s.project, &options, target)?;
            if !out.check.passed() {
                return Err(format!("the export was not written: its words do not match the manuscript ({})", out.check));
            }
            write(&path, &out.bytes)?;
            Ok(json!({ "path": path.to_string_lossy(), "bytes": out.bytes.len(), "check": out.check.to_string(), "left_out": out.left_out }))
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

/// Exports with their own writers (the standard manuscript layout, the book package): `None` for
/// formats handled by [`export::export`].
pub fn special(_s: &mut Session, _format: &str, _path: &Path, _o: &Options) -> Option<Outcome> {
    None
}
