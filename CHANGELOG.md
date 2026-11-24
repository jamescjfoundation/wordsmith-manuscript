# Changelog

What changed in each release of Wordsmith Manuscript. The book view's own versions, and what
Wordsmith Book must do when one changes, are in `docs/book-handoff.md`.

## 0.3.0 (prepared 2026-11-24; released when a maintainer approves it)

Drafts and finish. **Projects saved by 0.3 use schema 3 and do not open in 0.2** (decision 0006);
0.1 and 0.2 projects open in 0.3 unchanged.

- Drafts: keep the manuscript as it is and carry on with a new draft; compare drafts chapter by
  chapter and scene by scene; write any draft again; choose the draft that exports (and that
  Wordsmith Book typesets). A document set's earlier drafts come in as drafts.
- The interface in Spanish as well as English, following the computer's language or chosen in
  the command palette; more languages can be added as catalogs.
- Polish: the inspector's and windows' texts translate too; lint clean-up.

## 0.2.0 (prepared 2026-11-24; released when a maintainer approves it)

Daily-work completeness. **Projects saved by 0.2 use schema 2 and do not open in 0.1**
(decision 0005); 0.1 projects open in 0.2 unchanged.

### Writing

- Split the editor side by side or top and bottom; a document is editable in one pane at a time.
- Comments on selected words, written in the inspector, exported as real comments to DOCX, ODT and
  RTF when asked.
- Revision passes: what you type during a pass shows in its color until you clear it.
- The page view: the text as standard manuscript pages with the page count.
- Grammar hints, offline and off unless you turn them on.

### Planning and finding

- Custom fields (text, date, checkbox, list) in the inspector and as outline columns.
- Compare a snapshot with the current text word by word.
- Collections: hand-picked lists and saved searches beside the binder. Bookmarks above it.
- The freeform board: cards anywhere, lines between them, cards that stand for documents.
- A writing history calendar, kept on this computer.

### Story bible, templates, research

- Your own bible entry types; renaming a field keeps every value.
- Your own project templates (Save as Template).
- Pictures, PDFs and web links in Research, shown in the editor area.

### Import, export, files

- Web pages (their article only) and OPML outlines import.
- An EPUB reading copy (notes as numbered notes at each chapter's end).
- Safe in synced folders: outside changes are read in, both versions kept when both sides
  changed, and nothing is written over unseen without a copy in `conflicts/`.
- The "Offered fixes" inbox for fixes Wordsmith Book offers back (`docs/book-handoff.md`).

### Accessibility

- The editor and every view are named for screen readers.

## 0.1.0 (prepared 2026-11-24; released when a maintainer approves it)

The first usable release: plan, draft, revise and finish a novel in one project.

### Projects and files

- A project is a folder of plain text files (`.wsm`): `project.toml`, `binder.toml`, one Markdown
  file per document and bible entry, snapshots and media beside them (`docs/format.md`, schema 1).
  A packed single file (`.wsmz`) for sending, backups and Wordsmith Book.
- Saving writes only changed files, each safely, the binder last; unsaved work is kept for
  recovery; backups on close and every 30 minutes, ten kept.
- Novel and Short story collection templates; a start window with templates and recent projects.

### Writing

- The binder with five areas, roles (part, chapter, scene, front matter, back matter), include in
  export, drag and drop, the Trash.
- The editor with Manuscript's styles only (body, quote, verse, aside, headings, scene breaks;
  italic, bold, small caps, strikethrough, language), smart quotes, dashes and ellipses per
  language, plain paste, counts.
- Reading and editing a chapter, a part or the whole manuscript as one text.
- Find and replace in a document, a selection or the project; spelling with the project's words;
  links to items and bible entries; focus mode, typewriter scrolling, light, dark and sepia.

### Planning

- The inspector (synopsis, notes, label, status, point of view, keywords, linked entries, target,
  snapshots), the corkboard, the outline (sortable, editable), filters across all three.
- The story bible: characters, places, objects, groups, lore and plot threads from templates, with
  aliases, fields, bodies and backlinks; a Markdown notes folder imported as entries.
- Targets with deadlines and writing days, session targets and sprints; snapshots; project search.

### Import and export

- DOCX, ODT, RTF, Markdown, HTML and plain text imports mapped to parts, chapters and scenes, and
  sets of documents placed by the roles in their names; every import checked word for word.
- Standard manuscript DOCX and PDF; DOCX, ODT, RTF, Markdown, plain text and HTML; the book package
  for Wordsmith Book (book view version 1); every export checked word for word.

### Automation

- `manuscript-cli`: convert, export, counts, verify, report, run, and every command as an agent
  tool (`manuscript-cli mcp`).

### Known gaps

- The PDF's producer field names the upstream engine (core issue #5, C4).
- DOCX, ODT and RTF fixtures saved by two word processors are still to be added (`docs/fixtures.md`).
- Native file pickers are not offered by the core's shell yet: paths are typed on the File pages.
