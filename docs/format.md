# The Wordsmith Manuscript project format

Schema version 3. This file specifies every file of a project and the text dialect. The code is
`crates/manuscript-format`; its tests check each example here.

## The folder

A project is a folder, named with the `.wsm` extension (decision 0002):

```text
My Novel.wsm/
  project.toml                  settings and book metadata
  binder.toml                   the binder tree
  text/<id>.md                  one file per text item (and folder with text)
  bible/<id>.md                 one file per story bible entry
  media/<id>.<ext>              images, PDFs and other files
  scripts/<id>.wss              screenplays, in Wordsmith Script's own format (from 1.0)
  snapshots/<id>/<UTC time>.md  named, dated copies of one item's text
  drafts/<draft id>/binder.toml a stored draft's manuscript tree (the Manuscript area only)
  drafts/<draft id>/text/<id>.md its texts, in the same form as text/
  conflicts/<UTC time> <file>   a file changed elsewhere, kept before this copy wrote over it
  .recovery/                    unsaved work while the app runs
```

- **Drafts** (DRF-1): the Manuscript area is the working draft. Each stored draft listed in
  `project.toml` (`[[drafts]]`, ids of letters, digits and hyphens) keeps its manuscript in
  `drafts/<id>/`: a `binder.toml` holding only Manuscript items, and its texts. Items keep their
  ids from draft to draft, so the same scene can be compared across drafts.
- **Series** (BND-8, decision 0007): a book's `project.toml` may name a series bible (`series`, a
  path from the book's folder). A series bible is a project folder holding only a Story Bible:
  `[extra] series` (`name`, `books`: paths from the series folder) and `[extra] templates` in its
  `project.toml`, the Bible area in its `binder.toml`, the entries in `bible/`. The book's own
  folder then keeps no Bible area of its own.
- **Conflicts** (FIL-4): a save never writes over a file that changed on disk since it was read
  (another device, a sync service) without first copying it to `conflicts/`, named by the time
  and the file's path with `/` as `-`. The app reads in files changed only elsewhere, and keeps a
  document changed on both sides as this version plus a snapshot named "Changed elsewhere".
  Readers ignore `conflicts/`; packed projects and backups carry it.
- **Ids** are UUID version 7, lowercase and hyphenated (`0190a5c2-b7d0-7c3e-8d2f-1a2b3c4d5e6f`).
  Files are named by id, so renaming or moving an item never touches its file.
- All text files are UTF-8 with LF line ends. A file the app did not change is never rewritten,
  so text edited by hand keeps its exact bytes (CRLF line ends are read as LF).
- **Saving** writes only changed files, each atomically: text and entry files, then
  `project.toml`, then `binder.toml` last. Files of items that left the binder (after the Trash is
  emptied) are removed after the binder is written. An item whose text is empty has no file.
- The same project always gives the same bytes.

## `project.toml`

```toml
schema = 3
statuses = ["To do", "First draft", "Revised draft", "Final draft", "Done"]

[book]
title = "The Lighthouse"
author = "Test Author"
language = "en"

[[labels]]
name = "Idea"
color = "#8E7CC3"

[targets]
manuscript = 90000
deadline = "2027-06-30"
writing_days = ["mon", "tue", "wed", "thu", "fri"]
```

