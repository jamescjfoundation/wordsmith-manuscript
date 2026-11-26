# 0007: Series and a shared story bible

Date: 2026-11-24 · Status: accepted · Amended 2026-11-26 (the entries layer is in the core; no
Script project joins a series yet)

## Context

BND-8 groups several books in a series sharing one bible; BIB-5 shares that bible with Wordsmith
Script projects too (plan section 5.6: "a bible is a folder of entry files; a series or an
adaptation points several projects at one bible folder, with the same outside-change detection").

## Decision

- **A series bible is a project folder that holds only a Story Bible**: its `project.toml` names
  the series and lists its books (`[extra] series`: `name`, `books` as paths from the series
  folder) and keeps the entry types (`[extra] templates`); its `binder.toml` holds the Bible area;
  `bible/<id>.md` the entries, in the same format as a book's. So the series folder is read,
  saved, checked for outside changes, packed and backed up by the code that already does those
  things for books.
- **A book names its series** in `project.toml` (`series`, a path from the book's folder). In
  memory the book is whole (the series' bible merged into its Bible area), so every view and
  command, backlinks and spelling included, works unchanged; saving splits it again, the book's
  own part to its folder and the bible to the series folder.
- **Changes made by another book reach this one** through outside-change detection on the series
  folder: read in when nothing here is unsaved, otherwise this version is kept and theirs copied
  into the series folder's `conflicts/`.
- **Starting a series moves the book's bible into the new series folder; joining adds the book's
  own entries to the series; leaving keeps a copy** of the series bible as the book's own.
- **Script**: the folder format is the general one (entries by id, templates as data) so a
  Wordsmith Script project can point at the same folder once the entries layer is in the core
  (C13); until then only Manuscript projects join.

## Consequences

A book in a series is not complete without its series folder; opening it without one shows the
book with whatever bible its own folder holds and says why. Backups of a book include its series
folder's backup beside it.

## Revisit when

Script joins series (the entries layer in the core), or writers want parts of a bible private to
one book.

## Amended 2026-11-26

The entries layer has been in the core since core release 0.1.4 (`jcj-entries`), so the
condition under "Script" is met. No Wordsmith Script project joins a series yet: Script does
not read or write a series folder. Only Manuscript projects join.
