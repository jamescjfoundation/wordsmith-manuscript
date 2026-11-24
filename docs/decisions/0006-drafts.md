# 0006: Drafts in one project

Date: 2026-11-24. Status: accepted.

## Context

Writers keep drafts: the first draft as it was finished, the second as it is being written.
DRF-1 asks for several drafts of the manuscript in one project, a new draft started from the
current one, drafts compared scene by scene, both kept, and a choice of which draft exports.
Document sets often arrive as several drafts. The plan gave drafts schema 3 (decision 0005 took
schema 2).

## Decision

- **The Manuscript area is always the working draft**, the one being written. Every view,
  command and agent tool works on it as before. Its name is `draft_name` ("First Draft" when
  unset).
- **New Draft keeps the working draft as it is now** as a stored draft under its old name, and
  writing carries on in the Manuscript area under the new name. A stored draft holds its own
  manuscript tree and texts in `drafts/<id>/` (format, schema 3); the rest of the project (notes,
  research, the bible, snapshots, settings) is shared by all drafts.
- **Items keep their ids from draft to draft**, so the same chapter or scene is followed and
  compared one to one; the comparison is the word-by-word one of INS-5, item by item, with
  items added and cut. When a stored draft comes back into the Manuscript area while an item of
  the same id is elsewhere in the binder (the Trash), that item of the draft takes a new id, so
  nothing is dropped.
- **Any draft can be written again** (Write This swaps it with the working draft) and **any can
  export** (`export_draft`); exports and the book view use the chosen draft, so Book typesets the
  draft the writer chose.
- **Document sets**: the latest draft becomes the working draft and earlier ones stored drafts;
  their chapters and scenes take the working draft's ids where the titles match in the same
  place, so the drafts compare scene by scene from the start.
- Every draft command is one undo step that keeps the stored drafts too.
- **Schema 3.** Version 1 and 2 projects read unchanged; a fixture project of each is tested.

## Consequences

Projects saved by 0.3 do not open in 0.2. A stored draft costs its text again in the folder.

## Revisit when

Writers want to edit a stored draft without making it the working one, or to keep drafts of
notes and the bible too.
