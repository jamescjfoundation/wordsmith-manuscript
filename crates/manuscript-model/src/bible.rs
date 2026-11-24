// SPDX-License-Identifier: MIT OR Apache-2.0
//! The story bible (BIB-1, BIB-2): Manuscript's entry types and their default templates, on the
//! general entries layer ([`crate::entries`]).

use crate::entries::{FieldType, Template};
use crate::{Project, Settings};

use FieldType::{LongText, Text};

/// The default templates (SPEC 9.2), in the order shown.
pub fn default_templates() -> Vec<Template> {
    vec![
        Template::new(
            "character",
            "Character",
            "Characters",
            &[
                ("Role in story", Text),
                ("Age", Text),
                ("Appearance", LongText),
                ("Personality", LongText),
                ("Wants", LongText),
                ("Needs", LongText),
                ("Flaw", LongText),
                ("History", LongText),
            ],
        ),
        Template::new(
            "place",
            "Place",
            "Places",
            &[("Kind", Text), ("Location", Text), ("Description", LongText), ("Sensory details", LongText), ("History", LongText)],
        ),
        Template::new("object", "Object", "Objects", &[("Kind", Text), ("Owner", Text), ("Description", LongText), ("Significance", LongText)]),
        Template::new("group", "Group", "Groups", &[("Kind", Text), ("Members", FieldType::List), ("Purpose", LongText), ("History", LongText)]),
        Template::new("lore", "Lore", "Lore", &[("Kind", Text), ("Rules", LongText), ("History", LongText)]),
        Template::new(
            "thread",
            "Plot Thread",
            "Plot Threads",
            &[("Kind", Text), ("Question", LongText), ("Stakes", LongText), ("Resolution", LongText)],
        ),
    ]
}

/// The project's templates: the defaults, then any the writer made (kept in settings under
/// `templates`; made by hand from 0.2).
pub fn templates(settings: &Settings) -> Vec<Template> {
    let mut out = default_templates();
    if let Some(custom) = settings.extra.get("templates").and_then(|v| serde_json::from_value::<Vec<Template>>(v.clone()).ok()) {
        for t in custom {
            match out.iter_mut().find(|x| x.kind == t.kind) {
                Some(x) => *x = t,
                None => out.push(t),
            }
        }
    }
    out
}

/// The template of an entry type, if the project has it.
pub fn template(p: &Project, kind: &str) -> Option<Template> {
    templates(&p.settings).into_iter().find(|t| t.kind == kind)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_six_types_have_their_fields() {
        let t = default_templates();
        let kinds: Vec<&str> = t.iter().map(|x| x.kind.as_str()).collect();
        assert_eq!(kinds, ["character", "place", "object", "group", "lore", "thread"]);
        assert_eq!(t[0].fields.len(), 8);
        let plurals: Vec<&str> = t.iter().map(|x| x.plural.as_str()).collect();
        assert_eq!(plurals, crate::templates::BIBLE_FOLDERS);
    }

    #[test]
    fn a_project_template_replaces_a_default_or_adds_a_type() {
        let mut s = Settings::default();
        let custom = vec![
            Template::new("character", "Character", "Characters", &[("Pronouns", Text)]),
            Template::new("ship", "Ship", "Ships", &[("Crew", Text)]),
        ];
        s.extra.insert("templates".into(), serde_json::to_value(custom).unwrap_or_default());
        let t = templates(&s);
        assert_eq!(t.len(), 7);
        assert_eq!(t[0].fields.len(), 1);
        assert_eq!(t[6].kind, "ship");
    }
}
