# 0003: Our own reader for the text dialect

Date: 2026-11-24. Status: accepted.

## Context

Plan section 6.3 chose Markdown for item text, with `pulldown-cmark` as the parser unless it could
not give the exact round trips the format promises: model → text → model exact, and text the
writer did not change byte-identical. The dialect is a strict, line-based subset: one paragraph per
line, a blank line between blocks, a fixed set of block markers, and a few bracket extensions
(`[text]{.smallcaps}`, `[text]{lang=fr}`, `[text]{.comment id=c1}`) that CommonMark does not have.

A general CommonMark parser reads far more than the dialect (lists, code spans, HTML, reference
links, setext headings, lazy continuation), and its emphasis rules depend on the characters around
each `*` (flanking), so the writer would have to predict another program's decisions to stay
exact, and the extensions would still need our own code after it.

## Decision

- The dialect is read and written by our own code in `manuscript-format` (`dialect`): a
  line-based block reader and an inline reader with a stack of open markers. No new dependency.
- The writer emits one canonical form: marks nest in a fixed order (comment, link, language, small
  caps, bold, italic, strikethrough), dialect punctuation is always escaped, block markers at the
  start of a paragraph are escaped, and edge whitespace is written as character references.
- The inline reader does not use CommonMark's flanking rules. A run of `*` closes an open italic
  first, then a bold; otherwise it opens bold for two and italic for one. Markers left unclosed
  (only possible in hand-written text) are read as plain text and the paragraph is read again, so
  no character is lost; the retries are bounded.
- Underscore emphasis, lists, code spans and HTML are not part of the dialect: underscores and the
  other characters are escaped when written and read literally.
- Unchanged files are never rewritten, so hand-written text that is not in the canonical form keeps
  its bytes until the writer changes that item.

## Consequences

- Property tests (random texts with every construct and awkward characters, tens of thousands of
  cases) prove both round trips and that any input reads and settles.
- Any CommonMark viewer shows the files sensibly; a few extensions show as bracketed text.
- Text pasted from other Markdown tools (with `_emphasis_` or lists) arrives as plain text; the
  Markdown importer (IMP-5, through the core) handles full Markdown.

## Revisit when

The dialect needs a construct whose reading depends on context the line-based reader cannot see,
or a maintained CommonMark parser offers exact source positions and extension hooks that would
let us drop our reader.
