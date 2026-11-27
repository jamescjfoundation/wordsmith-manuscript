# Changelog

What changed in each release of Wordsmith Manuscript, newest first. `## Unreleased` holds what
the next release will bring; when that release is cut, the section takes its version and date
(`## 0.9.0 (2026-12-01)`), and the next `## Unreleased` opens above it. The book view's own
versions, and what Wordsmith Book must do when one changes, are in `docs/book-handoff.md`.

The first public release is 0.9.0, a preview; 1.0.0 follows it (decision 0009). Nothing was
published before it: the versions under "Before the first release", at the end, were the stages
of the build plan, each prepared as a release and none released.

## 0.9.0 (2026-11-27)

The first public release of Wordsmith Manuscript, 0.9.0: a preview of 1.0.0, for trying the app,
its downloads and its update check. Everything the app does is in it. The user guide is at
<https://wordsmithand.ink/wiki/manuscript/>, and in `docs/guide/` of the repository.

### Getting it

- One download for each system, and a list of their checksums:
  `wordsmith-manuscript-0.9.0-linux-x86_64.tar.gz` (Linux, 64-bit Intel or AMD; needs the C
  library of Ubuntu 24.04 or newer), `wordsmith-manuscript-0.9.0-macos-aarch64.dmg` (a Mac with
  Apple silicon, macOS 11 or later), `wordsmith-manuscript-0.9.0-windows-x64-portable.zip`
  (Windows, 64-bit; nothing is installed system-wide) and `SHA256SUMS.txt`.
- **The builds are not signed with a publisher's certificate yet.** macOS and Windows ask before
  they open the app the first time. On macOS: System Settings › Privacy & Security › Open
  Anyway. On Windows: More info, then Run anyway. The guide's "Installing and updating" page has
  the steps for each system, and how to check a download against its checksum and its build
  attestation.
- Each download holds the app (`wordsmith-manuscript`), the command line
  (`wordsmith-manuscript-cli`) and the license texts of everything they are built from. The
  Linux package has an install script that puts the app in your menu and registers the project
  file types for your account; the Windows package has a script that registers them.
- The app looks for a newer release when it starts, and File › About has Check for Updates. On
  Linux a copy installed under your home folder replaces itself and starts the new version; on
  macOS and Windows the app shows the release page. The check is the only request the app
  makes, and File › Preferences turns it off.

### What the app does

- **Projects.** A project is a folder of plain text files (`.wsm`): one Markdown file for each
  document and bible entry, with the binder and the settings beside them. A packed single file
  (`.wsmz`) is for sending, backups and Wordsmith Book. Saving writes only what changed, each
  file safely; unsaved work is kept for recovery; backups are made on close and every 30
  minutes; in a synced folder both versions are kept when both sides changed.
- **The binder.** Parts, chapters, scenes, front and back matter in one tree, with areas for the
  Story Bible, Research, Notes and a Trash. Collections, saved searches and bookmarks beside it.
- **Writing.** Manuscript styles only (body, block quote, verse, aside, headings, scene breaks;
  italic, bold, small caps, strikethrough, language). Smart quotes and dashes for the language.
  A chapter, a part or the whole book edited as one text. Two panes. The text as standard
  manuscript pages. Comments, revision passes in color, snapshots compared word by word, and
  drafts kept side by side. Find and replace across the project, spelling that knows the
  project's names, grammar hints, focus mode.
- **Planning.** The corkboard, the outline, a freeform board, thread lanes and a timeline of the
  story's own dates. Labels, statuses, keywords, points of view and your own fields, with
  filters across every view. Targets with deadlines and writing days, session targets, and a
  writing history kept on your computer.
- **The story bible.** Characters, places, objects, groups, lore and plot threads, your own
  entry types, relationships, links to the text and back, suggested links, and one bible shared
  by the books of a series.
- **Import.** DOCX, ODT, RTF, Markdown, HTML, web pages, plain text, OPML outlines, EPUB books,
  folders of Markdown notes, and a folder of drafts as one set. Every import is checked word
  for word.
- **Export.** A standard manuscript as DOCX and PDF; DOCX, ODT, RTF, Markdown, plain text, HTML;
  an EPUB reading copy; a book package for Wordsmith Book; other formats through a converter
  program you installed yourself. Every export is checked word for word.
- **Screenplays in a project.** Wordsmith Script files are kept as Script wrote them, shown
  read-only with their counts, and opened in Script to edit.
- **The interface** in English and Spanish, light, dark and sepia, named for screen readers
  throughout, with every command in the command palette and every key yours to change.

### The command line and agents

- `wordsmith-manuscript-cli` does the app's work without its window: `convert`, `export`,
  `counts`, `verify`, `report`, `run` and `commands`. It makes no network request.
- One convention with the other Wordsmith apps: a result on standard output as JSON; a reason
  on standard error; exit code 0 when it did what was asked, 1 when it could not, 2 when the
  command line was wrong and nothing was done, and 3 when `verify` ran and the texts differ.
  An unknown option is refused, never passed over.
