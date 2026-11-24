# Fixtures

Every file under `tests/fixtures/` is public-domain or written for the test, and is listed here
with where it came from. Private manuscripts are never fixtures: run them locally from `.private/`
(plan section 4, rule 1).

## Public-domain novels (`tests/fixtures/novels/`)

Plain UTF-8 text with LF line ends. Each is the body of a public-domain e-text, from the line after
its "START" marker to the line before its "END" marker; the distributor's header and licence
footer were removed, as its licence allows for public-domain texts, and nothing else was changed.
Paragraphs are separated by blank lines; italic is shown as `_underscores_`.

| File | Work | Edition | Source | Words |
| --- | --- | --- | --- | --- |
| `war-and-peace.txt` | *War and Peace*, Leo Tolstoy (translated by Louise and Aylmer Maude) | e-text 2600 | https://www.gutenberg.org/ebooks/2600 (fetched 2026-11-24 from a mirror) | about 563,000 |
| `great-expectations.txt` | *Great Expectations*, Charles Dickens | 1867 edition, e-text 1400 | https://www.gutenberg.org/ebooks/1400 (fetched 2026-11-24) | about 184,000 |
| `pride-and-prejudice.txt` | *Pride and Prejudice*, Jane Austen | 1894 illustrated edition, e-text 1342 | https://www.gutenberg.org/ebooks/1342 (fetched 2026-11-24) | about 127,000 |

`war-and-peace.txt` is the scale fixture (over 300,000 words). `great-expectations.txt` is the
novel of the v0.1 test (plan section 10).

## Documents (`tests/fixtures/documents/`, `tests/fixtures/document-set/`)

Written for the test by `tools/make-fixtures.py` (Python standard library only, deterministic):
the novels' paragraphs in the simplest valid form of each format, with "Chapter N." paragraphs in
the Heading 1 style and the texts' `_underscored_` words in italic.

| File | Made from |
| --- | --- |
| `documents/pride-and-prejudice.docx` | `novels/pride-and-prejudice.txt` |
| `documents/pride-and-prejudice.odt` | `novels/pride-and-prejudice.txt` |
| `documents/pride-and-prejudice.rtf` | `novels/pride-and-prejudice.txt` |
| `document-set/Great Expectations (First Draft) 1.docx` to `6.docx` | `novels/great-expectations.txt`, split before chapters 11, 21, 31, 41 and 51: the six-file document set of the v0.1 test |

**Still to add:** DOCX, ODT and RTF versions saved by at least two different word processors (plan
step M0.5). The build machine has no word processor, so these come from a person who has them: open
`novels/pride-and-prejudice.txt` in each, apply Heading 1 to the chapter titles, save as DOCX, ODT
and RTF, and add the files here with the program's name and version written as "an open-source
word processor, version N" or similar (no product names in the repository). Remove the program's
name from the files' generator metadata before committing.

## Written-for-test inputs

Small inputs written for one test live beside that test's fixtures and are listed here as they are
added.
