# Changelog

What changed in each release of Wordsmith Manuscript. The book view's own versions, and what
Wordsmith Book must do when one changes, are in `docs/book-handoff.md`.

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