| Key | Meaning |
| --- | --- |
| `schema` | The format version. A project with a higher number is refused ("saved by a newer version"); a lower one is migrated when read. |
| `[book]` | `title`, `subtitle`, `author` (legal name), `pen_name`, `surname` and `short_title` (for the manuscript header), `contact` (title-page lines), `language` (BCP 47), `series`, `series_number`, `isbn` (a table of edition = ISBN), `publisher`, `copyright_year`, `rights`, `description`, `subjects`, `keywords` |
| `[[labels]]`, `[[keywords]]` | `name` and `color` (`#RRGGBB`) |
| `statuses` | Status names in order |
| `[targets]` | `manuscript` (words), `deadline` (`YYYY-MM-DD`), `writing_days` (`mon` .. `sun`), `session` (words) |
| `[[fields]]` | Custom fields: `name`, `kind` (`text`, `date`, `checkbox`, `list`), `choices` |
| `words` | The project word list for spelling (besides bible names) |
| `[extra]` | Export presets, document-set rules, collections (`collections`: each a `name` with hand-picked `items` or a saved `search`), bookmarks (`bookmarks`: item ids in order), the freeform board (`board`: `board` with its cards, links and grid, and `items`, the item each card stands for), revision passes (`revisions`: the passes, each a name and color, the current one and whether revision mode is on) and other settings kept by name |

Unknown keys are ignored when read.

## `binder.toml`

One `[[item]]` per item, depth first, area by area (`manuscript`, `bible`, `research`, `notes`,
`trash`), so every parent comes before its children and the file's order is the binder's order.

```toml
schema = 3

[[item]]
area = "manuscript"
id = "0190a5c2-b7d0-7c3e-8d2f-000000000001"
kind = "folder"
role = "chapter"
title = "Chapter One"

[[item]]
area = "manuscript"
parent = "0190a5c2-b7d0-7c3e-8d2f-000000000001"
id = "0190a5c2-b7d0-7c3e-8d2f-000000000002"
kind = "text"
role = "scene"
title = "Arrival"
label = "Idea"
status = "First draft"
include = false
```

| Key | Meaning |
| --- | --- |
| `area`, `parent` | Where the item is; no `parent` means the top of the area. The parent must come earlier in the file and be in the same area. |
| `id`, `kind` | The id; `folder`, `text`, `entry`, `media`, `link` or `screenplay` |
| `role` | `part`, `chapter`, `scene`, `front-matter`, `back-matter`; absent means none |
| `title` | The title |
| `include` | `false` leaves the item (and its children) out of exports; absent means `true` |
| `label`, `status`, `icon` | Names from the project's lists; an icon name |
| `file` | For media, the file extension; for a link, the address |
| `from` | For an item in the Trash, the area it came from |

Label, status, role and include live here rather than in the text files, so the binder, the
corkboard and the outline can be shown without reading every text.

## Text files: `text/<id>.md`

Front matter (TOML between `+++` lines, then one blank line) only when the item has metadata,
then the body in the dialect below.

```text
+++
synopsis = "The keeper arrives."
keywords = ["night"]
entries = ["0190a5c2-b7d0-7c3e-8d2f-000000000004"]
target = 2500

[fields]
"Point of view" = "Mara"

[comments.c1]
text = "Check the tide tables."
+++

The keeper climbed the *long* stair[^1].

***

Morning came [late]{.comment id=c1}.

[^1]: Two hundred steps.
```

| Key | Meaning |
| --- | --- |
| `synopsis` | A short summary |
| `notes` | The writer's notes, in the dialect |
| `keywords` | Keyword names |
| `entries` | Bible entries linked by hand |
| `target` | The item's word target |
| `[fields]` | Custom field values by field name |
| `[comments.<id>]` | `text`, `by`, `date` of each comment anchored in the text |

## Entry files: `bible/<id>.md`

```text
+++
kind = "character"
name = "Mara"
aliases = ["the keeper"]

[fields]
Role = "Keeper"

[[relations]]
relation = "sister of"
entry = "0190a5c2-b7d0-7c3e-8d2f-000000000009"
+++

Grew up by the river.
```

`kind` names the entry's template; fields are kept by name, in any template. `[[links]]` in
imported notes are written as `wsm:` links in the body.

## The dialect: Manuscript Markdown

A strict subset of CommonMark (version 0.31.2) with a few extensions. Any Markdown viewer shows a
file sensibly; the app reads it exactly. Read and written by our own code (decision 0003).

### Blocks

