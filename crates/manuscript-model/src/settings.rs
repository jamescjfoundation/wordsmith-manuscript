// SPDX-License-Identifier: MIT OR Apache-2.0
//! Project settings (`project.toml`): the book's metadata, label, status and keyword lists,
//! targets, custom fields, export presets and document-set rules.

use serde::{Deserialize, Serialize};

fn is_default<T: Default + PartialEq>(v: &T) -> bool {
    *v == T::default()
}

/// The book's metadata: what the title page, headers and the book view use.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct BookInfo {
    /// The title.
    pub title: String,
    /// A subtitle.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub subtitle: String,
    /// The author's legal name (title page contact block).
    #[serde(skip_serializing_if = "String::is_empty")]
    pub author: String,
    /// The name the book is published under, when not the author's.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub pen_name: String,
    /// The surname for the manuscript header, when not the author's last word.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub surname: String,
    /// A short title for the manuscript header, when not the title.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub short_title: String,
    /// The title page's contact lines (address, phone, email), as the writer entered them.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub contact: Vec<String>,
    /// The text language (BCP 47).
    pub language: String,
    /// The series name.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub series: String,
    /// The number in the series.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub series_number: Option<u32>,
    /// ISBNs by edition name (`print`, `ebook`).
    #[serde(skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub isbn: std::collections::BTreeMap<String, String>,
    /// The publisher.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub publisher: String,
    /// The copyright year.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub copyright_year: Option<u32>,
    /// The rights statement.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub rights: String,
    /// A description (blurb).
    #[serde(skip_serializing_if = "String::is_empty")]
    pub description: String,
    /// Subject categories.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub subjects: Vec<String>,
    /// Search keywords for stores.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub keywords: Vec<String>,
}

impl Default for BookInfo {
    fn default() -> Self {
        BookInfo {
            title: "Untitled".into(),
            subtitle: String::new(),
            author: String::new(),
            pen_name: String::new(),
            surname: String::new(),
            short_title: String::new(),
            contact: Vec::new(),
            language: "en".into(),
            series: String::new(),
            series_number: None,
            isbn: Default::default(),
            publisher: String::new(),
            copyright_year: None,
            rights: String::new(),
            description: String::new(),
            subjects: Vec::new(),
            keywords: Vec::new(),
        }
    }
}

impl BookInfo {
    /// The name the book is published under: the pen name, else the author.
    pub fn byline(&self) -> &str {
        if self.pen_name.is_empty() { &self.author } else { &self.pen_name }
    }

    /// The surname for the manuscript header: the set surname, else the last word of the byline.
    pub fn header_surname(&self) -> String {
        if !self.surname.is_empty() {
            return self.surname.clone();
        }
        self.byline().split_whitespace().last().unwrap_or("").to_owned()
    }

    /// The short title for the manuscript header.
    pub fn header_title(&self) -> String {
        if self.short_title.is_empty() { self.title.clone() } else { self.short_title.clone() }
    }
}

/// A named color (labels and keywords).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Named {
    /// The name.
    pub name: String,
    /// `#RRGGBB`, or empty for none.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub color: String,
}

impl Named {
    /// A name with a color.
    pub fn new(name: &str, color: &str) -> Named {
        Named { name: name.into(), color: color.into() }
    }
}

/// The days of the week, for writing days.
pub const DAYS: [&str; 7] = ["mon", "tue", "wed", "thu", "fri", "sat", "sun"];

/// Word targets.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Targets {
    /// The manuscript's word target.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manuscript: Option<u64>,
    /// The deadline, `YYYY-MM-DD`.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub deadline: String,
    /// Writing days (`mon` .. `sun`); empty means every day.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub writing_days: Vec<String>,
    /// The session's word target.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session: Option<u64>,
}

/// The kind of a custom field.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FieldKind {
    /// Free text.
    #[default]
    Text,
    /// A date (`YYYY-MM-DD`).
    Date,
    /// Yes or no.
    Checkbox,
    /// One of a list of choices.
    List,
}

/// A custom field shown in the inspector and the outline.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FieldDef {
    /// The field's name (also its key in item metadata).
    pub name: String,
    /// Its kind.
    #[serde(default, skip_serializing_if = "is_default")]
    pub kind: FieldKind,
    /// For a list: the choices.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub choices: Vec<String>,
}

/// The name of the point-of-view field, built in at 0.1 (SPEC 7.3).
pub const POV_FIELD: &str = "Point of view";

/// Project settings.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// The book's metadata.
    pub book: BookInfo,
    /// Labels, in menu order.
    pub labels: Vec<Named>,
    /// Statuses, in order from not started to done.
    pub statuses: Vec<String>,
    /// Keywords.
    pub keywords: Vec<Named>,
    /// Targets.
    pub targets: Targets,
    /// Custom fields.
    pub fields: Vec<FieldDef>,
    /// The working draft's name (empty: "First Draft") (DRF-1).
    #[serde(skip_serializing_if = "String::is_empty")]
    pub draft_name: String,
    /// The stored drafts, oldest first (their manuscripts are in `drafts/<id>/`).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub drafts: Vec<crate::drafts::DraftInfo>,
    /// The series bible this book shares, as a path from the book's folder (BND-8; empty: the
    /// book keeps its own bible).
    #[serde(skip_serializing_if = "String::is_empty")]
    pub series: String,
    /// The draft that exports (empty: the working draft).
    #[serde(skip_serializing_if = "String::is_empty")]
    pub export_draft: String,
    /// Words the spelling check accepts in this project, besides bible names.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub words: Vec<String>,
    /// Export presets and import rules, kept as the io layer writes them (JSON-shaped values).
    #[serde(skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub extra: std::collections::BTreeMap<String, serde_json::Value>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            book: BookInfo::default(),
            labels: vec![
                Named::new("Idea", "#8E7CC3"),
                Named::new("Action", "#C0504D"),
                Named::new("Romance", "#D67FA3"),
                Named::new("Mystery", "#4F81BD"),
                Named::new("Backstory", "#9BBB59"),
            ],
            statuses: ["To do", "First draft", "Revised draft", "Final draft", "Done"].iter().map(|s| (*s).to_owned()).collect(),
            keywords: Vec::new(),
            targets: Targets::default(),
            fields: vec![FieldDef { name: POV_FIELD.into(), kind: FieldKind::Text, choices: Vec::new() }],
            words: Vec::new(),
            draft_name: String::new(),
            drafts: Vec::new(),
            export_draft: String::new(),
            series: String::new(),
            extra: Default::default(),
        }
    }
}

impl Settings {
    /// The color of a label.
    pub fn label_color(&self, name: &str) -> Option<&str> {
        self.labels.iter().find(|l| l.name == name).map(|l| l.color.as_str()).filter(|c| !c.is_empty())
    }

    /// The color of a keyword.
    pub fn keyword_color(&self, name: &str) -> Option<&str> {
        self.keywords.iter().find(|l| l.name == name).map(|l| l.color.as_str()).filter(|c| !c.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_names_fall_back_sensibly() {
        let mut b = BookInfo { title: "The Long Road".into(), author: "Ada Q. Writer".into(), ..BookInfo::default() };
        assert_eq!(b.header_surname(), "Writer");
        assert_eq!(b.header_title(), "The Long Road");
        b.pen_name = "A. Q. Penman".into();
        assert_eq!(b.byline(), "A. Q. Penman");
        assert_eq!(b.header_surname(), "Penman");
        b.short_title = "Road".into();
        assert_eq!(b.header_title(), "Road");
    }
}