- `wordsmith-manuscript-cli mcp <project>` serves every Manuscript command to an agent as a
  tool named `manuscript_` and the command's id. `--read-root <folder>` and
  `--write-root <folder>` keep the files an agent's commands name to folders you choose.
- `wordsmith-manuscript --version` and `wordsmith-manuscript-cli --version` print the
  program's name and version.

### For Wordsmith Book

- This release reads project formats 1 to 3 and writes format 3, and gives book view version 1
  (`docs/book-handoff.md`).

### Not yet

- Signed builds, and with them an update installed in place on Windows.
- A web version: the core has no web build.
- Native file dialogs: paths are typed on the File pages.

### If you built the app from source before this release

What changed since the last stage of the build plan was prepared (1.0.0, below):

- **The command-line program is `wordsmith-manuscript-cli`** (it was `manuscript-cli`), in its
  file name, its usage, its version line and the start of every message. Scripts and agent
  configurations that name the old program must be changed. Every program of a release is named
  after the app, so two Wordsmith apps installed in one folder share no name.
- **Release files have the names above.** The earlier, unused release workflow would have named
  the macOS and Windows files in a way the update check does not find. Releases are made by
  the shared workflow of open-source-core now
  ([core decision 0019](https://github.com/jamescjfoundation/open-source-core/blob/main/docs/decisions/0019-app-releases.md)),
  started by hand on `main`; a pushed tag starts nothing.
- **An installed update is started.** The app closes when an update is ready and starts the new
  version; it used to install one and carry on with the old.
- The macOS app's file types are named `org.jcjfoundation.manuscript.wsm` and `.wsmz`.
- The command line's convention (`docs/guide/command-line-and-agents.md`). What a script will
  notice:
  - `verify` ends with exit code 3 when the two texts differ (it was 2). Its answer is on
    standard output either way. 1 and 2 now mean the check did not run.
  - A wrong command line ends with 2 (it was 1) and the reason and the list of commands on
    standard error: an unknown command, a missing or extra argument, parameters that are not a
    JSON object.
  - An option the command does not have is refused. It was passed over without a word, with
    the argument after it, so a misspelled `--format` exported in the wrong format.
  - With no argument the program does nothing and ends with 2 (it printed the list and ended
    with 0). `--help` prints the list and ends with 0.
  - `-V` prints the version, as `--version` does.
- Agents (`wordsmith-manuscript-cli mcp`):
  - `--read-root <folder>` and `--write-root <folder>` keep the files an agent's commands name
    to folders you choose. The project you name is yours and is saved where it lives.
  - Each command says which of its parameters it reads and which it writes. A new series bible
    and an export through a converter are checked as files written (they were checked as files
    read).
  - With either option, the tool that runs a converter program is refused: a program cannot be
    kept to a folder.
  - The tools are Manuscript's own commands and nothing else. The general document tools
    (`execute`, `list_commands` and the rest), which worked on an empty document, are no longer
    listed.
  - An export is no longer marked as a tool that only reads: it writes a file.
- The interface language holds for the whole window. With Español chosen, the File page,
  Preferences, the dialogs and the command palette are in Spanish too; they stayed in English.
- The EPUB reading copy's notes are easier to use: a note's number in the text is a link to the
  note, the notes at the end of a chapter are a numbered list that reading systems know as
  endnotes, and "Notes" above them is a heading (it was a line in bold). They are numbered
  through the book, as before.

## Before the first release

Nothing in this section was published. 0.1.0, 0.2.0, 0.3.0 and 1.0.0 were the four stages of
the build plan (`docs/PLAN.md`), all built in November 2026. Each was written up as a release
that would go out once a maintainer approved it; none did, and no tag or release of them
exists. The first public release was then set at 0.9.0 (decision 0009). The notes are kept as
they were written, as the record of what each stage built. They name the command-line program
`manuscript-cli`, as it was then, and "0.1", "0.2" and "0.3" in them, in the guide's history and
in the code's comments are these stages, not versions anyone could download.

### 1.0.0 (prepared 2026-11-24, never released)

Beyond the single book. No format change: 1.0 reads and writes schema 3.

- Series: several books share one Story Bible kept in its own folder; what one book saves reaches
  the others; a book can leave with a copy.
- The story bible's general layer now comes from the core (`jcj-entries`, core 0.1.4), ready for
  Wordsmith Script's characters.
- Screenplays in a project: Wordsmith Script files kept as Script wrote them, shown read-only
  with their counts, opened in Script to edit. A script saved in a newer format than this
  version reads is refused with a message naming both formats, never shown with words that may
  be wrong; braces in a script of Script's first format are shown as typed.
- Relationships between bible entries, read from both sides.
- Thread lanes (plot threads or points of view) and a timeline of the story's own dates.
- Suggested bible links from the names a scene mentions.
- EPUB books come back as editable manuscripts.
- Export to other formats through a document converter you installed yourself.
- Not yet: a web version (the core has no web build).
- Later additions to this stage (the command line's convention, the folders an agent may use,
  one interface language for the whole window, the EPUB reading copy's notes) are under 0.9.0,
  the release they first appeared in.

### 0.3.0 (prepared 2026-11-24, never released)

Drafts and finish. **Projects saved by 0.3 use schema 3 and do not open in 0.2** (decision 0006);
0.1 and 0.2 projects open in 0.3 unchanged.

- Drafts: keep the manuscript as it is and carry on with a new draft; compare drafts chapter by
  chapter and scene by scene; write any draft again; choose the draft that exports (and that
  Wordsmith Book typesets). A document set's earlier drafts come in as drafts.
- The interface in Spanish as well as English, following the computer's language or chosen in
  the command palette; more languages can be added as catalogs.
- Polish: the inspector's and windows' texts translate too; lint clean-up.

### 0.2.0 (prepared 2026-11-24, never released)

Daily-work completeness. **Projects saved by 0.2 use schema 2 and do not open in 0.1**
(decision 0005); 0.1 projects open in 0.2 unchanged.

#### Writing

- Split the editor side by side or top and bottom; a document is editable in one pane at a time.
- Comments on selected words, written in the inspector, exported as real comments to DOCX, ODT and
  RTF when asked.
- Revision passes: what you type during a pass shows in its color until you clear it.
- The page view: the text as standard manuscript pages with the page count.
- Grammar hints, offline and off unless you turn them on.

#### Planning and finding

- Custom fields (text, date, checkbox, list) in the inspector and as outline columns.
- Compare a snapshot with the current text word by word.
- Collections: hand-picked lists and saved searches beside the binder. Bookmarks above it.
- The freeform board: cards anywhere, lines between them, cards that stand for documents.
- A writing history calendar, kept on this computer.

#### Story bible, templates, research

- Your own bible entry types; renaming a field keeps every value.
- Your own project templates (Save as Template).
- Pictures, PDFs and web links in Research, shown in the editor area.

#### Import, export, files

- Web pages (their article only) and OPML outlines import.
- An EPUB reading copy (notes as numbered notes at each chapter's end).
- Safe in synced folders: outside changes are read in, both versions kept when both sides
  changed, and nothing is written over unseen without a copy in `conflicts/`.
- The "Offered fixes" inbox for fixes Wordsmith Book offers back (`docs/book-handoff.md`).

#### Accessibility

- The editor and every view are named for screen readers.

### 0.1.0 (prepared 2026-11-24, never released)

The first usable release: plan, draft, revise and finish a novel in one project.

#### Projects and files

- A project is a folder of plain text files (`.wsm`): `project.toml`, `binder.toml`, one Markdown
  file per document and bible entry, snapshots and media beside them (`docs/format.md`, schema 1).
  A packed single file (`.wsmz`) for sending, backups and Wordsmith Book.
- Saving writes only changed files, each safely, the binder last; unsaved work is kept for
  recovery; backups on close and every 30 minutes, ten kept.
- Novel and Short story collection templates; a start window with templates and recent projects.

#### Writing

- The binder with five areas, roles (part, chapter, scene, front matter, back matter), include in
  export, drag and drop, the Trash.
- The editor with Manuscript's styles only (body, quote, verse, aside, headings, scene breaks;
  italic, bold, small caps, strikethrough, language), smart quotes, dashes and ellipses per
  language, plain paste, counts.
- Reading and editing a chapter, a part or the whole manuscript as one text.
- Find and replace in a document, a selection or the project; spelling with the project's words;
  links to items and bible entries; focus mode, typewriter scrolling, light, dark and sepia.

#### Planning

- The inspector (synopsis, notes, label, status, point of view, keywords, linked entries, target,
  snapshots), the corkboard, the outline (sortable, editable), filters across all three.
- The story bible: characters, places, objects, groups, lore and plot threads from templates, with
  aliases, fields, bodies and backlinks; a Markdown notes folder imported as entries.
- Targets with deadlines and writing days, session targets and sprints; snapshots; project search.

#### Import and export

- DOCX, ODT, RTF, Markdown, HTML and plain text imports mapped to parts, chapters and scenes, and
  sets of documents placed by the roles in their names; every import checked word for word.
- Standard manuscript DOCX and PDF; DOCX, ODT, RTF, Markdown, plain text and HTML; the book package
  for Wordsmith Book (book view version 1); every export checked word for word.

#### Automation

- `manuscript-cli`: convert, export, counts, verify, report, run, and every command as an agent
  tool (`manuscript-cli mcp`).

#### Known gaps

- The PDF's producer field names the upstream engine (core issue #5, C4).
- DOCX, ODT and RTF fixtures saved by two word processors are still to be added (`docs/fixtures.md`).
- Native file pickers are not offered by the core's shell yet: paths are typed on the File pages.
