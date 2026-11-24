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

## The board

View › Board (Mod+4) is a canvas for thinking: cards anywhere, with lines between them. Double-click
the canvas (or Add Card) for a free note; Add the Selected Item puts a binder item on the board as a
card that always shows its current title and synopsis. Drag cards to move them, drag from a card's
edge to another card to draw a line, Delete takes selected cards off the board (items stay in the
binder). Double-click a note to write on it, or an item's card to open the item. Arrange lines the
selected cards (or all) up in a grid. Every change can be undone, and the board is kept with the
project.

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
Compare opens the snapshot beside the current text word by word: added words underlined, removed
words struck through, with a count of each (`snapshot.compare` also compares two snapshots).

## Targets

File › Targets sets the manuscript's word target, a deadline and your writing days, and a target
for this session. The status bar shows your progress and how many words a writing day the
deadline needs. Start a timed sprint there too.

**Writing History** (on File › Targets, or in the command palette) shows a calendar of the words
written each day, darker for busier days, with the month's total, your best day and your current
streak. Deleted words count against the day, so the numbers stay honest. The history is yours
alone: it is kept on this computer beside the app's preferences, never in the project, so it does
not travel with the project or show up for anyone you share it with.

## Collections

A collection is a list of items beside the binder; the items stay where they are. Right-click an
item › Add to Collection to put it in a hand-picked collection (or start a new one). After a
project search, Save Search keeps the search as a collection that always shows what matches now.
When the project has collections, the menu above the binder switches between the binder and each
collection; × takes an item out of a hand-picked collection.

## Thread lanes

View › Lanes (Mod+6) lays the scenes out in story order, one row per plot thread: a scene sits in
a thread's lane when it is linked to that thread's entry (Plot Thread entries in the Story Bible,
linked in the inspector), and scenes in no thread share the last row. Switch to Points of view for
one row per point-of-view character. Drag a card to another lane to move the scene to that thread
(or point of view); double-click a card to open the scene. Gaps in a row show where a thread goes
quiet.
