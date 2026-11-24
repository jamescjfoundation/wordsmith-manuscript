# The hand-off to Wordsmith Book

Book view version 1. This is the contract between Wordsmith Manuscript and Wordsmith Book (plan
section 9.2). Manuscript writes the words and structure; Book designs and typesets them.

## How Book reads a manuscript

1. **Book opens the Manuscript project directly**: a project folder (`.wsm`) or a packed project
   (`.wsmz`), through `wordsmith-manuscript-format`'s read-only `book` module, depended on at a
   release tag of this repository:
   - `book::open_folder(path)` and `book::open_packed(bytes)` give a `book::Book`;
   - `book::from_project(&project)` gives the same from a project in memory.
   EPUB is only an export, never the hand-off. Book converts its own DOCX, Markdown and text
   imports into the same model, so it has one input model.
2. **Book never writes the Manuscript project.** Its own layer (designs, editions, store links,
   print-only and ebook-only flags, per-edition overrides) lives in a Book file keyed by
   Manuscript's ids.
3. **Fixes offered back** (Manuscript 0.2): a light fix made in Book is stored as a change against
   an id and offered to Manuscript, which shows it in an "Offered fixes" inbox for the writer to
   accept or reject. If a later export conflicts with a Book fix, Book shows the difference and
   never overwrites silently.

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
| `Section.content` | Semantic text only: paragraphs with italic, bold, small caps, strikethrough, language spans, links, note references; headings (front matter pages); scene breaks; block quotes; verse; asides with their kind; images with alt text and role (`inline`, `wide`, `page`); raw blocks for a target format | `Vec<Block>` (the text dialect's model, `docs/format.md`) |
| `Section.notes` | The notes the content refers to, as notes only: footnote or endnote is Book's choice | `Vec<Note>` |
| `Section.children` | Sections under it (a part's chapters, a chapter's scenes) | `Vec<Section>` |
| `assets` | Pictures and other files in the project, with their paths inside it | `Vec<Asset>` |

Helpers: `book::outline` (every id and role in order), `book::words` (every word of the parts'
and chapters' titles and the content, in order), `book::missing_metadata` (what Book needs that
the writer has not set), `book::counts`.

Chosen fonts with their licenses come with Manuscript 0.2's export settings; until then the
book view gives none and Book uses its own.

## Offering fixes back (Manuscript 0.2)

Book never writes the project folder. It writes its offers into a file **beside** the folder,
`manuscript-format::offers::offers_path(root)`: for `My Novel.wsm`, `My Novel.wsm-offers.json`.
The file is `manuscript-format::offers::OffersFile`, written with `offers::write`:

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
  `manuscript-format::offers::decisions(&project)` (offer id → accepted or rejected). An offer
  with no decision is still waiting. When a later export differs from an accepted or waiting fix,
  Book shows the difference and never overwrites.

The offers file has its own version (`offers::VERSION`, 1); Manuscript refuses a newer one.

## The book package (EXP-5)

Export › Book package writes the project as a packed `.wsmz` (deterministic, `docs/format.md`),
after reading it back through the book view and checking every word, role and id against the
project; the report lists the metadata Book needs that is not set yet.

## Versions

The book view has its own version; Book declares which versions it reads. A change that breaks
Book raises the version and is a minor release of this repository with a "What Book must do"
note in `CHANGELOG.md`, as the core's changelog does for apps.

| Version | Manuscript release | Changes |
| --- | --- | --- |
| 1 | 0.1 | First version |

## Review

This contract awaits review by a person from the Book side (plan step M6.3).
