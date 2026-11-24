# Wordsmith Manuscript: behavior spec

As of 2026-11-24. This is how the app behaves. The build plan (`docs/PLAN.md`) says what is built
when; this file says what it does. Feature ids are the plan's (section 7).

Each statement is marked:

- **[V] Verified**: checked against two or more public sources;
- **[S] Sourced**: one public source;
- **[D] Designed**: our own design, recorded here (and in a decision where the plan asks for one).

## Sources

| Key | Public source |
| --- | --- |
| S1 | https://en.wikipedia.org/wiki/Standard_manuscript_format |
| S2 | https://www.mtsu.edu/offcenter/submit/ |
| S3 | https://fictionfoundry.alumni.columbia.edu/workshop_manuscript_guidelines |
| S4 | https://www.cmu.edu/dietrich/english/student-experience/adamson-award-info/fiction-adamsons-2022-guidelines.pdf |
| S5 | https://spec.commonmark.org/0.31.2/ (the CommonMark specification, for the text dialect) |
| S6 | https://www.rfc-editor.org/rfc/rfc9562 (UUID version 7, for item ids) |

General long-form practice (a book kept as parts, chapters and scenes; plans, notes and research
kept beside the text; drafts revised in passes) is the plan's section 3 and is marked [D] where we
turn it into behavior.

## 1. Binder and roles (BND-1, BND-2, BND-3, BND-5)

1. The binder is a tree in a side panel with five fixed top-level areas, always in this order:
   **Manuscript**, **Story Bible**, **Research**, **Notes**, **Trash**. Areas cannot be renamed,
   moved or deleted. [D]
2. Items are folders, text documents, bible entries, media files, web links or screenplays. Folders
   and documents can both hold children, so a chapter can be a document with scene documents under
   it. Media, links and screenplays cannot. [D]
3. Every item has a stable id (a lowercase UUID version 7, S6) that never changes on rename or
   move. [S]
4. Dragging an item drops it before, after or inside the target; the drop position is shown while
   dragging. A move into the item itself or one of its descendants is refused. Moves keep the
   item's children. [D]
5. Every Manuscript item has one **role**: Part, Chapter, Scene, Front matter, Back matter or none.
   New items take a role from their parent: inside a Part, Chapter; inside a Chapter, Scene;
   otherwise none. Roles drive export headings, numbering and scene breaks; the text never holds
   "Chapter 3". [D]
6. **Include in export** is a per-item switch, on by default. An excluded item is shown dimmed and
   is left out of every export and the book view, with its children. [D]
7. **Delete** moves an item and its children to the Trash, remembering its area. **Restore** puts
   it back at the end of that area. **Empty Trash** asks for confirmation, naming how many items
   and words will be removed, and cannot be undone; nothing else ever removes an item for good. [D]
8. Every binder change is one command: undoable, and callable by agents. [D]
9. The binder stays responsive with 1,000 items: drawing shows only the visible rows. [D]

## 2. Templates (BND-4)

1. **New project** offers templates: at 0.1 **Novel** and **Short story collection**. [D]
2. Novel: Manuscript holds Front matter (Title page), Part One with Chapter One holding Scene One;
   Story Bible holds empty folders Characters, Places, Objects, Groups, Lore and Plot threads;
   Notes holds Premise and Synopsis; Research is empty. [D]
3. Short story collection: Manuscript holds Front matter (Title page, Contents) and Story One with
   one scene; the bible and notes as for the novel. [D]
4. A template gives fresh ids each time; the label list, status list and targets are set from the
   template. [D]
5. The start window shows recent projects and the template chooser when the app opens with no
   project. [D]

## 3. Editor and styles (ED-1, ED-3, ED-5, ED-10)

1. **Paragraph styles:** Body, Block quote, Verse, Aside (with a kind: letter, note, message or
   another word), Heading 1 to 3 (for front and back matter pages), Scene break. [D]
2. **First paragraph** is not chosen by hand: the first paragraph of an item, and the first after
   a heading or a scene break, is shown and exported without its first-line indent. [V: S1 shows
   unindented openings by example; general typographic practice] [D for the automatic rule]
