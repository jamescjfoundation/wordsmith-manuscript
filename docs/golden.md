# Golden files

Reviewed reference outputs (plan section 4, rule 7). The tests compare each export with its file;
a change that alters one is deliberate: rerun the tests with `UPDATE_GOLDEN=1`, review the
difference, and add a line below saying what changed and why.

| File | What it holds | Test |
| --- | --- | --- |
| `tests/golden/standard-manuscript.pdf.txt` | The standard manuscript PDF of a written-for-test project (three chapters of two scenes): every page's text, line by line, with its page breaks | `manuscript-io/tests/golden.rs`, `standard_manuscript_pdf_text_and_page_breaks` |
| `tests/golden/standard-manuscript.docx.txt` | The same manuscript as DOCX, read back: page size and margins, then each paragraph's style, alignment, first-line indent, space before, line spacing, page break, section break and first words | `manuscript-io/tests/golden.rs`, `standard_manuscript_docx_structure` |

## Changes

| Date | File | What changed | Reviewed |
| --- | --- | --- | --- |
| 2026-11-24 | both | First version: title page (contact block, word count "about 2,000 words" at the top right, title a third down, "by", byline), then from page 1 the header "Writer / LIGHTHOUSE / n", chapters on a new page a third down, double spacing, half-inch indents, scene breaks centered, END | Checked by the author of the change against SPEC 16.2; awaits a maintainer's review |

## Exports with no golden file

The EPUB reading copy has no reference file here: its bytes are the core's writer's, which the
core holds to a hash of its own. What Manuscript chooses is held by tests
(`manuscript-io/tests/export.rs`): one file per part and chapter, the notes, and the word check.

| Date | What changed | Why |
| --- | --- | --- |
| 2026-11-26 | The reading copy's notes are written by the core's EPUB writer, not put into the text as paragraphs before it runs. Three things differ in the file, on purpose: a note's number where it is called is a link to the note (it was a raised number); the notes at a chapter's end are a numbered list marked as endnotes for reading systems (they were paragraphs starting "1. "); "Notes" is a heading (it was a bold paragraph). The numbering, once through the book, and the absence of a link back are as before. The word check changed with it: with notes, the file is read back and compared by letters, because a note's number stands directly after a word, against the manuscript's text, its notes and the words the writer says it added; the text's letters must also stand in their order. Without notes the file is still compared word for word, in order | Core issue #33: the core's writer now takes these choices and reports its added words, so this app's copy of the work is gone |

## Known gap

The PDF's creator and producer fields name the upstream engine rather than this app: the core
writes them from a fixed value (core issue #5, C4;
[issue 27](https://github.com/jamescjfoundation/wordsmith-manuscript/issues/27) here). Changing it needs a patch in the core's
upstream fork and a move of its pin, which waits on a maintainer's approval; until then the
standard manuscript PDF carries the title and author the writer set and the upstream name as
producer.
