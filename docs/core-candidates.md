# Core changes and core-candidates

Every core change Manuscript has asked for and every `// core-candidate:` in this repository,
with its core issue and state. The ids are plan section 5.4's; that table is as the plan was
written, and this file is what is true now. Issue numbers are open-source-core's.

States:

- **app-first**: built here, marked `// core-candidate: open-source-core#N`, waiting for a second
  app to want it. Nothing in the core does this yet.
- **copy kept**: the core has released its version, but this repository still holds its own, for
  the reason given, marked with the open issue that would free it.
- **released in core vX, used**: the core has it and Manuscript uses the core's.
- **open**: asked for, not started.
- **not needed**: the need went away.

The rows, the markers in the code and the core's issues are kept in step. Each app-first and
copy-kept row's issue is open, labelled `core-candidate`, and names the file here that holds the
code. A marker is written `core-candidate: open-source-core#N`, and `cargo xtask docs` fails on
one that names no issue. Last checked against the issues and core v0.1.20 on 2026-11-26.

## Markers in the code

| Id | What | `// core-candidate:` location | Core issue | State |
| --- | --- | --- | --- | --- |
| C6 | A sortable, editable table widget (the outline) | `crates/manuscript-ui/src/table.rs` | [#7](https://github.com/jamescjfoundation/open-source-core/issues/7) | app-first |
| C7 | Search across many documents with an index | `crates/manuscript-ui/src/search.rs` | [#8](https://github.com/jamescjfoundation/open-source-core/issues/8) | app-first |
| C8 | Editing several documents as one (the stitched view) | `crates/manuscript-ui/src/stitch.rs` | [#9](https://github.com/jamescjfoundation/open-source-core/issues/9) | app-first |
| C11 | Story board lanes | `crates/manuscript-ui/src/lanes.rs` | [#12](https://github.com/jamescjfoundation/open-source-core/issues/12) | app-first. The plan meant a promotion from Wordsmith Script; code never moves between app repositories, so lanes were built here |
| – | Notes in the EPUB reading copy | `crates/manuscript-io/src/export.rs` (`notes_as_endnotes`) | [#33](https://github.com/jamescjfoundation/open-source-core/issues/33), after [#21](https://github.com/jamescjfoundation/open-source-core/issues/21) | copy kept: see [below](#notes-in-the-epub-reading-copy) |

## Core changes with no marker here

| Id | What | Core issue | State |
| --- | --- | --- | --- |
| C1 | Template chooser and start window | [#2](https://github.com/jamescjfoundation/open-source-core/issues/2) | released in core v0.1.2, used (`ShellOptions::start_window`, `TemplateSpec`). The issue is still open |
| C2 | Writing statistics, session targets and the sprint timer | [#3](https://github.com/jamescjfoundation/open-source-core/issues/3) | released in core v0.1.2, used (`jcj_shell::writing`). The issue is still open |
| C3 | Document tabs and split view | [#4](https://github.com/jamescjfoundation/open-source-core/issues/4) | released in core v0.1.3, used (`jcj_shell::split`). The issue is still open |
| C4 | The PDF names the app that made it | [#5](https://github.com/jamescjfoundation/open-source-core/issues/5) | open. The standard manuscript PDF carries the upstream engine's name as its producer (`docs/golden.md`, "Known gap") |
| C5 | A tree widget with drag and drop (the binder) | [#6](https://github.com/jamescjfoundation/open-source-core/issues/6) | released in core v0.1.8, used since 2026-11-26: the binder draws with `jcj_shell::tree` and this repository's copy (`manuscript-ui/src/tree.rs`) is gone |
| C9 | Word-for-word verification of an import or export | [#10](https://github.com/jamescjfoundation/open-source-core/issues/10) | released in core v0.1.9, used since 2026-11-26: `wordsmith_manuscript_io::verify` is the core's `jcj_features::verify` under the old name and this repository's copy (`manuscript-io/src/verify.rs`) is gone |
| C10 | Grammar hints beside spelling | [#11](https://github.com/jamescjfoundation/open-source-core/issues/11) | not needed: `jcj_proof::check` already gave them (progress, R2 ED-9). The issue is still open |
| C12 | EPUB import | [#13](https://github.com/jamescjfoundation/open-source-core/issues/13) | released in core v0.1.3, used (`jcj_interchange::import` with `Format::Epub`, IMP-9). The issue is still open for what the reader does not keep |
| C13 | Entries: typed records with templates, links, backlinks and a name list | [#14](https://github.com/jamescjfoundation/open-source-core/issues/14) | released in core v0.1.4 as `jcj-entries`, used (`manuscript-model/src/entries.rs` is this app's layer over it; decision 0004). The entry file format stays here until a second app shares bible folders. The issue is still open |
| – | The word list and the word-level comparison of two texts, where a model crate can use them | [#31](https://github.com/jamescjfoundation/open-source-core/issues/31), after [#10](https://github.com/jamescjfoundation/open-source-core/issues/10) and [#17](https://github.com/jamescjfoundation/open-source-core/issues/17) | released in core v0.1.20 (`jcj_doc::words`, `jcj_doc::wordiff`), used since 2026-11-26: `manuscript-model`'s `diff` re-exports the core's comparison under the same names and keeps `texts`, its own entry point over this app's text type; `words::words` calls the core's. The copy and the test that compared the two word rules are gone. The rule is written once |
| – | Interface translation catalogs | [#32](https://github.com/jamescjfoundation/open-source-core/issues/32), after [#18](https://github.com/jamescjfoundation/open-source-core/issues/18) | released in core v0.1.20 (`jcj_shell::i18n`), used since 2026-11-26: the interface looks its texts up through the shell and this repository's catalog code (`manuscript-ui/src/i18n.rs`) is gone. `manuscript-ui/src/languages.rs` holds only what is this app's: the languages it comes in and its catalogs. One language now holds for the whole window, the shell's own texts included |

## What would free each copy

Each of these was re-checked against core v0.1.18 on 2026-11-26. None is a drop-in, and none is
forced: the open issue says what the core would need to offer.

### Notes in the EPUB reading copy

`notes_as_endnotes` in `manuscript-io/src/export.rs` turns each chapter's notes into numbered
paragraphs under a "Notes" heading before the core writes the EPUB, and returns the words it
added so the word check can leave them out. The core's writer has kept notes itself since
v0.1.12.

Why it stays: the core's writer numbers notes from 1 in every chapter file, where the reading
copy numbers them through the book; it writes no heading; and it adds a link under each note
whose text is the fixed English word "Back", which a caller can neither change, translate nor
learn. Moving to it would change what the reader of the EPUB sees and put words in the file the
word check does not expect.

What the core must offer (#33): options on writing whose defaults give today's bytes. The
backlink's text given by the caller, or no visible word; numbering per chapter file or through
the book; an optional heading; and the words the writer added, returned or computable, so a
word check can leave out exactly those.