3. **Character styles:** italic, bold, small caps, strikethrough, and a language span. Underline is
   not offered: it is shown as italic in a manuscript (S1). [S]
4. Only styles the file format can hold are offered, so nothing is lost on save; pasting text with
   other formatting keeps the words and drops what the format cannot hold, and says so in the
   status bar. [D]
5. **Scene break** is its own paragraph. In the editor it shows as a centered symbol; its printed
   symbol is an export setting (default `#`, S1). [S]
6. **Smart typing** (ED-5), per the project language: straight quotes become curly quotes
   (English: “ ” ‘ ’; French: « » with no-break spaces; German: „ “ ‚ ‘), an apostrophe inside a
   word becomes ’, two hyphens become an em dash, a spaced hyphen between words becomes an en dash,
   three periods become an ellipsis. Undo right after a replacement restores what was typed. Each
   rule can be switched off. [D]
7. **Focus mode** (ED-3) hides everything but the text, centered at the editor width; typewriter
   scrolling keeps the line being typed at a fixed height. Escape leaves. [D]
8. **Inline links** (ED-10): text can link to a binder item or bible entry. A link is underlined in
   the link color; hovering shows the target's title and synopsis (an entry's first lines); clicking
   with the command key opens it. A link to a deleted item shows as broken and is never removed
   silently. [D]

## 4. The stitched view (ED-2)

1. Any selection of binder items (a chapter, a part, the whole manuscript, or a hand-picked set)
   opens as one continuous text, in binder order. [D]
2. A divider showing each item's title separates items. Dividers cannot be edited or deleted;
   selecting across one and typing replaces the text on both sides but keeps both items. [D]
3. A paragraph typed at a boundary belongs to the item above. Deleting all of an item's text leaves
   the item, empty. [D]
4. The view has one undo history; undo after a cross-boundary edit restores both items. [D]
5. Changes are written back to each item at every save point: auto-save, Save, closing the view,
   switching to another view. Only changed items are written. [D]
6. While a stitched view holds an item, the item opens read-only elsewhere, with a note saying
   where it is being edited. [D]

## 5. Find and replace (ED-4)

