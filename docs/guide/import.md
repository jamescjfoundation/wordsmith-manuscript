# Importing

File › Import brings documents into the open project. Every import shows a **preview** first (the
binder it will make), and a **report**: what was made, what formatting could not be kept, and a
**word-for-word check** that every word of the source arrived, in order. An import whose words do
not match is not made.

## Documents

DOCX, ODT, RTF, Markdown, HTML, EPUB and plain text. An EPUB book comes back as an editable manuscript, one chapter per reading page.

- **Headings** become parts, chapters and scenes: with three heading levels, part, chapter and
  scene; with two, part and chapter; with one, chapter.
- **"Chapter 12"**, "CHAPTER XII." or "Part Two" on a line of its own starts a chapter or part
  even without a heading style (a rule you can switch off).
- **Scene breaks** (a line of `***`, `* * *`, `#`, a run of one symbol, a short centered symbol
  line, or a mark you add) split a chapter into scenes, or stay as scene breaks.
- Italic, bold, small caps, strikethrough, foreign-language spans, notes and comments are kept.
  Underlined text becomes italic (or is left plain). Fonts, sizes and colors are dropped and listed.
- Plain text joins hard-wrapped lines into paragraphs (or keeps every line), and text with no
  chapters is split at separator lines.
- **Web pages** (a draft or a serial's chapter saved from a browser): when the page marks its text
  as an article or main part, only that is imported; its menus, header and footer are left out,
  and the import report says so. Scripts and styles are never imported.

## Outlines

An OPML file from an outliner becomes binder items: each outline line an item titled by it, its
note the item's synopsis. Lines with lines under them become folders, the rest documents. A title
like "Part Two", "Book I", "Volume 3", "Act One" or "Chapter 12" sets the role; otherwise the
shape does (the innermost lines are scenes, the lines above them chapters, anything above parts).
Every word of every title and note is checked.

## A set of documents

Document Set (a Folder) imports every document in a folder, placed by the role in its name:

| Name contains | Goes to |
| --- | --- |
| (Plan), (Outline) | Notes |
| (Rough Work), (Notes), (Research) | Research |
| (First Draft), (Second Draft), (Third Draft), (Final Draft), (Draft), or nothing | The manuscript |

Several files of one draft ("Part 1", "Part 2", or numbered names) are joined in name order into
one manuscript. When the set holds several drafts, the latest is the draft you write and the earlier
ones are kept as drafts (View › Drafts), their chapters matched to compare scene by scene. The name rules can be changed (`import.set` with `names`) and are kept in the
project.
