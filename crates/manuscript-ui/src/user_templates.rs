// SPDX-License-Identifier: MIT OR Apache-2.0
//! Project templates the writer makes (BND-4): Save as Template keeps the open project's shape
//! (see `manuscript-model::templates::skeleton`: binder, lists, fields, entry types, targets,
//! guidance notes, front and back matter; not the manuscript's text) as a packed project on this
//! computer, beside the app's preferences. The New page offers them after the built-in ones;
//! each project made from one gets ids of its own.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use wordsmith_manuscript_model::{ItemId, Project, templates};

/// One template the writer made.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct UserTemplate {
    /// Its file name, without the extension.
    pub slug: String,
    /// Its name.
    pub name: String,
    /// What it is for.
    pub about: String,
}

/// The id the New page uses for a template the writer made.
pub fn id_of(slug: &str) -> String {
    format!("user:{slug}")
}

fn slug_of(name: &str) -> String {
    let s = crate::entry_types::key_of(name);
    if s.is_empty() { "template".into() } else { s.chars().take(60).collect() }
}

fn file(dir: &Path, slug: &str, ext: &str) -> PathBuf {
    dir.join(format!("{slug}.{ext}"))
}

/// The templates in `dir`, by name.
pub fn list(dir: &Path) -> Vec<UserTemplate> {
    let Ok(rd) = std::fs::read_dir(dir) else { return Vec::new() };
    let mut out: Vec<UserTemplate> = rd
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .filter_map(|p| {
            let slug = p.file_stem()?.to_string_lossy().into_owned();
            if !file(dir, &slug, "wsmz").exists() {
                return None;
            }
            let meta: UserTemplate = std::fs::read(&p).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
            Some(UserTemplate { slug, ..meta })
        })
        .collect();
    out.sort_by_key(|a| a.name.to_lowercase());
    out
}

fn write(dir: &Path, name: &str, bytes: &[u8]) -> Result<(), String> {
    wordsmith_manuscript_format::folder::Sink::write(&mut wordsmith_manuscript_format::folder::Disk::new(dir), name, bytes).map_err(|e| e.to_string())
}

/// Keep a template made from `p` under `name`. Returns its slug; a template of the same name is
/// replaced.
pub fn save(dir: &Path, name: &str, about: &str, p: &Project) -> Result<String, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("a template needs a name".into());
    }
    std::fs::create_dir_all(dir).map_err(|e| format!("the template could not be kept: {e}"))?;
    let slug = slug_of(name);
    let bytes = wordsmith_manuscript_format::pack::pack_project(&templates::skeleton(p)).map_err(|e| e.to_string())?;
    write(dir, &format!("{slug}.wsmz"), &bytes)?;
    let meta = UserTemplate { slug: slug.clone(), name: name.to_owned(), about: about.trim().to_owned() };
    write(dir, &format!("{slug}.json"), &serde_json::to_vec_pretty(&meta).map_err(|e| e.to_string())?)?;
    Ok(slug)
}

/// A new project from a template the writer made, with fresh ids and `title`.
pub fn build(dir: &Path, slug: &str, title: &str) -> Result<Project, String> {
    let bytes = std::fs::read(file(dir, slug, "wsmz")).map_err(|_| format!("there is no template `{slug}`"))?;
    let p = wordsmith_manuscript_format::pack::read_packed(&bytes).map_err(|e| e.to_string())?;
    let mut p = templates::with_fresh_ids(&p, &mut ItemId::new);
    p.settings.book.title = title.to_owned();
    Ok(p)
}

/// Delete a template the writer made.
pub fn delete(dir: &Path, slug: &str) -> Result<(), String> {
    for ext in ["wsmz", "json"] {
        let f = file(dir, slug, ext);
        if f.exists() {
            std::fs::remove_file(&f).map_err(|e| format!("{}: {e}", f.display()))?;
        }
    }
    Ok(())
}