Blocks are separated by one blank line. Each paragraph is **one line**; a line break inside a
paragraph is a backslash at the end of the line (CommonMark's hard break), and the next line
continues the paragraph whatever it holds.

| Content | Written as |
| --- | --- |
| Paragraph | One line of text |
| Empty paragraph | A line holding only `\` |
| Scene break | A line holding only `***` |
| Heading 1 to 3 | `# Title`, `## Title`, `### Title` (`#` alone for an empty heading) |
| Block quote | Every line starts with `> `; a line holding only `>` separates its paragraphs |
| Verse | `::: verse`, the stanzas as paragraphs (lines joined by line breaks), `:::` |
| Aside | `::: aside`, or `::: aside letter` (any kind of aside), paragraphs, `:::` |
| Image | `![alt](media/<id>.png){role=inline}`; the role is `inline`, `wide` or `page` |
| Raw block | A fence of three or more backticks with the info `raw <format>`, the content exactly, the same fence |
| Notes | After every block: `[^label]: text`; further paragraphs of a note after a blank line, indented four spaces |

### Inside a paragraph

| Content | Written as |
| --- | --- |
| Italic, bold | `*italic*`, `**bold**` |
| Strikethrough | `~~text~~` |
| Small caps | `[text]{.smallcaps}` |
| Another language | `[text]{lang=fr}` |
| Link to an item or entry | `[text](wsm:<id>)`; other addresses as `[text](https://...)` |
| Comment | `[text]{.comment id=c1}`, the comment in the front matter |
| Revision pass | `[text]{rev=2}`: written in revision pass 2 (an index into `[extra] revisions`); no leading zeros |
| Note reference | `[^label]` |

Marks nest in one fixed order, outermost first: revision pass, comment, link, language, small caps, bold, italic,
strikethrough: `[[*both*]{lang=de}](wsm:x)`. Neighbouring text with the same marks is one span.

### Escapes

So that no character is ever taken for syntax:

- `\ * [ ] ~ _ < `` ` `` are written with a backslash before them; `&` before `#` too; `^` right
  after a `[`.
- At the start of a paragraph, `# > + - = |`, `:::` and ```` ``` ```` get a backslash, and so does
  the `.` or `)` after a number that would start a list (`1\. Not a list`).
- Spaces and tabs at the very start or end of a paragraph are written `&#32;` and `&#9;`; a
  carriage return anywhere is `&#13;`; a line break at the very start of a paragraph is `&#10;`.
- In link and image addresses, `\ ( )` and spaces get a backslash.

Reading accepts any backslash escape of ASCII punctuation and decimal (`&#NNN;`) or hexadecimal
(`&#xHH;`) character references. A marker that is never closed in hand-written text is read as
plain text, so no character is lost. Underscores are never emphasis.

### Round trips

- For every text the editor can produce, model → text → model is exact (property tests generate
  random texts with every construct and awkward characters).
- Text written by the app reads back and writes back byte-identical; files not changed by the
  writer are never rewritten at all.

## Limits

Reading refuses, with an error and never a crash: `project.toml` over 4 MiB, `binder.toml` over
32 MiB, a text or entry file over 64 MiB, front matter over 1 MiB, more than 100,000 items, a tree
deeper than 32 levels, more than 1 GiB of text in all, text that is not UTF-8, and damaged TOML.
Marks nested deeper than 32 levels read the rest of the paragraph as plain text.

## Versions

| Schema | Release | Changes |
| --- | --- | --- |
| 1 | 0.1 | First version |
| 2 | 0.2 | Revision pass marks (`{rev=N}`) and the project's revision passes; a version 1 project reads unchanged (decision 0005) |
| 3 | 0.3 | Drafts: `draft_name`, `[[drafts]]` (`id`, `name`, `created`) and `export_draft` in `project.toml`, each stored draft in `drafts/<id>/`; versions 1 and 2 read unchanged, each tested with a fixture project (decision 0006) |
