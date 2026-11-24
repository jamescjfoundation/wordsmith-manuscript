// SPDX-License-Identifier: MIT OR Apache-2.0
//! Filters (SRC-3, SPEC 11.2): show only items with a label, status, keyword or point of view,
//! in the binder, the corkboard and the outline together. Items that do not match are hidden;
//! their ancestors stay as context. A filter is a view: it never changes the project.

use std::collections::HashSet;

use jcj_shell::egui::{self, Ui};
use wordsmith_manuscript_model::{ItemId, POV_FIELD, Project};

/// What to show. Within one kind any value matches; across kinds all must match.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Filter {
    /// Labels.
    pub labels: Vec<String>,
    /// Statuses.
    pub statuses: Vec<String>,
    /// Keywords.
    pub keywords: Vec<String>,
    /// Points of view.
    pub povs: Vec<String>,
}

impl Filter {
    /// Whether the filter shows everything.
    pub fn is_empty(&self) -> bool {
        self.labels.is_empty() && self.statuses.is_empty() && self.keywords.is_empty() && self.povs.is_empty()
    }

    /// Whether one item matches (by itself, not its children).
    pub fn matches(&self, p: &Project, id: ItemId) -> bool {
        let Some(item) = p.binder.get(id) else { return false };
        let meta = p.text(id).map(|t| t.meta.clone()).unwrap_or_default();
        let any = |want: &[String], have: &[String]| want.is_empty() || want.iter().any(|w| have.contains(w));
        any(&self.labels, &item.label.clone().into_iter().collect::<Vec<_>>())
            && any(&self.statuses, &item.status.clone().into_iter().collect::<Vec<_>>())
            && any(&self.keywords, &meta.keywords)
            && any(&self.povs, &meta.fields.get(POV_FIELD).cloned().into_iter().collect::<Vec<_>>())
    }
}

/// The items to hide: those that do not match and have no matching descendant.
pub fn hidden(p: &Project, f: &Filter) -> HashSet<ItemId> {
    let mut out = HashSet::new();
    if f.is_empty() {
        return out;
    }
    for row in p.binder.rows() {
        let keep = row.item.walk().iter().any(|i| f.matches(p, i.id));
        if !keep {
            out.insert(row.item.id);
        }
    }
    out
}

/// The points of view used in the project, for the filter's list.
pub fn povs(p: &Project) -> Vec<String> {
    let mut v: Vec<String> = p.texts.values().filter_map(|t| t.meta.fields.get(POV_FIELD).cloned()).collect();
    v.sort();
    v.dedup();
    v
}

fn pick(ui: &mut Ui, title: &str, choices: &[String], chosen: &mut Vec<String>) -> bool {
    let mut changed = false;
    let text = if chosen.is_empty() { title.to_owned() } else { format!("{title}: {}", chosen.join(", ")) };
    let r = egui::ComboBox::from_id_salt(("manuscript-filter", title)).selected_text(text).width(120.0).show_ui(ui, |ui| {
        for c in choices {
            let mut on = chosen.contains(c);
            if ui.checkbox(&mut on, c).changed() {
                if on {
                    chosen.push(c.clone());
                } else {
                    chosen.retain(|x| x != c);
                }
                changed = true;
            }
        }
    });
    r.response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::ComboBox, true, title));
    changed
}

/// The filter bar (above the binder). Returns whether the filter changed.
pub fn bar(ui: &mut Ui, p: &Project, f: &mut Filter) -> bool {
    let labels: Vec<String> = p.settings.labels.iter().map(|l| l.name.clone()).collect();
    let keywords: Vec<String> = p.settings.keywords.iter().map(|l| l.name.clone()).collect();
    let statuses = p.settings.statuses.clone();
    let povs = povs(p);
    let mut changed = false;
    ui.horizontal_wrapped(|ui| {
        changed |= pick(ui, "Label", &labels, &mut f.labels);
        changed |= pick(ui, "Status", &statuses, &mut f.statuses);
        changed |= pick(ui, "Keyword", &keywords, &mut f.keywords);
        changed |= pick(ui, "Point of view", &povs, &mut f.povs);
        if !f.is_empty() && ui.small_button("Show All").clicked() {
            *f = Filter::default();
            changed = true;
        }
    });
    changed
}

#[cfg(test)]
mod tests {
    use super::*;
    use wordsmith_manuscript_model::{Area, Item, ItemText, Kind, Place};

    fn id(n: u64) -> ItemId {
        ItemId::from_counter(n)
    }

    fn project() -> Project {
        let mut p = Project::new();
        let mut ch = Item::with_id(id(1), Kind::Folder, "Chapter");
        let mut a = Item::with_id(id(2), Kind::Text, "A");
        a.label = Some("Action".into());
        let b = Item::with_id(id(3), Kind::Text, "B");
        ch.children = vec![a, b];
        p.binder.insert(Place::end_of(Area::Manuscript), ch).unwrap();
        p.binder.insert(Place::end_of(Area::Manuscript), Item::with_id(id(4), Kind::Text, "C")).unwrap();
        let mut t = ItemText::default();
        t.meta.keywords = vec!["night".into()];
        t.meta.fields.insert(POV_FIELD.into(), "Mara".into());
        p.texts.insert(id(3), t);
        p
    }

    #[test]
    fn non_matching_items_hide_and_ancestors_stay() {
        let p = project();
        let f = Filter { labels: vec!["Action".into()], ..Filter::default() };
        assert_eq!(hidden(&p, &f), [id(3), id(4)].into_iter().collect());
        let f = Filter { keywords: vec!["night".into()], povs: vec!["Mara".into()], ..Filter::default() };
        assert_eq!(hidden(&p, &f), [id(2), id(4)].into_iter().collect());
        let f = Filter { keywords: vec!["night".into()], labels: vec!["Action".into()], ..Filter::default() };
        assert_eq!(hidden(&p, &f).len(), 4, "all kinds must match");
        assert!(hidden(&p, &Filter::default()).is_empty());
        assert_eq!(povs(&p), ["Mara"]);
    }
}
