# Exporting

File › Export writes the manuscript to a file. Export never changes the project, and every export is
read back and checked word for word before the file is kept.

| Format | For |
| --- | --- |
| Standard manuscript (DOCX, PDF) | Agents, editors and publishers |
| Word processor (DOCX), OpenDocument (ODT), rich text (RTF) | Other word processors |
| Markdown, plain text, web page (HTML) | The web and other tools |
| Reading copy (EPUB) | Reading the draft on a phone or e-reader: each part and chapter on its own page with a contents entry, notes numbered at the end of their chapter |
| Book package (`.wsmz`) | Wordsmith Book |

## Options

Which items (by default every included manuscript item); how chapters and parts are titled (their
titles, "Chapter 1", "Chapter One", "One", "Chapter 1: Title", or none); scene titles as headings;
the scene-break symbol; front and back matter; replace-text rules; END after the last line. Save
options as a preset (`export.savePreset`) to use again.

## The standard manuscript

Letter or A4 paper, 1-inch margins, 12-point serif or monospaced type, double spacing, half-inch
indents. A title page with your name and contact lines, the word count rounded ("about 82,000
words"), the title a third down and the name you publish under. From the first page of text, a
header with your surname, the short title and the page number, counting from 1. Chapters start on
a new page a third of the way down; scene breaks are a centered `#`; the text ends with END.
Set your name, pen name, surname and short title for the header, and your contact lines, in the
book details (`project.setBook`).

## The book package

The book package is the project packed as one file for Wordsmith Book, checked by reading it back
through the book view. Its report lists the details Book needs that are not set yet (copyright
year, rights statement, description...).

## Fixes offered back by Wordsmith Book

When Wordsmith Book suggests a light fix (a typo, a missing comma) while typesetting, it offers it
back instead of changing your project. The status bar shows how many fixes wait; click it, or use
Offered Fixes in the command palette. Each fix shows the document, why, and the words before and
after. Accept takes it into the text (one undo step); Reject turns it down; Open shows the
document. A fix whose words you have since changed can only be rejected.

## Other formats, through a converter

For a format Manuscript does not write, you can use a document converter you installed yourself
(Manuscript never comes with one). On File › Export choose "Another format, through a converter
you installed", give the converter program and its arguments, with `{input}` where it takes the
manuscript (Manuscript hands it a DOCX), `{output}` where it writes the file, and `{format}` for
the file's extension, then Export. The program is run directly, with no command shell, and stopped
if it runs for more than five minutes. When Manuscript can read the result back it checks every
word; otherwise the report says it was not checked.
