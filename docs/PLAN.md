# Wordsmith Manuscript: build plan

As of 2026-11-24. Status: accepted as the build plan; changed only by pull request.

Wordsmith Manuscript is the J. C. J. Foundation's open-source app for writing novels and other long
work: a binder of parts, chapters and scenes; index cards on a corkboard; an outline table; a story
bible of characters, places and plot threads beside the text; word targets; snapshots; and exports
that give an editor a standard manuscript and give Wordsmith Book a finished book to typeset.

It is the second app on [open-source-core](https://github.com/jamescjfoundation/open-source-core),
the foundation's shared base, after
[Wordsmith Script](https://github.com/jamescjfoundation/wordsmith-script). The core already holds
the window, ribbon, panels, File page, preferences, saving, backups, recovery, spelling, undo,
format import and export, PDF, cards and boards, theming and agent control. This repository holds
only what is about writing a manuscript.

## Contents

1. How to follow this plan
2. Goals and non-goals
3. Who it is for, and the writer's loop
4. Ground rules
5. Architecture
6. Data format
7. Features by release
8. Import
9. Export and the hand-off to Wordsmith Book
10. Milestones and steps
11. Testing and quality gates
12. Risks
13. Open questions
14. Third-party building blocks

## 1. How to follow this plan

This plan is meant to be carried out step by step, by people and coding agents, until v1.0.

- **Order.** Milestones run in the order of section 10. Steps inside a milestone run in order
  unless they say they can run in parallel. A step is one branch and one pull request, named as the
  step says (`m2/stitched-view`).
- **Each step's "Done when"** is its acceptance test. A step is finished when every item there is
  true, `cargo xtask ci` passes, and the step's tests are in the pull request.
- **Progress** is recorded in `docs/progress.md`: one line per merge to `main` (date, branch, what
  changed, test count), as Wordsmith Script does. At the end of a milestone, mark its row in the
  table at the top of section 10.
- **Decisions** about this app's behavior go in `docs/decisions/NNNN-title.md` (context, decision,
  consequences, revisit when). Where this plan says "decision", write one in the step that makes
  it. Decisions about shared interfaces go in the core's `docs/decisions/`, never here.
- **Core changes** (section 5.4) follow the core's AGENTS.md, "Building between an app and the
  core": a core issue first, both repositories side by side with `cargo xtask dev-link`, the core
  pull request first, a core release, then the app's pin. A step that needs a core change lists it
  under "Needs". Releasing the core, moving upstream pins and merging pull requests need a person's
  approval.
- **Changing the plan.** If a step turns out wrong (an interface the core does not have, a feature
  that does not fit), stop, write down why in the pull request, and change this plan in the same or
  a separate pull request. Do not work around the plan silently.
- **Feature ids** (`BND-1`, `ED-3`) in section 7 are used in branch descriptions, tests and
  `docs/progress.md`, so each feature can be traced from plan to code.

## 2. Goals and non-goals

### Goals

1. A writer can plan, draft, revise and finish a 200,000-word novel in one project, on macOS,
   Windows and Linux, without losing a word.
2. The usual long-form workflow is there from the first release: binder, editor, corkboard,
   outline, inspector, targets, snapshots, export.
3. A writer whose book lives in a set of word-processor documents can import the set and get a
   structured project, with every word checked against the source.
4. The story bible (characters, places, objects, groups, lore, plot threads) lives in the project,
   is linked from the text, and can later be shared with Wordsmith Script.
5. Files are open, plain and durable: readable in a text editor, friendly to version control and
   cloud folders, and still usable if the app disappears.
6. The finished manuscript reaches Wordsmith Book as clean, structured content, with no manual
   reformatting.
7. Built from open-source-core and established open-source libraries, with every license recorded
   and checked in CI.

### Non-goals for v1.0

- Book design and typesetting: Wordsmith Book's job. Manuscript exports only a standard manuscript
  PDF and a plain reading copy.
- Real-time co-writing and cloud accounts. Local files first; device sync is after v1.0.
- Screenplay formatting and editing: Wordsmith Script's job. A project can hold screenplays, but
  Script edits them (SCR-1).
- AI writing features. If added later, behind a setting that is off by default.
- Mobile apps, until the file format has been stable for a release.
- Proprietary file formats, as in Script (Script decision 0042): an import needs a legitimately
  published description of the format first.

## 3. Who it is for, and the writer's loop

Primary users, in order:

1. Novelists and long-form writers who draft in a word processor and want structure, or who are
   moving from another writing app.
2. The foundation itself, preparing books for its own imprint through Wordsmith Book.
3. Students and writing groups the foundation supports.

What their work tells us, and what the app must do about it:

| How long-form writers actually work | What Manuscript must do |
| --- | --- |
| A book kept as a set of documents named by role: "(Plan)", "(First Draft)", "(Rough Work)", "(Second Draft)" | Import a set and keep each document's role; plans, scratch work and several full drafts belong in one project |
| One novel of about 200,000 words split across several documents | Handle very long projects smoothly; merge several documents into one manuscript on import, in order |
| A story bible of character sheets in the writer's own template, worldbuilding notes, timelines and plots, kept in a separate notes tool | A real bible with templates and fields, not loose notes; import a folder of Markdown notes with `[[links]]` |
| Spreadsheets tracking a series and word counts per book | Series-level view later; progress and word-count tracking now |
| Self-publishing through an ebook tool | The path to publication matters: the hand-off to Wordsmith Book |

The writer's loop the app is designed around:

1. **Start** a project from a template (novel, short story collection, non-fiction) or by
   importing documents.
2. **Plan**: a premise, cards for chapters and scenes on the corkboard, characters and places in
   the bible, plot threads.
3. **Draft** scene by scene, with the scene's synopsis, notes and linked bible entries in the
   inspector, and a session target in the status bar.
4. **Restructure**: drag scenes in the binder or on the corkboard; read a chapter or an act as one
   stitched document; filter by label, status, point of view or keyword.
5. **Revise**: snapshot before changes, compare with it, track status from "To do" to "Final",
   later start a second draft beside the first.
6. **Finish**: export a standard manuscript for agents and editors, or hand the book to Wordsmith
   Book.

## 4. Ground rules

These hold for every step. They restate the core's AGENTS.md where it applies and add this app's
own rules; `AGENTS.md` in this repository carries them for agents (step M0.1).

1. **Public, forever.** This repository is public, history included. No one's name, email or
   account, no details of anyone's machine, no private writing (even as test data), no secrets, no
   conversation quotes. Test fixtures are public-domain or written for the test. Private
   manuscripts may be used only in local test runs whose inputs and outputs never enter the
   repository (`.private/` is ignored by Git).
2. **Our own names and design.** Every interface name, menu label, help text, template and icon is
   our own. Never use another product's name, trademarks or distinctive feature names, even where
   the word would fit. Generic vocabulary (binder, corkboard, outline, inspector, snapshot, target,
   keyword, label, status, collection) is fine. Behavior comes from `docs/SPEC.md` (step M0.3),
   which is built from public manuscript conventions and our own design. Where the spec is silent,
   design it fresh and record a decision. The maintainers keep the list of names to avoid; if
   unsure about a name, ask in the pull request.
3. **The writer's words are sacred.** Nothing the writer typed is dropped or rewritten by saving,
   importing, exporting, merging or splitting. Every importer and exporter proves it with a
   word-for-word test (section 11). Text is never "cleaned up" without the writer asking.
4. **App or core.** Before writing anything, ask the core's four questions (core AGENTS.md, step 1).
   Section 5.4 has the answers for everything this plan already knows about. Never name WordCraft or
   craft-libs types here, never copy code from another app repository, never change a core
   interface from this repository.
5. **House rules for code** (the core's): SPDX line, `#![forbid(unsafe_code)]` and the six
   panicking lints denied at every crate root; product code returns errors and never panics; input
   from files is bounded and never trusted; new dependencies pass the license allowlist (core
   decision 0001); public items documented in plain English.
6. **Licenses.** MIT, Apache-2.0, BSD, ISC, Zlib and the core allowlist's other entries may be
   linked; MPL-2.0 libraries may be linked unchanged. LGPL and GPL code is never copied or linked; a
   GPL program may only be run as a separate, optional program the user installed.
7. **Golden files are the judge.** Standard manuscript PDF and DOCX output, and the format's
   round trips, are compared with reviewed reference files. A change that alters them is
   deliberate, reviewed and recorded in `docs/golden.md`.

## 5. Architecture

### 5.1 Workspace

The repository starts from the core's `template/` (step M0.1) and keeps its shape:

```text
apps/
  manuscript/            desktop app (eframe), implements jcj_shell::App
  manuscript-cli/        command line: convert, export, counts, verify, reports
  manuscript-web/        web build (after v1.0 core support; placeholder until then)
crates/
  manuscript-model/      project, binder, roles, metadata, targets, bible, links, drafts
  manuscript-format/     the project folder: reader, writer, text dialect, migrations, packing,
                         and the read-only book view Wordsmith Book depends on
  manuscript-io/         import mapping, document-set rules, word-for-word verifier,
                         export presets, standard manuscript layout
  manuscript-ui/         views: binder, editor and stitched view, corkboard, outline,
                         inspector, bible, search, export dialog, ribbon tabs, panels
assets/brand/manuscript/ tokens.json, mark, icons (the brand lives here, never in the core)
docs/                    PLAN.md, SPEC.md, format.md, book-handoff.md, decisions/, guide/,
                         progress.md, golden.md, fixtures.md
tests/fixtures/          public-domain and written-for-test inputs, with provenance
xtask/                   the core's shared tasks plus this app's (SHIPPED, PRODUCT)
```

Package names are `wordsmith-manuscript-*` in `Cargo.toml` (folder names above are short). App id:
`org.jcjfoundation.manuscript`; extension namespace: `manuscript.` (core decision 0006).

### 5.2 Layers

Each layer calls only the layers under it; `cargo xtask layers` gets this app's table in step M0.1.

| Layer | Crate | Depends on |
| --- | --- | --- |
| 1 | `manuscript-model` | `jcj-doc` (and `serde`, `uuid`); no files, no interface |
| 2 | `manuscript-format` | model, `jcj-doc`, `jcj-project` (atomic writes, backups, recovery, auto-save clock) |
| 3 | `manuscript-io` | format, model, `jcj-interchange`, `jcj-layout`, `jcj-output`, `jcj-features`, `jcj-proof` |
| 4 | `manuscript-ui` | everything below, `jcj-engine`, `jcj-shell`, `jcj-theme`, `jcj-board`, `jcj-agents` |
| 5 | `apps/*` | the crates above |

`manuscript-format` must stay free of interface and engine code: Wordsmith Book depends on it.

### 5.3 What the core gives Manuscript

| Need | Core crate | How Manuscript uses it |
| --- | --- | --- |
| Rich text model, styles, notes, comments, extension data | `jcj-doc` | One `Document` per binder text item; Manuscript's paragraph and character styles; `manuscript.*` keys for roles, links and marks |
| Editing, undo, commands, recovery | `jcj-engine` | One session per open document (and one for a stitched view); Manuscript's commands registered with `registry_with(commands::merged(..))` |
| Pages, measuring, fonts | `jcj-layout` | Page view and the standard manuscript layout as a `PaginationPass` |
| PDF and printing | `jcj-output` | Standard manuscript PDF; print |
| DOCX, ODT, RTF, Markdown, HTML, text, EPUB writing | `jcj-interchange` | Every import and plain export; Manuscript maps the result to its structure |
| Spelling, hyphenation, user words | `jcj-proof` | Spelling as you type; a project word list fed from bible entry names |
| Atomic saves, backups, recovery, auto-save timing | `jcj-project` | Writes of every file in the project folder; backups of the packed project |
| Notes, bookmarks, revision marks, comparison, headers and footers | `jcj-features` | Comments, revision passes, snapshot compare (`Grouping` by scene), manuscript headers through a `FieldSource` |
| Window, ribbon, panels, File page, preferences, keys, updates | `jcj-shell` | Manuscript implements `App` and supplies tabs, panels, File page sections and About |
| Theme | `jcj-theme` | `assets/brand/manuscript/tokens.json` |
| Cards, corkboard, canvas | `jcj-board` | Corkboard of a folder's children; freeform board |
| Agent control | `jcj-agents` | Every Manuscript command as a tool |

### 5.4 Core changes Manuscript needs

Each is opened as a core issue describing the general need. "Promote" means moving code Script
kept until a second app wanted it (core decision 0014) into the core: Manuscript is that second
app, so the rule "a second app already needs it" now applies. Promotions also need a follow-up
pull request in Script.

| Id | Change | Kind | Needed by | Core issue |
| --- | --- | --- | --- | --- |
| C1 | Template chooser and start window | Promote from Script | M1.4 | [#2](https://github.com/jamescjfoundation/open-source-core/issues/2) |
| C2 | Writing statistics, session targets and the sprint timer | Promote from Script | M7.1 | [#3](https://github.com/jamescjfoundation/open-source-core/issues/3) |
| C3 | Document tabs and split view | Promote from Script | v0.2 (ED-6) | [#4](https://github.com/jamescjfoundation/open-source-core/issues/4) |
| C4 | The app names itself in PDF metadata (known core gap) | Core fix | M6.2 | [#5](https://github.com/jamescjfoundation/open-source-core/issues/5) |
| C5 | A tree widget with drag and drop (binder) | Build in app first, `core-candidate` | M1.3 | [#6](https://github.com/jamescjfoundation/open-source-core/issues/6) |
| C6 | A sortable, editable table widget (outline) | Build in app first, `core-candidate` | M3.2 | [#7](https://github.com/jamescjfoundation/open-source-core/issues/7) |
| C7 | Search across many documents with an index | Build in app first, `core-candidate` | M7.3 | [#8](https://github.com/jamescjfoundation/open-source-core/issues/8) |
| C8 | Editing several documents as one (stitched view) | Build in app first, `core-candidate` | M2.3 | [#9](https://github.com/jamescjfoundation/open-source-core/issues/9) |
| C9 | Word-for-word verification of an import or export | Build in app first, `core-candidate` | M5.1 | [#10](https://github.com/jamescjfoundation/open-source-core/issues/10) |
| C10 | Grammar hints (Harper) beside spelling | Core (`jcj-proof`) | v0.2 (ED-9) | [#11](https://github.com/jamescjfoundation/open-source-core/issues/11) |
| C11 | Story board lanes | Promote from Script | v1.0 (PLN-4) | [#12](https://github.com/jamescjfoundation/open-source-core/issues/12) |
| C12 | EPUB import (known core gap) | Core | v1.0 (IMP-9) | [#13](https://github.com/jamescjfoundation/open-source-core/issues/13) |
| C13 | Entries: typed records with field templates, links, backlinks and a name list, stored as Markdown files with front matter (the story bible's general part, section 5.6) | Build in app first, `core-candidate`; promote when Script uses it | M4.1; promoted in R4 (core 0.1.4, `jcj-entries`; the entry file format stays here until Script shares folders) | [#14](https://github.com/jamescjfoundation/open-source-core/issues/14) |

"Build in app first" items are marked `// core-candidate:` in code with a core issue labelled
`core-candidate`, and move once they work and a second app wants them.

### 5.5 The stitched view (C8)

Reading and editing a chapter, an act or any selection of documents as one continuous text is the
hardest new piece, so its design is fixed here:

- Opening a stitched view builds one `Document` from the selected items in binder order. Each
  item's paragraphs carry `manuscript.source` (the item's id) and a divider paragraph with the
  item's title separates items; dividers are read-only.
- One engine session edits the combined document, with its own undo history.
- On every save point (auto-save, explicit save, closing the view, switching away) the combined
  document is split back by `manuscript.source` and each changed item is written. A paragraph typed
  at a boundary belongs to the item above it. Deleting a whole item's text never deletes the item.
- While a stitched view holds an item, that item opens read-only elsewhere, so there is one editing
  session per item at a time.
- Built for size: only the visible region is laid out; a 200,000-word stitched view must scroll and
  type within the frame budget of section 11.

### 5.6 The story bible and Wordsmith Script

Decided (decision to be written in M4.1): the bible's general part becomes a core crate, built
here first.

- **Built here first, in a general shape.** In M4 the bible is written as two layers in this
  repository: a general entries layer (records with an id, a type, fields from a template, a
  Markdown body, links, backlinks, and the list of names and aliases spelling learns from) marked
  `// core-candidate:` with core issue C13; and Manuscript's own layer on top (the Story Bible
  area, the entry types Character, Place and so on, linking from scenes, the inspector). The
  general layer names no manuscript concept: entry types and templates are data, not code.
- **Promoted to the core when Script starts using it** (R4), as a `jcj-` crate, by the core's rule
  "a second app has a written plan to use it". Script's characters then read and write the same
  entries.
- **Why not a crate of this repository that Script depends on:** Script would then wait on
  Manuscript's releases to change its own characters, and two apps would write data only one of
  them owns. The core exists to hold exactly what several apps share; Book reading the project is
  different, because Book only reads (section 9.2).
- **Sharing in practice:** a bible is a folder of entry files. A project normally keeps its own
  (`bible/`); a series or an adaptation points several projects, Manuscript or Script, at one
  bible folder, with the same outside-change detection as section 6.2.

### 5.7 Screenplays inside a project

A project can hold screenplays, for adaptations and for books that contain scripts:

- A screenplay is a binder item of kind `screenplay`, stored as `scripts/<id>.wss` in Wordsmith
  Script's own format, unchanged.
- Manuscript has no screenplay rules (they are Script's, and code is never copied between app
  repositories). It shows the screenplay read-only as its text, counts its words, lists it on the
  corkboard and in the outline with synopsis, label and status, and opens it in Wordsmith Script
  for editing. Script saves the file in place; Manuscript's outside-change detection reloads it.
- Screenplays are left out of manuscript exports and the book view unless a later decision adds
  them.
- If a richer preview is wanted (pages as Script prints them), Script's renderer must first reach
  Manuscript through the core, by the core's rules; until then the text view stands.

## 6. Data format

Decided here; specified in `docs/format.md` (step M1.1). Script made the same choice for the same
reason (Script decision 0044: `.wss` stays a readable text format rather than a zip container).

### 6.1 Requirements

1. Plain text and diff-friendly: every document is a separate text file a person can read without
   the app; a change to one scene changes one file.
2. Lossless for the writer's words, proven by tests (section 11).
3. Semantic styling only: "italic", "scene break", "chapter heading"; never fonts or sizes.
4. Stable identifiers for every binder item and bible entry, surviving renames and moves.
5. A schema version; every older version is read and migrated, a newer one is refused clearly.
6. Bounded, safe parsing: hostile or damaged files fail cleanly with an error, never a crash.

### 6.2 Layout

A project is a folder. On macOS it is registered as a package so the system shows it as one file; on
Windows and Linux the user opens `project.toml` inside it.

```text
My Novel.wsm/                   extension: `.wsm` unless the extension check (M0.2) finds a conflict
  project.toml                  schema version, title, author, language, settings, label and
                                status lists, keyword list, custom field definitions, targets,
                                export presets, document-set rules
  binder.toml                   the tree: ids, kinds, roles, titles, order, include-in-export,
                                label, status, icon; per area: manuscript, bible, research,
                                notes, trash
  text/<id>.md                  one file per text item (manuscript scenes, notes, research notes)
  bible/<id>.md                 one file per bible entry: fields in front matter, body below
  media/<id>.<ext>              research images and PDFs, the cover, pictures in the text
  scripts/<id>.wss              screenplays, in Wordsmith Script's format (v1.0, SCR-1)
  snapshots/<id>/<UTC time>.md  named, dated copies of one item's text
  drafts/<draft id>/binder.toml later drafts of the manuscript (v0.3, DRF-1)
  .recovery/                    unsaved work while the app runs; removed after a good save
```

- **Ids** are UUID version 7 (time-ordered) in lowercase text; files are named by id so renames
  and moves never touch them.
- **Single-file form**: "Save as one file" and backups write a deterministic zip of the folder
  (`.wsmz`): `mimetype` first and stored, fixed entry times, sorted entries, no `.recovery/`, so
  the same project gives the same bytes. Opening a `.wsmz` unpacks into a folder the user chooses.
  Reading follows the core's container rules: entries read one by one into memory, sizes and
  counts capped, never extracted blindly.
- **Saving** writes only the files that changed, each one atomically through `jcj-project`. The
  binder is written last, so an interrupted save leaves the previous tree pointing at whole files.
- **Outside changes** (a cloud folder, a version-control checkout) are detected by file time and
  content hash when the app regains focus; a changed file not open for editing is reloaded, one
  open with unsaved edits gets a conflict copy beside it and a message. Never a silent overwrite.
- **Limits**: maximum file sizes, item counts, tree depth and front matter size are constants in
  `manuscript-format` with tests that feed files beyond each limit.

### 6.3 The text dialect

Documents are written in a strict, documented subset of CommonMark with a small set of extensions
("Manuscript Markdown", specified in `docs/format.md`). Markdown is chosen over alternatives for
its reach: any text editor, any version-control viewer and many notes tools read it, and bible
entries stay compatible with Markdown notes folders.

| Content | Written as |
| --- | --- |
| Paragraphs, italic, bold | CommonMark (`*italic*`, `**bold**`); one paragraph per line, blank line between |
| Small caps, strikethrough, language span | `[text]{.smallcaps}`, `~~text~~`, `[text]{lang=fr}` |
| Scene break | A line holding only `***`; its printed symbol is an export setting |
| Headings inside a document | `#`..`###`, used only where an item's text needs them (front matter pages) |
| Block quote, verse, aside (letter, note, message) | `>` quote; fenced blocks `::: verse`, `::: aside letter` |
| Notes (footnote or endnote decided at export or in Book) | `[^n]` with the note text at the end of the file |
| Comments anchored to text | `[text]{.comment id=..}` with the comment in front matter `comments:` |
| Link to a binder item or bible entry | `[text](wsm:<id>)` |
| Images | `![alt](media/<id>.png){role=inline}`; the role is `inline`, `wide` or `page` |
| Raw block for content the dialect cannot express | A fenced code block whose info string is `raw` and the target format |

- **Front matter** (TOML between `+++` lines) holds per-item metadata: synopsis, notes reference,
  label, status, keywords, linked bible entries, custom fields, targets, comments.
- **Round trip rule**: for every document the editor can produce, model → text → model is exact,
  and text → model → text is byte-identical for text the writer did not change. The editor offers
  only styles the dialect can hold, so nothing is lost on save.
- **Parser**: `pulldown-cmark` (MIT) for CommonMark, with the extensions parsed by our own code in
  `manuscript-format`. If `pulldown-cmark` does not give exact source positions for the round-trip
  rule, the decision in M1.1 records the alternative chosen.
- **Bible entries** use the same dialect; `[[Entry name]]` links are read on import and written as
  `wsm:` links, with the display text kept.

### 6.4 Model

`manuscript-model` holds the binder tree (items with id, kind, role, title, children, metadata),
the project settings, bible entries and their templates, and the link index (which items link to
which entries, built from `wsm:` links and manual links). A text item's body is a `jcj_doc::Document`
while open; Manuscript's own paragraph data uses typed keys (`manuscript.source`,
`manuscript.scene_break`, `manuscript.aside`, ...) and run data uses marks (`manuscript.link`,
`manuscript.lang`). Roles (part, chapter, scene, front matter, back matter) live on binder items,
never in the text, so exports number and title things without manual formatting.

## 7. Features by release

Releases: **0.1** = first usable release; **0.2** and **0.3** = complete for daily long-form work;
**1.0** = series, drafts, sharing with Script and more. "Core" marks features that need a core
change from section 5.4.

### Project and binder

| Id | Feature | What it does | Release |
| --- | --- | --- | --- |
| BND-1 | Binder | Tree of folders and documents in a side panel; drag to reorder and nest; fixed top-level areas: Manuscript, Story Bible, Research, Notes, Trash | 0.1 |
| BND-2 | Roles | Each manuscript item is a part, chapter, scene, front matter or back matter; roles drive export headings, numbering and scene breaks | 0.1 |
| BND-3 | Include in export | Per-item switch to leave an item out of exports without deleting it | 0.1 |
| BND-4 | Templates | Novel and short story collection at 0.1; non-fiction at 0.2; user-made templates at 0.2 (C1) | 0.1 / 0.2 |
| BND-5 | Trash | Deleted items go to Trash; emptied only on request, with confirmation | 0.1 |
| BND-6 | Research media | Images, PDFs and web links stored in Research, shown in the editor area | 0.2 |
| BND-7 | Bookmarks | Quick links to items from anywhere in the project | 0.2 |
| BND-8 | Series | Several book projects grouped, sharing one bible | 1.0 |

### Editor

| Id | Feature | What it does | Release |
| --- | --- | --- | --- |
| ED-1 | Rich text with manuscript styles | Paragraph styles (body, first paragraph, block quote, verse, aside, scene break) and character styles (italic, bold, small caps, strikethrough, language) | 0.1 |
| ED-2 | Stitched view | Edit a chapter, act or any selection as one text (section 5.5, C8) | 0.1 |
| ED-3 | Focus mode | Full-screen text only, typewriter scrolling; dimmed surrounding paragraphs at 0.2 | 0.1 / 0.2 |
| ED-4 | Find and replace | In the document, the selection or the whole project; regular expressions; whole word and case | 0.1 |
| ED-5 | Smart typing | Curly quotes, dashes and ellipsis per language | 0.1 |
| ED-6 | Split editor | Two documents side by side or stacked (C3) | 0.2 |
| ED-7 | Comments and notes | Comments anchored to text and notes in the inspector, exported correctly | 0.2 |
| ED-8 | Page view | The text as standard manuscript pages with a page count | 0.2 |
| ED-9 | Grammar hints | Offline grammar and style hints, off by default (C10) | 0.2 |
| ED-10 | Inline links | Link text to an item or bible entry; hover shows a preview | 0.1 |
| ED-11 | Spelling | As you type, with the project word list learning bible names | 0.1 |

### Planning views

| Id | Feature | What it does | Release |
| --- | --- | --- | --- |
| PLN-1 | Corkboard | Cards for a folder's children: title, synopsis, label color, status stamp; drag to reorder; new cards become documents | 0.1 |
| PLN-2 | Outline | Table of the binder: title and synopsis, label, status, words, target, point of view, keywords, custom fields; sortable, editable in place (C6) | 0.1 |
| PLN-3 | Freeform board | Cards anywhere on a canvas with lines between them | 0.2 |
| PLN-4 | Thread lanes | Plot threads or point-of-view characters as lanes, scenes as cards in story order (C11) | 1.0 |
| PLN-5 | Timeline | Scenes on an in-story calendar from scene dates | 1.0 |

### Inspector and metadata

| Id | Feature | What it does | Release |
| --- | --- | --- | --- |
| INS-1 | Synopsis and notes | Short summary per item (on its card and in the outline); notes per item and per project | 0.1 |
| INS-2 | Label and status | One color label and one status per item; both lists editable per project | 0.1 |
| INS-3 | Keywords | Many colored keywords per item; filter and search by them | 0.1 |
| INS-4 | Linked bible entries | Which entries appear in a scene; set by hand at 0.1, suggested from the text at 1.0 | 0.1 / 1.0 |
| INS-5 | Snapshots | Take and restore at 0.1; compare word by word at 0.2 | 0.1 / 0.2 |
| INS-6 | Custom fields | Text, date, checkbox and list fields (point of view, scene date, location) shown in the outline | 0.2 |

### Story bible

| Id | Feature | What it does | Release |
| --- | --- | --- | --- |
| BIB-1 | Entry types | Character, place, object, group, lore, plot thread; each an entry with a field template | 0.1 |
| BIB-2 | Templates | Default sheets at 0.1; user-made templates with text, list, image and link fields at 0.2 | 0.1 / 0.2 |
| BIB-3 | Backlinks | Each entry lists every item that links to it or mentions its name | 0.1 |
| BIB-4 | Relationships | Typed links between entries (sibling of, rival of) | 1.0 |
| BIB-5 | Shared bible | One bible for a series, and shared with Wordsmith Script projects (section 5.6, C13) | 1.0 |

### Screenplays

| Id | Feature | What it does | Release |
| --- | --- | --- | --- |
| SCR-1 | Screenplays in a project | Screenplay items stored in Script's format, shown read-only with counts, card and outline metadata, opened in Wordsmith Script to edit (section 5.7) | 1.0 |

### Progress and revision

| Id | Feature | What it does | Release |
| --- | --- | --- | --- |
| PRG-1 | Counts | Live words and characters for document, selection, folder and manuscript | 0.1 |
| PRG-2 | Targets | Manuscript target with deadline and writing days; session target; per-item targets with progress in the outline (C2) | 0.1 |
| PRG-3 | Writing history | Words written per day, kept locally, shown as a calendar | 0.2 |
| PRG-4 | Revision passes | Edits marked in a color per pass (`jcj-features` revisions) | 0.2 |
| DRF-1 | Drafts | Several drafts of the manuscript in one project; start a new draft from the current one; compare scene by scene; keep both | 0.3 |

### Search and organization

| Id | Feature | What it does | Release |
| --- | --- | --- | --- |
| SRC-1 | Project search | Full-text search across text, notes, synopses and bible entries, results in the binder area (C7) | 0.1 |
| SRC-2 | Saved searches and collections | Named searches that update; hand-picked lists of items | 0.2 |
| SRC-3 | Filters | Filter binder, corkboard and outline by label, status, keyword, point of view | 0.1 |

### Files and safety

| Id | Feature | What it does | Release |
| --- | --- | --- | --- |
| FIL-1 | Auto-save | Saves continuously with atomic writes (core timing) | 0.1 |
| FIL-2 | Backups | Packed backups on close and on a schedule, with a number kept | 0.1 |
| FIL-3 | Recovery | Reopens unsaved work after a crash | 0.1 |
| FIL-4 | Safe in synced folders | Outside changes detected; conflict copies, never silent overwrite | 0.2 |
| FIL-5 | Sync between devices | Conflict-free merging (CRDT) | after 1.0 |

### Access and languages

| Id | Feature | What it does | Release |
| --- | --- | --- | --- |
| ACC-1 | Themes | Light, dark and sepia; editor font and width separate from export | 0.1 |
| ACC-2 | Keyboard | Every command reachable by keyboard; keys editable (core) | 0.1 |
| ACC-3 | Screen readers | Binder, editor and views exposed through AccessKit | 0.2 |
| ACC-4 | Interface languages | Translations through the core's catalog | 0.3 |

### Automation

| Id | Feature | What it does | Release |
| --- | --- | --- | --- |
| AUT-1 | Command line | `convert`, `export`, `counts`, `verify` (word-for-word check), `report` | 0.1 |
| AUT-2 | Agent tools | Every Manuscript command as an agent tool (`jcj-agents`) | 0.1 |

## 8. Import

Every import produces a report: what was created, any formatting that could not be kept, and a
**word-for-word check** that the imported text matches the source text exactly. The originals can
be copied into Research on request. Imports go through `jcj-interchange` to a `jcj_doc::Document`;
Manuscript's code only maps that document to its structure.

| Id | Source | How | Release |
| --- | --- | --- | --- |
| IMP-1 | DOCX (including DOCX downloaded from online word processors) | Headings map to parts, chapters and scenes by rules the writer can change; scene-break lines (`***`, `#`, a centered symbol, an empty-looking styled paragraph) become scene breaks; italic, bold, notes and comments kept | 0.1 |
| IMP-2 | A set of documents | Pick files or a folder; each file becomes an item; name rules (editable, saved in `project.toml`) place each by role: "(Plan)" into Notes, "(Rough Work)" into Research, a draft into the manuscript; several files of one draft are merged in order | 0.1 |
| IMP-3 | ODT | Through `jcj-interchange` | 0.1 |
| IMP-4 | RTF | Through `jcj-interchange` | 0.1 |
| IMP-5 | Markdown and plain text | Split by heading or by a chosen separator | 0.1 |
| IMP-6 | Markdown notes folder | Each note becomes a bible entry; front matter becomes fields; `[[links]]` kept | 0.1 |
| IMP-7 | HTML | Web drafts and serials | 0.2 |
| IMP-8 | OPML | Outline into binder and synopses | 0.2 |
| IMP-9 | EPUB | Recover a book as an editable manuscript (C12) | 1.0 |

Messy sources (fake headings made of bold text, manual page breaks, odd fonts, inconsistent scene
breaks) are handled by a preview step: the writer sees the proposed binder, changes the rules, and
then imports. Splitting and merging items afterwards is always possible.

## 9. Export and the hand-off to Wordsmith Book

### 9.1 Exports Manuscript makes itself

| Id | Format | Use | Release |
| --- | --- | --- | --- |
| EXP-1 | DOCX, standard manuscript format | Submitting to agents, editors and publishers: 12 pt serif or monospaced font, double spaced, 1-inch margins, header with surname, short title and page number, chapters starting a third down the page, scene-break marks, word count on the title page | 0.1 |
| EXP-2 | PDF, standard manuscript format | The same layout, through `jcj-layout` and `jcj-output` | 0.1 |
| EXP-3 | ODT, RTF | Other word processors | 0.1 |
| EXP-4 | Markdown, plain text, HTML | Web and other tools | 0.1 |
| EXP-5 | Book package | The hand-off to Wordsmith Book (9.2) | 0.1 |
| EXP-6 | EPUB reading copy | For beta readers; not a designed book (core EPUB writer) | 0.2 |
| EXP-7 | Through an external converter | Run a document converter the user installed as a separate program for other formats; never bundled or linked | 1.0 |

Export options: which items to include; how parts, chapters and scenes are titled and numbered;
the scene-break symbol; front and back matter; replace-text rules; saved presets. Export never
changes the project. The standard manuscript layout and its sources are specified in
`docs/SPEC.md` from public submission guides.

### 9.2 Hand-off to Wordsmith Book

Manuscript writes the words and structure; Book designs and typesets them. The contract, agreed
with the Wordsmith Book plan and written up as `docs/book-handoff.md` in step M6.3:

1. **Book opens the Manuscript project directly**, folder or `.wsmz`, through
   `wordsmith-manuscript-format`'s read-only `book` module, at a release tag of this repository.
   EPUB is only an export, never the hand-off. Book converts its own DOCX, Markdown and text
   imports into the same model, so it has one input model.
2. **What the book view gives**: metadata (title, subtitle, author and pen name, series name and
   number, language, ISBN per edition, publisher, copyright year, rights statement, description,
   subjects, keywords); structure (front matter: title page, copyright, dedication, epigraph,
   contents; parts, chapters; back matter: acknowledgements, about the author, also by), each with
   its role, title, optional subtitle and number; content as semantic text only (section 6.3,
   including notes stored only as "notes", asides, verse, language spans, images with alt text and
   role, raw blocks); assets (images, cover, chosen fonts with their licenses); and the stable id of
   every chapter and scene.
3. **Book never writes the Manuscript project.** Its own layer (designs, editions, store links,
   print-only and ebook-only flags, per-edition overrides) lives in a Book file keyed by
   Manuscript's ids.
4. **Fixes offered back.** A light fix made in Book is stored as a change against an id and
   offered to Manuscript, which shows it in an "Offered fixes" inbox for the writer to accept or
   reject (v0.2). If a later export conflicts with a Book fix, Book shows the difference and never
   overwrites silently.
5. **Versioned.** The book view has its own version; Book declares which versions it reads. A
   change that breaks Book is a minor release of this repository with a "What Book must do" note,
   as the core's changelog does for apps.
6. **Typesetting engine.** If Book adopts Typst, it comes into the core as `jcj-typeset` (core
   decision 0002, "Revisit when"); Manuscript may then move its standard manuscript PDF onto it.
   Until then Manuscript uses `jcj-layout` and `jcj-output`.

## 10. Milestones and steps

| Milestone | Release | Result | State |
| --- | --- | --- | --- |
| M0 Foundations | – | Repository from the template, rules, spec, brand | Done 2026-11-24 (spec awaits review) |
| M1 Project | 0.1 | Format, binder, saving, templates | Done 2026-11-24 |
| M2 Editor | 0.1 | Manuscript styles, stitched view, find, spelling, focus | Done 2026-11-24 |
| M3 Plan | 0.1 | Inspector, corkboard, outline, filters | Done 2026-11-24 |
| M4 Bible | 0.1 | Entries, templates, links, backlinks, notes-folder import | Done 2026-11-24 |
| M5 Import | 0.1 | All 0.1 imports, document sets, verification | Done 2026-11-24 |
| M6 Export | 0.1 | Standard manuscript DOCX and PDF, plain exports, book view | Done 2026-11-24 (contract awaits Book review) |
| M7 Progress | 0.1 | Counts, targets, snapshots, project search | Done 2026-11-24 |
| M8 Release | 0.1 | CLI, agent tools, guide, installers, release | Done 2026-11-24 (release awaits a maintainer's approval and tag) |
| R2 | 0.2 | Daily-work completeness | Done 2026-11-24 (release tag awaits approval) |
| R3 | 0.3 | Drafts and finish | Done 2026-11-24 (release tag awaits approval) |
| R4 | 1.0 | Beyond | Done 2026-11-24 except the web build, which waits for the core (release tag awaits approval) |

**The test of v0.1**: a public-domain novel of at least 150,000 words, split across six DOCX files
named as a document set, imports as a structured project with every word verified; it can be
reorganized on the corkboard and in the outline, edited in a stitched view of the whole
manuscript, and exported to standard manuscript DOCX and PDF and opened through the book view,
with every word still matching. This runs as an end-to-end test in CI (M8.4).

### M0 Foundations

**M0.1 `m0/template`**: start from the core's `template/` at the newest core release.
- Rename the package and apps (section 5.1), set `APP`, `NAME`, `SHIPPED`, `PRODUCT`; run
  `cargo xtask sync`, `install-hooks`, `ci`.
- Add LICENSE-MIT, LICENSE-APACHE, NOTICE (with the core's required credit), SECURITY.md,
  CONTRIBUTING.md, CODE_OF_CONDUCT.md, ATTRIBUTION.md, `.gitignore` with `.private/`.
- AGENTS.md: the template's text plus "Wordsmith Manuscript's own rules" (section 4, rules 2, 3
  and 7, and where each crate's code goes). CLAUDE.md: `@AGENTS.md`.
- Empty crates of section 5.1 with crate-root rules; the layers table in `xtask`.
- `docs/progress.md`, `docs/decisions/README.md`, decision 0001 "Built on open-source-core"
  (what comes from the core, the folder format choice of section 6, the crate layout).
- Done when: `cargo xtask ci` and `cargo xtask sync --check` pass locally and in CI; the empty app
  opens a window through `jcj-shell` with Manuscript's name.

**M0.2 `m0/name`** (can run in parallel with M0.3–M0.4): the name "Wordsmith Manuscript" is
confirmed. Check that `.wsm` and `.wsmz` are not registered to another common file type on macOS,
Windows or Linux. Done when: decision 0002 records the name and the extensions; if an extension
changes, this plan is updated.

**M0.3 `m0/spec`**: `docs/SPEC.md`, the behavior spec, from public manuscript-format guides and
general long-form writing practice, with our own design where they are silent. Each statement is
marked Verified (checked against two or more public sources), Sourced (one public source) or
Designed (ours). Sections: binder and roles, editor and styles, stitched view, planning views,
inspector and metadata, bible, targets and counts, snapshots, search, import rules, export and the
standard manuscript format (title page, header, margins, spacing, chapter starts, scene breaks,
word count rounding). Sources are listed by public URL. Done when: every 0.1 feature id in
section 7 has a spec section, and a person has reviewed it.

**M0.4 `m0/brand`**: `assets/brand/manuscript/tokens.json` in the shape of `jcj-theme`'s
`TOKENS.md` (light, dark, extra-dark, plus a sepia editor surface), the mark and app icons in the
suite's brand style, fonts chosen from OFL fonts already bundled or added with licenses
(`ATTRIBUTION.md`, `cargo xtask assets`). Done when: the app window uses the tokens and icons, and
`cargo xtask assets` passes.

**M0.5 `m0/fixtures`**: `tests/fixtures/` and `docs/fixtures.md`. Public-domain novels as text
(at least one over 300,000 words for scale), and DOCX, ODT and RTF versions saved by at least two
different word processors (not by this app), recorded with source, edition and tool. A six-file
document set of one novel for the v0.1 test. Done when: every fixture's provenance is recorded and
`cargo xtask privacy` passes.
- Changed 2026-11-24: the build machine has no word processor, so the DOCX, ODT and RTF fixtures
  are first written by a small standard-library script (`tools/make-fixtures.py`, not this app);
  versions saved by two word processors are added by a person who has them, as `docs/fixtures.md`
  describes. Importers are tested on both once they exist.

**M0.6 `m0/core-issues`** (no branch needed; can run in parallel): open one core issue for each
change C1 to C13 in section 5.4, each describing the general need without Manuscript's features,
labelled `core-candidate` where the work starts in this app. Link each issue from section 5.4 in
the same pull request that next touches this plan. Done when: every row of 5.4 has an issue link.

### M1 Project

**M1.1 `m1/format`**: `manuscript-model` (binder tree, items, roles, metadata, settings) and
`manuscript-format` (read and write the folder of section 6.2, the text dialect of 6.3, schema
version 1, limits). `docs/format.md` specifies every file and the dialect with examples. Decision
on the parser (6.3).
- Done when: round-trip tests pass for every construct in the dialect (model → text → model
  exact; unchanged text byte-identical); property tests generate random documents and binders;
  hostile-input tests hit every limit; a project saved twice gives the same bytes; a version-2
  project is refused with a clear error.

**M1.2 `m1/save`**: saving through `jcj-project`'s atomic writes (changed files only, binder last),
auto-save on the core's `AutoSave` timing driven by `jcj-shell`, recovery in `.recovery/`, packed
backups (`.wsmz`) on close and on a schedule with a number kept, Save as one file, open `.wsmz`.
- Done when: crash-injection tests (stop after each file write) always reopen to the last good
  save or the recovered work; packing is deterministic; a damaged `.wsmz` is refused cleanly.

**M1.3 `m1/binder`**: the binder panel: tree with areas, drag to reorder and nest, roles, include
in export, rename, new item and folder, move to Trash, empty Trash with confirmation. All changes
are commands (undoable, agent-callable). The tree widget is built here as a `core-candidate` (C5).
- Done when: binder operations are tested at the model level and through the egui test harness;
  1,000 items stay responsive.

**M1.4 `m1/templates`**: New project from Novel and Short story collection templates; the start
window and template chooser (C1, promoted from Script).
- Needs: C1 released in the core.
- Done when: both templates create valid projects that pass the format tests.

### M2 Editor

**M2.1 `m2/styles`**: Manuscript's paragraph and character styles on `jcj-doc`, keys in the
`manuscript.` namespace, the ribbon's Write tab, the editor panel for one item.
- Done when: every style round-trips through the format; styles not in the dialect cannot be
  applied.

**M2.2 `m2/typing`**: smart quotes, dashes and ellipsis per project language; scene-break insert;
counts in the status bar (document and selection).

**M2.3 `m2/stitched-view`**: the stitched view of section 5.5 (C8, `core-candidate`).
- Done when: edits across boundaries land in the right items; undo works across the whole view;
  split-back is tested with random edits (property test: words in = words out); a 200,000-word
  view meets the section 11 frame budget.

**M2.4 `m2/find-spelling`**: find and replace in document, selection and project (core find,
extended to many items); spelling with the project word list; inline links (ED-10) with hover
preview.

**M2.5 `m2/focus`**: focus mode with typewriter scrolling; light, dark and sepia themes (ACC-1).

### M3 Plan

**M3.1 `m3/inspector`**: inspector panel: synopsis, notes, label, status, keywords, linked bible
entries (manual); project label, status and keyword lists editable.

**M3.2 `m3/outline`**: outline view with the columns of PLN-2 (custom fields wait for 0.2), sort,
edit in place, drag to reorder; table widget as a `core-candidate` (C6).

**M3.3 `m3/corkboard`**: corkboard for any folder through `jcj-board`; card title, synopsis,
label color, status stamp; drag to reorder changes the binder; new cards become items.

**M3.4 `m3/filters`**: filters by label, status, keyword and point of view across binder,
corkboard and outline (SRC-3).

- Milestone done when: reordering in any of binder, corkboard or outline shows at once in the
  other two, and every action is one undoable command.

### M4 Bible

**M4.1 `m4/entries`**: the general entries layer (C13, `core-candidate`, section 5.6) and, on it,
the bible area, entry types and default templates (BIB-1, BIB-2), entry editor with fields and
body. Decision: the bible's two layers and the plan to promote the general one.
- Done when: the general layer has its own tests and no manuscript words in its interface.

**M4.2 `m4/links`**: links from text to entries, backlinks (by link and by name mention), the
project word list fed from entry names and aliases.

**M4.3 `m4/notes-import`**: Markdown notes folder import (IMP-6), with the import report.

- Milestone done when: a written-for-test notes folder of 200 entries with `[[links]]` imports
  with every link resolved or reported.

### M5 Import

**M5.1 `m5/verify`**: the word-for-word verifier (C9, `core-candidate`): extracts the words of a
source and of the result in reading order, compares them exactly, and reports the first difference
with context. Used by every importer and exporter and by `manuscript-cli verify`.

**M5.2 `m5/docx-odt-rtf`**: IMP-1, IMP-3, IMP-4: mapping rules (headings to roles, scene-break
detection), the import preview, the report.

**M5.3 `m5/markdown-text`**: IMP-5.

**M5.4 `m5/document-sets`**: IMP-2: name rules, merge of several files of one draft, placement by
role; the rules editable and saved in the project.

- Milestone done when: every fixture of M0.5 imports with a passing word-for-word check, and the
  six-file set becomes one ordered manuscript.

### M6 Export

**M6.1 `m6/plain-exports`**: EXP-3, EXP-4 through `jcj-interchange`, with export options and
presets.

**M6.2 `m6/manuscript-format`**: EXP-1 and EXP-2: the standard manuscript layout as a
`PaginationPass`, headers through `jcj-features` with a `FieldSource` for surname and short title,
title page with rounded word count. Golden PDF and DOCX files reviewed and recorded in
`docs/golden.md`.
- Needs: C4 for the PDF's producer name.

**M6.3 `m6/book-view`**: the read-only `book` module in `manuscript-format` and
`docs/book-handoff.md` (section 9.2); EXP-5 writes a `.wsmz` with the book metadata filled in.
- Done when: a test reads the v0.1 test project through the book view only and reproduces every
  word, role and id; a person from the Book side has reviewed the contract.

### M7 Progress

**M7.1 `m7/targets`**: PRG-1, PRG-2 on the promoted statistics and targets (C2); per-item targets in
the outline.
- Needs: C2 released in the core.

**M7.2 `m7/snapshots`**: take, name, list and restore snapshots (INS-5); a snapshot before every
Replace All across the project.

**M7.3 `m7/search`**: project search (SRC-1) with an index built in the background
(`core-candidate`, C7; Tantivy if a plain in-memory index is too slow at the scale of section 11).

### M8 Release

**M8.1 `m8/cli`**: `manuscript-cli` with `convert`, `export`, `counts`, `verify`, `report`
(AUT-1).

**M8.2 `m8/agents`**: every Manuscript command as an agent tool (AUT-2), with tests that drive a
project through the tools.

**M8.3 `m8/guide`**: user guide in `docs/guide/` (getting started, the binder, writing, planning,
the bible, import, export, files and backups, keys, command line and agents).

**M8.4 `m8/release`**: installers for macOS (with the package type registered), Windows and Linux;
signed and attested releases through the core's release workflow; update checks from this
repository's releases; the v0.1 end-to-end test in CI; CHANGELOG.md.
- Done when: the test of v0.1 passes in CI on all three platforms and a person approves the
  release.

### R2 (v0.2): daily-work completeness

Steps, one branch each, in this order: split editor (ED-6, C3); comments and notes in the inspector
(ED-7); custom fields (INS-6); snapshot compare (INS-5, `jcj-features` compare with scene
grouping); saved searches and collections (SRC-2); bookmarks (BND-7); research media (BND-6);
freeform board (PLN-3); page view (ED-8); revision passes (PRG-4); writing history (PRG-3);
grammar hints (ED-9, C10); custom bible templates and user-made project templates (BIB-2,
BND-4); HTML and OPML import (IMP-7, IMP-8); EPUB reading copy (EXP-6); safe in synced folders
(FIL-4); screen readers (ACC-3); the "Offered fixes" inbox from Book (9.2 item 4).

### R3 (v0.3): drafts and finish

Drafts (DRF-1): format version 3 with `drafts/` (version 2 went to revision pass marks in R2,
decision 0005), migration from versions 1 and 2 with a fixture of each version, new draft from current, compare scene by scene, choose which draft exports; document-set
import places later drafts as drafts; interface languages (ACC-4); polish from 0.1 and 0.2 use.

### R4 (v1.0): beyond

Series with a shared bible (BND-8, BIB-5); the entries layer promoted to the core (C13) and the
bible shared with Wordsmith Script projects (section 5.6); screenplays in a project (SCR-1,
section 5.7); relationships (BIB-4);
thread lanes (PLN-4, C11); timeline (PLN-5); suggested bible links (INS-4); EPUB import (IMP-9,
C12); export through an external converter (EXP-7); a web build if the core supports one.

## 11. Testing and quality gates

| Gate | What | When it runs |
| --- | --- | --- |
| The core's gate | `cargo xtask ci`: format, lint, tests, licenses and attribution, layers, privacy, sync check | Every push (`--changed`), full nightly and before release |
| Format round trips | Every dialect construct and random documents and binders, both directions | Every push |
| Word-for-word | Every importer and exporter on every fixture; failures block the merge | Every push |
| Golden exports | Standard manuscript PDF text and page breaks, DOCX document structure, compared with reviewed files | Every push |
| Data safety | Crash injection during save; damaged and hostile files (sizes, depth, counts, bad UTF-8, broken zip); outside-change detection | Every push; fuzzing nightly |
| Scale | A 300,000-word project with 1,000 scenes: open under 2 s, project search under 200 ms, stitched view of the whole manuscript typing at under 16 ms median per frame on the CI machine (recorded in `docs/performance.md`) | Nightly; any regression over 20 % fails |
| Interface | egui test harness for binder, corkboard, outline, inspector and stitched view interactions | Every push |
| End to end | The test of v0.1 (section 10) | Nightly from M5; every push from M8 |

Private manuscripts may be run through the same tests locally from `.private/`; their results are
never committed.

## 12. Risks

| Risk | Impact | Mitigation |
| --- | --- | --- |
| Losing or changing a writer's words | Fatal for trust | Atomic saves, binder written last, backups, recovery, word-for-word tests on every import and export, no silent clean-up |
| Stitched view across many documents is hard (undo, boundaries, size) | Slow progress, subtle data bugs | Design fixed in section 5.5; property tests that words in equal words out; scale tests from M2 |
| Rich-text editing feel in egui (selection, input methods, bidirectional text, accessibility) | Rough feel | Use the core's engine and editor work; test with long real-length drafts from M2; report gaps as core issues |
| Core changes Manuscript needs wait on core releases | Milestones blocked | Section 5.4 lists them up front; open their core issues in M0; build "app first" items here as `core-candidate` |
| Markdown dialect cannot express something the editor allows | Silent loss | The editor offers only what the dialect holds; raw blocks as an escape hatch; round-trip property tests |
| Messy word-processor documents (fake headings, page breaks, inconsistent scene breaks) | Messy structure after import | Editable rules, preview before import, easy split and merge |
| Long projects slow to open, search or scroll | Unusable for the main user | Visible-region layout, background index, scale gates in CI |
| Synced folders create conflicts | Duplicate or lost files | One file per document, change detection, conflict copies, never overwrite |
| GPL or other incompatible licenses enter the build | License violation | Core license allowlist in CI; GPL programs only as optional separate programs |
| Reused upstream code is young | Hidden bugs | The core's contract tests and pins; report and fix upstream through the core's forks |
| Manuscript and Script drift apart | Two apps that feel unrelated | The shared core and brand style; the shared bible at 1.0 |
| Book's needs change the hand-off | Rework | The book view is versioned (9.2 item 5) and reviewed by the Book side at M6.3 |

## 13. Open questions

Each needs a person's answer; record it as a decision and update this plan.

None open. Answered on 2026-11-24:

1. The name "Wordsmith Manuscript" is confirmed (M0.2 still checks the file extensions).
2. A project can hold screenplays; Script edits them (section 5.7, SCR-1).
3. The shared bible's general part becomes a core crate, built here first (section 5.6, C13).

## 14. Third-party building blocks

Besides open-source-core and what it brings in. Licenses are re-checked before each is added;
`cargo deny` enforces the allowlist.

| Project | Use | License | When |
| --- | --- | --- | --- |
| [pulldown-cmark](https://crates.io/crates/pulldown-cmark) | Not used: the dialect has its own reader (decision 0003) | MIT | – |
| [toml](https://crates.io/crates/toml) | `project.toml`, `binder.toml`, front matter (already in the core's lock file) | MIT OR Apache-2.0 | M1.1 |
| [uuid](https://crates.io/crates/uuid) | Version 7 ids (already in the core's lock file) | MIT OR Apache-2.0 | M1.1 |
| [zip](https://crates.io/crates/zip) | `.wsmz` packing, at the core's version and with its reading rules | MIT | M1.2 |
| [proptest](https://crates.io/crates/proptest) | Property tests for round trips and the stitched view (dev only) | MIT OR Apache-2.0 | M1.1 |
| [Tantivy](https://crates.io/crates/tantivy) | Project search index, if a plain index is too slow | MIT | M7.3 if needed |
| [Harper](https://crates.io/crates/harper-core) | Offline grammar hints, through `jcj-proof` (C10) | Apache-2.0 | R2 |
| [Loro](https://crates.io/crates/loro) or [Automerge](https://crates.io/crates/automerge) | Sync between devices, later; the format keeps one file per document so adding one stays possible | MIT | After 1.0 |
