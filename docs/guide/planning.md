# Planning

## The corkboard

View › Corkboard (Mod+2) shows the selected folder's items as index cards: the title, the
synopsis, the label's color and the status. Drag a card to move the item; New Card adds an item at
the end; double-click a card to edit its title and synopsis.

## The outline

View › Outline (Mod+3) shows everything under the selected folder as a table: title and synopsis,
label, status, words, target with progress, one column for each custom field, and keywords. Click a heading to sort
(sorting only changes the view; "Apply This Order to the Binder" makes it real, as one undo step).
Double-click a cell to edit it. Drag rows to move items when the table is in binder order.

The binder, the corkboard and the outline always show the same order.

## The inspector

The inspector holds the selected item's synopsis, label, status, custom fields, target, keywords,
linked bible entries, notes, comments and snapshots. Changes are kept when you leave a field.

**Custom fields.** Every project starts with a Point of view field. Add your own under Project
fields at the foot of the inspector: text, a date (written YYYY-MM-DD, so dates sort in order), a
checkbox, or a list of choices (scene date, location, weather, "needs research"). Each field is a
column in the outline, where it can be edited and sorted. Renaming a field keeps every value
(`project.renameField`); removing one in use asks first (`project.setFields`).

Labels, statuses and keywords come from the project's lists; removing a name in use asks first and
then clears it from the items (`project.setLabels`, `project.setStatuses`, `project.setKeywords`).

## Filters

Above the binder, filter by label, status, keyword and point of view. Items that do not match are
hidden in the binder, the corkboard and the outline; their folders stay so you can see where things
are. Show All clears the filter.

## Snapshots

Take Snapshot in the inspector keeps a named, dated copy of the item's text. Restore puts it back
(after keeping the current text as a snapshot too). Snapshots live in the project beside the text.

## Targets

File › Targets sets the manuscript's word target, a deadline and your writing days, and a target
for this session. The status bar shows your progress and how many words a writing day the
deadline needs. Start a timed sprint there too.
