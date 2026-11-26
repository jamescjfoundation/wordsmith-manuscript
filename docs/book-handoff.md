# The hand-off to Wordsmith Book

This is the contract between Wordsmith Manuscript and Wordsmith Book
([core decision 0015](https://github.com/jamescjfoundation/open-source-core/blob/main/docs/decisions/0015-apps-that-depend-on-apps.md),
[project decision 0004](https://github.com/jamescjfoundation/wordsmith/blob/main/project/decisions/0004-apps-that-depend-on-apps.md)).
Manuscript writes the words and structure; Book designs and typesets them. Book builds on two
crates of this repository, `wordsmith-manuscript-model` and `wordsmith-manuscript-format`, at a
tag ([Tags](#tags)).

Manuscript owns this file. It says what Book may rely on:

- [the items of the two crates Book may use](#what-book-may-use);
- [what the book view gives](#what-the-book-view-gives);
- [the file formats Book reads and writes through them, and their versions](#formats-and-versions).

Anything in the two crates that is not listed here may change without notice. Book never builds
on `wordsmith-manuscript-io`, `wordsmith-manuscript-ui` or the programs.

Book view version 1.

## How Book reads a manuscript

1. **Book opens the Manuscript project directly**: a project folder (`.wsm`) with
   `folder::Folder::open`, or a packed project (`.wsmz`) with `pack::read_packed`. Either gives
   a `Project`, and `book::from_project` gives its book view, a `book::Book`. EPUB is only an
   export, never the hand-off.
2. **Book never writes the writer's project.** Its own layer (designs, editions, store links,
   print-only and ebook-only flags, per-edition overrides) lives in Book's own project, keyed
   by Manuscript's ids.
3. **A source Book owns is the same model.** Book turns its own DOCX, Markdown and text imports
   into a `Project` and keeps it as a project folder inside its own project, written with
   `Folder::create` and `Folder::save`. So Book has one input model, and a source it owns can be
   opened in Manuscript.
4. **Fixes are offered back**, never written: [Offering fixes back](#offering-fixes-back).

## What Book may use

`model::` stands for `wordsmith_manuscript_model::` and `format::` for
`wordsmith_manuscript_format::`. The list is what Book uses today, checked against Book's code.

How an item is used is part of the contract:

- A struct marked **built field by field** is one Book writes out in full. A new field breaks
  Book.
- An enum marked **matched in full** is one Book handles variant by variant. A new variant
  breaks Book.
- For every other struct, the fields named are the ones Book may read or set. More may be added.

`crates/manuscript-format/tests/book_contract.rs` uses every item below the way Book does, so a
change that breaks one fails in this repository's gate. The same test fails when this list
names an item the test does not use, or the test uses one this list does not name.

<!-- BEGIN contract surface (crates/manuscript-format/tests/book_contract.rs reads the first column) -->

### The project and its binder

| Item | What Book relies on |
| --- | --- |
| `model::Project` | The fields `settings` (of which `settings.book`), `binder` and `texts`. It can be cloned and compared |
| `model::Project::new` | An empty project, for a source Book owns |
| `model::Project::text` | An item's text, when it has one |
| `model::Project::included_manuscript` | The ids of the manuscript items an export holds, in reading order. An item left out of export is not among them |
| `model::BookInfo` | The book's details as the writer set them: `title`, `subtitle`, `author` (the legal name), `pen_name`, `language`, `series`, `series_number`, `isbn` (by edition name), `publisher`, `copyright_year`, `rights`, `description`, `subjects`, `keywords`. It can be cloned and compared |
| `model::Binder::insert` | Put an item at a place. Refused with an error, never a panic |
| `model::Binder::get` | An item by id |
| `model::Binder::get_mut` | The same, to change it |
| `model::Binder::remove` | Take an item out, with what is under it |
| `model::Binder::rows_of` | An area's items in reading order, parents before children |
| `model::Row` | The field `item` |
| `model::Item` | The fields `id`, `kind`, `role`, `title`, `include` and `children` |
| `model::Item::with_id` | An item with a given id, kind and title, included in export |
| `model::Item::role` | The same item with a role |
| `model::ItemId` | An id that is copied, compared, ordered and hashed. It reads from its text form (`str::parse`) and prints as it |
| `model::ItemId::from_counter` | The same id for the same number every time, for fixtures and for ids Book makes |
| `model::ItemId::as_text` | The id as text: lowercase, hyphenated, 36 characters |
| `model::BadId` | Why a text is not an id, as a sentence |
| `model::ItemText` | The field `body`, an item's text |
| `model::ItemText::with_body` | An item's text with nothing else set |
| `model::Kind` | `Folder` and `Text` |
| `model::Area` | `Manuscript` and `Research` |
| `model::Place::end_of` | The end of an area |
| `model::Place::inside` | The end of an item's children |
| `model::Role` | **Matched in full**: `None`, `Part`, `Chapter`, `Scene`, `FrontMatter`, `BackMatter` |

### The text of an item

| Item | What Book relies on |
| --- | --- |
| `model::Text` | **Built field by field**: `blocks`, `notes`. It has an empty default |
| `model::Text::from_paragraphs` | Plain paragraphs as a text |
| `model::Text::plain_paragraphs` | The paragraphs as a reader sees them, without marks: the blocks', then the notes' |
| `model::Text::inline_runs` | Every paragraph's content in that same order |
| `model::Text::inline_runs_mut` | The same, to change it |
| `model::Text::normalize` | The canonical form: neighbouring pieces with the same marks are one, empty pieces are gone |
| `model::Block` | **Matched in full, each variant with every field**: `Para`, `Heading { level, content }`, `SceneBreak`, `Quote`, `Fenced { kind, paras }`, `Image { alt, src, role }`, `Raw { format, text }` |
| `model::Inline` | **Matched in full, each variant with every field**: `Span { text, marks }`, `NoteRef { label, marks }` |
| `model::Inline::plain` | A piece of text with no marks |
| `model::Inline::marked` | A piece of text with marks |
| `model::Inlines` | A paragraph's content: a list of `Inline` |
| `model::Marks` | **Built field by field**: `italic`, `bold`, `small_caps`, `strike`, `lang`, `link`, `comment`, `rev`. Its default is no mark |
| `model::Marks::italic` | Italic and nothing else |
| `model::Note` | **Built field by field**: `label`, `paras` |
| `model::Fence` | **Matched in full**: `Verse`, `Aside` with its optional kind |
| `model::ImageRole` | **Matched in full**: `Inline` (the default), `Wide`, `Page` |
| `model::ImageRole::key` | The role's word: `inline`, `wide`, `page` |
| `model::ImageRole::from_key` | The role a word names, if any |
| `model::text::plain` | What a reader sees of a paragraph: its words without their marks. A note reference adds nothing |
| `model::text::normalize_inlines` | The canonical form of one paragraph's content |
| `model::words::words` | The words of a text, by the one rule every word-for-word check in the project uses: split at white space, punctuation kept on its word, a piece with no letter or digit is no word |
| `model::series::relative` | The path from one folder to another as a project stores it, with `/` on every system |

### Reading and writing a project

| Item | What Book relies on |
| --- | --- |
| `format::SCHEMA` | The newest project schema this build reads, and the one it writes |
| `format::Error` | Every error prints as a sentence for the person at the screen. `Error::Newer { found, supported }` is a project saved by a newer Manuscript, with both schema numbers, so Book can word its own message. `Error::NotAProject` is a folder that holds no project |
| `format::folder::Folder` | An open project folder, kept to notice changes and to save again |
| `format::folder::Folder::open` | Read a project folder. It writes nothing |
| `format::folder::Folder::create` | Make a project folder and save into it. A folder that holds a project is refused |
| `format::folder::Folder::save` | Save: only changed files are written, each atomically |
| `format::folder::Folder::outside_changes` | The files changed on disk since they were read or saved here. Empty when nothing changed |
| `format::pack::MAX_PACKED` | The largest packed project read, in bytes |
| `format::pack::pack_folder` | A project folder as a packed project. The same folder gives the same bytes |
| `format::pack::folder_files` | The files `pack_folder` packs, by their path in the project |
| `format::pack::unpack` | The files of a packed project, by their path in it, pictures included. Size, count and names are checked first |
| `format::pack::read_packed` | A packed project as a `Project`, in memory |

### The book view

| Item | What Book relies on |
| --- | --- |
| `format::book::VERSION` | The book view's version, 1 |
| `format::book::Book` | **Built field by field**: `version`, `metadata`, `front`, `body`, `back`, `assets`. It can be cloned and compared |
| `format::book::Metadata` | **Built field by field**: `info`, `byline` |
| `format::book::Section` | **Built field by field**: `id`, `role`, `kind`, `title`, `number`, `content`, `notes`, `children`. It can be cloned and compared |
| `format::book::Asset` | **Built field by field**: `id`, `path`, `title` |
| `format::book::FrontKind` | **Matched in full**: `TitlePage`, `Copyright`, `Dedication`, `Epigraph`, `Contents`, `Acknowledgements`, `AboutTheAuthor`, `AlsoBy`, `Other` |
| `format::book::from_project` | The book view of a project. The same project gives the same view every time |
| `format::book::outline` | Every section's id and role, in reading order |
| `format::book::counts` | How many sections of each role |
| `format::book::missing_metadata` | The details Book needs that the writer has not set, by name |

### Fixes offered back

| Item | What Book relies on |
| --- | --- |
| `format::offers::VERSION` | The offers file's version, 1 |
| `format::offers::MAX_FILE` | The largest offers file read, in bytes |
| `format::offers::Offer` | **Built field by field**: `id`, `item`, `find`, `replace`, `note`, `made` |
| `format::offers::OffersFile` | **Built field by field**: `version`, `offers` |
| `format::offers::Decision` | **Matched in full**: `Accepted`, `Rejected` |
| `format::offers::offers_path` | Where a project folder's offers file is: beside the folder, never in it |
| `format::offers::read` | An offers file, checked. A newer version is refused |
| `format::offers::write` | An offers file as bytes |
| `format::offers::decisions` | The writer's decisions so far, by offer id |
| `format::offers::decide` | Record a decision in a project (tests of Book's side use it) |

<!-- END contract surface -->

The two crates offer more, such as `book::open_folder`, `book::open_packed`, `book::words`, the
`diff` and `drafts` modules and the text dialect. Book does not use them, so they are not part
of the contract. Book asks for an item to be added with an issue here
([How this contract changes](#how-this-contract-changes)).

## What the book view gives

| Part | What | Type |
| --- | --- | --- |
| `version` | The book view's version (1) | `u32` |
| `metadata.info` | Title, subtitle, author (legal name), pen name, series name and number, language, ISBN per edition, publisher, copyright year, rights statement, description, subjects, keywords | `BookInfo` |
| `metadata.byline` | The name the book is published under (the pen name, else the author) | `String` |
| `front`, `body`, `back` | Sections in reading order: front matter; parts, chapters, scenes and plain items; back matter. Items left out of export, screenplays, bible entries and media are not sections | `Vec<Section>` |
| `Section.id` | The item's stable id (UUID version 7): Book keys its layer by it | `ItemId` |
| `Section.role` | Part, chapter, scene, front matter, back matter or none | `Role` |
| `Section.kind` | For front and back matter: title page, copyright, dedication, epigraph, contents, acknowledgements, about the author, also by, or other (from the item's title) | `FrontKind` |
| `Section.title`, `Section.number` | The title as the writer set it; parts and chapters numbered in order, each counting on its own | `String`, `Option<u32>` |
| `Section.content` | Semantic text only, in its canonical form: paragraphs with italic, bold, small caps, strikethrough, language spans, links, note references; headings (front matter pages); scene breaks; block quotes; verse; asides with their kind; images with alt text and role (`inline`, `wide`, `page`); raw blocks for a target format | `Vec<Block>` (the text dialect's model, `docs/format.md`) |
| `Section.notes` | The notes the content refers to, as notes only: footnote or endnote is Book's choice | `Vec<Note>` |
| `Section.children` | Sections under it (a part's chapters, a chapter's scenes) | `Vec<Section>` |
| `assets` | Pictures and other files in the project, with their paths inside it | `Vec<Asset>` |

The view is of the draft the writer chose for export.

What the view keeps and leaves out of the writer's working marks, and what it does not say:

- **Revision pass marks are taken off** (`Marks::rev` is always none in the view).
- **Comment anchors stay.** A span a comment is on keeps the comment's id in `Marks::comment`.
  The comment itself (its text, who wrote it, when) is not in the view. A reader of the view
  ignores the anchor: it changes nothing a reader of the book sees. It also means two
  neighbouring spans may differ only in their anchor, so a reader that joins text by its marks
  leaves `comment` out of the comparison. Taking the anchors off would change the spans every
  reader gets, so it would be a new view version.
- **Raw blocks** (`Block::Raw`) hold content for one target format, named by the writer
  (`html`, `latex`, any word). The contract names no list of formats a reader must honour. A
  reader that does not write the named format leaves the block out and says so in its report.
- **Pictures** carry alternative text and a role. Nothing marks a picture as the cover or as
  decorative, and empty alternative text only means the writer set none.
- **ISBNs** are keyed by an edition name the writer typed. No list of editions or of their
  formats exists.

The last three are open requests from Book: [Requests from Book](#requests-from-book).

Chosen fonts with their licenses are not part of the view. Book uses its own.

## Formats and versions

Book reads and writes files through the two crates. These are the formats, the version of each
that this contract covers, and what a build of Book on a tag of this contract does with them.

| Format | Specified in | Versions read | Version written | A newer version |
| --- | --- | --- | --- | --- |
| The project folder (`.wsm`): `schema` in `project.toml` and `binder.toml` | `docs/format.md` | 1, 2 and 3 | 3 | Refused with `Error::Newer`, which gives both numbers. Never read by guessing |
| The packed project (`.wsmz`) | `docs/format.md`, and below | The same: 1, 2 and 3 | 3 (`pack_folder` packs the files as they are) | Refused the same way |
| The book view | This file | 1 (`book::VERSION`) | – | A build of Book has one view version compiled in |
| The offers file (`<project>-offers.json`) | [Offering fixes back](#offering-fixes-back) | 1 (`offers::VERSION`) | 1 | Refused by `offers::read` |

- **The schema is what changes at run time.** The view's version is a constant compiled into
  Book, so checking it can never fail in a built program. What a built Book meets in the world
  is a project saved by a newer Manuscript. Book should catch `Error::Newer` and say in its own
  words that Wordsmith Book must be updated: the text of the error itself is written for
  Manuscript's window.
- **An older project is migrated when read**, in memory. `Folder::save` then writes it as
  schema 3. Book saves only sources it owns, so it never rewrites a writer's older project.
- **A packed project** is a zip written so that the same project gives the same bytes: a
  stored `mimetype` entry first (`application/x-wordsmith-manuscript-packed`), then
  `project.toml`, `binder.toml` and the files under `text/`, `bible/`, `media/`, `scripts/`,
  `snapshots/`, `conflicts/` and `drafts/`, in sorted order. It has no version of its own: its
  version is the schema of the `project.toml` inside. A reader checks the file's size
  (`pack::MAX_PACKED`), the number and size of the entries and every name before it reads.
- **Ids** are UUID version 7, written lowercase with hyphens. An item keeps its id for good:
  through renames, moves, drafts and schema migrations.

### What a change to a version means

| The change | What it is | What happens |
| --- | --- | --- |
| A listed item is removed or renamed, a field is added to a struct built field by field, a variant is added to an enum matched in full, or the book view gives something else for the same project | A break | `book::VERSION` is raised. The first tag of the new view version is cut. `CHANGELOG.md` and the table under [Tags](#tags) say what Book must do. An issue is opened in the Book repository |
| Manuscript starts to write a new schema | A change Book must follow, and a break only when the view changes too | A new tag is cut in the same view version. `CHANGELOG.md` says what Book must do: move to the tag and release, so that Book opens projects the new Manuscript saves. An issue is opened in the Book repository |
| An item is added to the list | An addition | A new tag in the same view version. Book moves when it wants the item |
| The offers file gains a version | A change both apps make together | As for a schema |

**Today:** nothing makes a release of Manuscript that writes a new schema wait for a release
of Book that reads it. Until Book has moved, a Book already installed refuses projects saved by
the newer Manuscript.

## Offering fixes back

Book never writes the project folder. It writes its offers into a file **beside** the folder,
`offers::offers_path(root)`: for `My Novel.wsm`, `My Novel.wsm-offers.json`. The file is an
`offers::OffersFile`, written with `offers::write`:

```json
{
  "version": 1,
  "offers": [
    { "id": "7d2c…", "item": "0190a5c2-…", "find": "teh harbour", "replace": "the harbour",
      "note": "typo", "made": "2026-11-24T10:00:00Z" }
  ]
}
```

- `id`: Book's own id for the offer, unique in the file. `item`: the stable id of the item whose
  text changes (from the book view). `find`: the exact words as a reader sees them (no marks),
  within one paragraph; they must occur exactly once in the item, so give enough words around the
  change. `replace`: the new words (plain text; they take the formatting of the words replaced).
- Book appends offers and never removes or rewrites one Manuscript may have decided.
- Manuscript lists them in its "Offered fixes" inbox (`offers.list`, `offers.accept`,
  `offers.reject`); an offer whose words are no longer there exactly once is "stale" and can only
  be rejected.
- The writer's decisions are kept in the project; Book reads them with
  `offers::decisions(&project)` (offer id → accepted or rejected). An offer with no decision is
  still waiting. When a later export differs from an accepted or waiting fix, Book shows the
  difference and never overwrites.

The offers file has its own version (`offers::VERSION`, 1); Manuscript refuses a newer one.

## The book package (EXP-5)

Export › Book package writes the project as a packed `.wsmz`, after reading it back through the
book view and checking every word, role and id against the project; the report lists the
metadata Book needs that is not set yet.

## Tags

Book names a tag of this repository on its two `wordsmith-manuscript-` lines. It pins a tag,
never a branch or a bare commit.

### How a tag is named

`book-view-<view version>.<count>`.

- `<view version>` is `book::VERSION` on the tagged commit.
- `<count>` rises by one with each tag of that view version. The first tag of a view version
  has no count: `book-view-1`, then `book-view-1.1`, `book-view-1.2` and so on; a second view
  version would start at `book-view-2`.
- The tag is annotated. Its message says the view version, the core release and why it was
  cut.

### When a tag is cut

- **Manuscript moved to a newer core release.** Book holds one core release, so it can follow
  Manuscript to a newer core only through a tag on that commit.
- **The contract changed**: an item was added to [the list](#what-book-may-use), a listed item
  or the book view changed, or a format gained a version.
- **Book asked for a change** in an issue here, and the change has landed.

A change that touches nothing Book builds on (the interface, imports and exports, the command
line, documents) needs no tag.

### Who cuts it, and on which commit

A person or an agent, on a commit of `main` where both of these have passed:

1. the full gate, `cargo xtask ci`, which holds the contract test;
2. the workflow `book-consumer.yml` ("Book consumer"), which runs Book's own gate against that
   commit's model and format crates ([How this contract is kept](#how-this-contract-is-kept)).

Then the row is added to the table below in the same change, or the next one.

**After a core move** Book's `main` still names the older core, and a build may hold only one.
The workflow says so and stops. Book prepares a branch on the new core, the workflow is run by
hand against that branch (its `book-ref` input), the tag is cut when it passes, and Book's
branch then moves its two `wordsmith-manuscript-` lines to the tag.

### A tag is never moved or deleted

Book's builds name it. The repository's ruleset for tags refuses a move or a deletion of any
`book-view-*` or `v*` tag. A mistake is corrected with the next tag.

### Release tags

A `book-view` tag is not a release: it marks a commit Book may build on, and no program is
published from it. No release of Manuscript has been published yet.

Once Manuscript has releases, a release tag `vX.Y.Z` is also a tag Book may pin, when it is
listed in the table below. It is listed when the release commit passed both checks above, and
the table says which view version and core release it carries. Book prefers a listed release
tag to a `book-view` tag on the same core release. Between releases, `book-view` tags go on
being cut as above, because the core moves more often than Manuscript is released.

### The tags

| Tag | Book view | Core release | Why it was cut |
| --- | --- | --- | --- |
| `book-view-1` | 1 | v0.1.4 | The first book view |
| `book-view-1.1` | 1 | v0.1.16 | Manuscript moved to a newer core |
| `book-view-1.2` | 1 | v0.1.17 | Manuscript moved to a newer core |
| `book-view-1.3` | 1 | v0.1.18 | Manuscript moved to a newer core |
| `book-view-1.4` | 1 | v0.1.20 | Manuscript moved to a newer core |
| `book-view-1.5` | 1 | v0.1.20 | The contract is written as a list with a test, and the Book consumer workflow exists. The model's word list and comparison are the core's (`jcj_doc`), with the same names and results. Nothing Book uses changed, so Book has nothing to do but move its two lines |
| `book-view-1.6` | 1 | v0.1.21 | Manuscript moved to a newer core |

## How this contract is kept

Two checks in this repository, so a break shows before it lands and not when Book next moves.

- **The contract test**, `crates/manuscript-format/tests/book_contract.rs`, in the gate. It
  uses every listed item the way Book does, asserts what the book view keeps and leaves out and
  which schemas read and are written, and fails when this file's list and its own differ.
- **The Book consumer workflow**, `.github/workflows/book-consumer.yml`. On a pull request or
  a push to `main` that touches `crates/manuscript-model`, `crates/manuscript-format` or the
  root `Cargo.toml`, and when run by hand, it checks out Wordsmith Book, links it to this
  checkout's two crates (`cargo xtask dev-link manuscript=<folder>` in Book) and runs Book's
  whole gate (`cargo xtask ci --linked`). It takes about half an hour, which is why it runs
  only for those paths.

To try the same on a computer, with the two repositories side by side: in the Book checkout,
`cargo xtask dev-link manuscript=../wordsmith-manuscript`, then `cargo xtask ci --linked`, and
`cargo xtask dev-unlink` when done. The link changes no file Git tracks.

**Today:** the test's list is kept in step with Book's code by hand. An item Book starts to use
without asking is found by the workflow only when a change here breaks it.

## Requests from Book

What Book needs arrives as an issue in this repository
([Route B](https://github.com/jamescjfoundation/wordsmith/blob/main/project/README.md#route-b)).
Open requests for something the model does not have yet, and what the contract says Book does
meanwhile:

| Issue | What Book asks for | Meanwhile |
| --- | --- | --- |
| [#9](https://github.com/jamescjfoundation/wordsmith-manuscript/issues/9) | A cover role for a picture | The view names no cover. Each edition names its cover file in Book's own project |
| [#10](https://github.com/jamescjfoundation/wordsmith-manuscript/issues/10) | Editions with their formats in the book's details | `BookInfo.isbn` is keyed by a name the writer typed. Book keeps its own editions and offers an ISBN where a name matches |
| [#11](https://github.com/jamescjfoundation/wordsmith-manuscript/issues/11) | Which target formats' raw blocks a reader should honour | The contract names none. Book leaves out a raw block whose format it does not write, and lists it in its report |
| [#13](https://github.com/jamescjfoundation/wordsmith-manuscript/issues/13) | A decorative mark on pictures | `Block::Image` cannot say so, and empty alternative text does not mean it. Book keeps its own list of decorative pictures |

## How this contract changes

Manuscript owns this contract
([project decision 0004](https://github.com/jamescjfoundation/wordsmith/blob/main/project/decisions/0004-apps-that-depend-on-apps.md),
[Route B](https://github.com/jamescjfoundation/wordsmith/blob/main/project/README.md#route-b)).

- What Book needs is asked for in an issue in this repository. The change is made here, lands
  here first and is tagged; Book then moves to the tag. A change in Book never edits this
  repository.
- A change here to `manuscript-model`, `manuscript-format`, the project schema or the book view
  is checked against this file before it lands. The contract test and the Book consumer
  workflow do most of that; [What a change to a version means](#what-a-change-to-a-version-means)
  says what follows from each kind of change.
- An item is added to the list by adding its row here and its use in the contract test, in one
  change.
- For anything Book must follow, an issue is opened in the Book repository.

## Review

The Book side's review of this contract is `docs/handoff-review.md` in the Book repository. It
awaits a person's sign-off there (plan step M6.3).