1. Find in the document, the selection, or the whole project (every text item, notes and synopses
   in the project's reading order). [D]
2. Options: match case, whole word, regular expression. [D]
3. Replace All across the project takes a snapshot of every item it changes first (INS-5), then
   reports how many replacements in how many items. [D]

## 6. Spelling (ED-11)

1. Spelling is checked as you type with the core's spelling, in the project language. [D]
2. The project word list holds the writer's added words and, automatically, every bible entry's
   name and aliases; renaming an entry updates the list. [D]

## 7. Planning views (PLN-1, PLN-2)

1. **Corkboard** (PLN-1): the children of the selected folder or document as index cards in a
   grid, in binder order. A card shows the title, the synopsis, a stripe in the label's color and a
   stamp with the status. Dragging a card reorders the binder at once. A new card makes a new text
   item at that position with the role the binder would give it. Double-clicking a card's text
   edits it in place. [D]
2. **Outline** (PLN-2): a table of the selected folder's descendants, indented as in the binder.
   Columns: title and synopsis, label, status, words, target and progress, point of view, keywords.
   Clicking a header sorts by that column (sorting is a view; it never reorders the binder unless
   the writer chooses "Apply this order"). Title, synopsis, label, status and target are editable
   in place. Rows can be dragged to reorder when the table is in binder order. [D]
3. Point of view is a project field (a character's name or bible entry) until custom fields come
   at 0.2; it is shown in the outline and used by filters. [D]
4. Binder, corkboard and outline show the same items; a change in one shows at once in the
   others, as one undoable command. [D]

## 8. Inspector and metadata (INS-1 to INS-5)

1. The inspector panel shows the selected item's synopsis, notes, label, status, keywords, linked
   bible entries, target and snapshots. [D]
2. **Synopsis** is a short plain-text summary; it is the card's text and the outline's second line.
   **Notes** are rich text, per item; the project has its own notes too. [D]
3. **Label**: one per item, from the project's label list (a name and a color). **Status**: one per
   item, from the project's status list. Default statuses: To do, First draft, Revised draft, Final
   draft, Done. Default labels: Idea, Action, Romance, Mystery, Backstory (each with a color). Both
   lists are editable; removing a name in use asks whether to clear it from items. [D]
4. **Keywords**: any number per item, from the project keyword list, each with a color. [D]
5. **Linked bible entries** (INS-4): chosen by hand at 0.1; shown as chips; each opens the entry. [D]
6. **Snapshots** (INS-5): a snapshot is a named, dated copy of one item's text. Take one by hand,
   or automatically before Replace All. The list shows name and date; Restore replaces the text
   (after taking a snapshot of the current text, so restore can be undone). [D]

## 9. Story bible (BIB-1, BIB-2, BIB-3)

1. Entry types: Character, Place, Object, Group, Lore, Plot thread. Each entry has a name, aliases,
   fields from its type's template, and a free body. [D]
2. Default templates (BIB-2 at 0.1):
   - Character: role in story, age, appearance, personality, wants, needs, flaw, history;
   - Place: kind, location, description, sensory details, history;
   - Object: kind, owner, description, significance;
   - Group: kind, members, purpose, history;
   - Lore: kind, rules, history;
   - Plot thread: kind (main or sub), question, stakes, resolution. [D]
3. Templates are data, not code; the same general entries layer serves any app (plan section 5.6). [D]
4. **Backlinks** (BIB-3): an entry lists every item that links to it, and every item whose text
   mentions its name or an alias as a whole word (case-sensitive for names starting with a capital
   letter). Mentions and links are shown separately. [D]

## 10. Targets and counts (PRG-1, PRG-2)

1. **Counts** are live for the document, the selection, a folder and the whole manuscript: words
   and characters (with and without spaces). Words are counted with the core's word rules, the same
   in every view and export. [D]
2. Only included Manuscript items count toward the manuscript total. [D]
3. **Manuscript target**: a word count, an optional deadline, and the writing days of the week;
   the app shows the words per writing day needed to finish by the deadline. [D]
4. **Session target**: words for this session; the session starts when the project opens and can be
   reset; deleted words reduce the session count. [D]
5. **Item targets**: a word count per item, with progress in the outline and the inspector. [D]

## 11. Search and filters (SRC-1, SRC-3)

1. Project search finds words in text, notes, synopses and bible entries; results are shown in the
   binder area, grouped by item, with the matching line. Options as for find. [D]
2. Filters by label, status, keyword and point of view apply to the binder, the corkboard and the
   outline together; items that do not match are hidden, their ancestors stay as context. [D]
3. Search on a 300,000-word project answers in under 200 ms (plan section 11). [D]

## 12. Files and safety (FIL-1, FIL-2, FIL-3)

1. **Auto-save** saves changes on the core's schedule; every file is written atomically and the
   binder last. [D]
2. **Backups**: a packed copy (`.wsmz`) on close and on a schedule (default every 30 minutes of
   editing), with a number kept (default 10). [D]
3. **Recovery**: unsaved work is kept in `.recovery/` inside the project while the app runs and is
   offered back after a crash. [D]

## 13. Themes and keys (ACC-1, ACC-2)

1. Light, dark and sepia themes. Sepia changes the editor surface only. The editor font, size and
   width are separate from the export settings. [D]
2. Every command is reachable from the keyboard and listed in the command palette; keys are
   editable (core). [D]

## 14. Command line and agents (AUT-1, AUT-2)

1. `manuscript-cli convert`, `export`, `counts`, `verify` (the word-for-word check) and `report`.
   [D]
2. Every Manuscript command is an agent tool through the core. [D]

## 15. Import rules (IMP-1 to IMP-6)

1. Every import shows a preview of the binder it will make, a report of what was created and of
   formatting it could not keep, and a **word-for-word check** that the imported text matches the
   source exactly. Nothing is imported until the writer accepts the preview. [D]
2. **Headings to roles** (IMP-1, IMP-3, IMP-4): by default Heading 1 makes a Part when the document
   also has Heading 2s, otherwise a Chapter; the next level down makes a Chapter; the level below
   that a Scene. A paragraph that matches "Chapter", "Part" or "Book" followed by a number or number
   word starts a Chapter or Part even without a heading style. The rules are editable in the
   preview. [D]
3. **Scene breaks** are detected from a paragraph holding only `***`, `* * *`, `#`, `~`, a run of
   one symbol, a centered short symbol line, or an empty paragraph with spacing that sets it apart.
   They split the chapter into scenes (default) or stay as scene-break paragraphs. [D]
4. Italic, bold, small caps, strikethrough, notes and comments are kept. Fonts, sizes and colors
   are dropped (and listed in the report). [D]
5. **Document sets** (IMP-2): the writer picks files or a folder. Name rules place each file by its
   role in parentheses: "(Plan)" and "(Outline)" into Notes, "(Rough Work)" and "(Notes)" into
   Research, "(First Draft)", "(Second Draft)" and "(Draft)" into the Manuscript; files without a
   role go to the Manuscript. Several files of one draft ("Part 1", "Part 2" or numbered names) are
   merged in name order into one manuscript. When a set has more than one draft, the latest draft
   goes into the Manuscript and earlier ones become drafts (DRF-1, from 0.3; into Research as
   folders before that).
   The rules are editable and saved in `project.toml`. [D]
6. **Markdown and plain text** (IMP-5): split by heading (Markdown) or by a chosen separator line
   (default: a line holding only `***` or `#`). [D]
7. **Markdown notes folder** (IMP-6): each `.md` file becomes a bible entry named after the file;
   front matter becomes fields; `[[Name]]` and `[[Name|text]]` links become links to the entry of
   that name, keeping the display text; a link with no matching entry is kept as text and reported.
   The folder's subfolders become bible folders. [D]

## 16. Export and the standard manuscript format (EXP-1 to EXP-5)

### 16.1 Options

1. Which items to include (default: every included Manuscript item); how parts, chapters and scenes
   are titled and numbered ("Chapter 1", "Chapter One", "One", the item title, or none); the
   scene-break symbol; whether front and back matter are included; replace-text rules; saved
   presets. Export never changes the project. [D]

### 16.2 The standard manuscript layout (EXP-1, EXP-2)

| Rule | Value | Mark |
| --- | --- | --- |
| Paper | US Letter (8.5 × 11 in) or A4, by project setting | [S: S1] |
| Margins | 1 inch on every side | [V: S1, S2] |
| Font | 12 pt; a serif font (Times-like) or a monospaced font (Courier-like), by preset | [V: S1, S2, S3] |
| Line spacing | Double spaced | [V: S1, S2, S3, S4] |
| Paragraphs | First line indented 0.5 inch, no extra space between paragraphs, ragged right | [S: S1] |
| Header | On every page but the title page, top right: surname / short title / page number | [V: S1, S3] |
| Page numbers | Every page after the title page, starting at 1 on the first page of text | [V: S1, S3, S4] |
| Title page | Top left: the author's legal name and contact lines; top right: approximate word count; centered about a third down: title, then "by" and the name the book is published under | [S: S1] |
| Word count | Rounded: under 1,000 words to the nearest 100; under 25,000 to the nearest 500; above that to the nearest 1,000; shown as "about 82,000 words" | [D; S1 asks for an approximate count] |
| Chapter starts | A new page; the chapter title starts about a third of the way down | [S: S1] |
| Scene breaks | A centered `#` on its own line (setting) | [S: S1] |
| End | A centered "END" line after the last paragraph (setting, on by default) | [S: S1] |
| Italic | Kept as italic (an option shows it as underline for older guidelines) | [S: S1] |

The surname and short title for the header come from the project's author and title (each can be
overridden in the export settings). [D]

### 16.3 Other exports

1. ODT, RTF, Markdown, plain text and HTML use the same options and the core's writers (EXP-3,
   EXP-4). [D]
2. **Book package** (EXP-5): a `.wsmz` of the project with the book metadata filled in, for
   Wordsmith Book (plan section 9.2, `docs/book-handoff.md`). [D]
3. Every export is checked word for word against the project before it is reported as done. [D]
