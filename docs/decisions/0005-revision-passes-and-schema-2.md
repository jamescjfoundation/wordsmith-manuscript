# 0005: Revision passes, and schema 2 of the format

Date: 2026-11-24 · Status: accepted

## Context

Revision passes (PRG-4) mark what the writer types during a round of revision, in a color per
round, until the writer clears the marks. The marks belong to the text, so they must be saved in
the text files. The text dialect of schema 1 has no way to say "written in pass 2", and a schema 1
reader meeting an attribute it does not know reads the brackets as literal text: a project opened
in 0.1 after 0.2 marked it would show `[words]{rev=2}` in the writer's text and save it that way.
That changes the writer's words, which this app never does.

The plan (section 10, R3) gave schema 2 to drafts. Revision passes come first (R2).

## Decision

- **Schema 2** adds one span attribute, `{rev=N}`: the text was written in revision pass `N`, an
  index into the project's passes (`[extra] revisions` in `project.toml`). It nests outermost
  (revision pass, comment, link, language, small caps, bold, italic, strikethrough), takes no
  leading zeros, and is written only where text carries a pass. Every project this build writes
  declares schema 2, so 0.1 refuses it ("saved by a newer version") instead of misreading it.
- **Migration from schema 1** changes nothing: every schema 1 file is a valid schema 2 file. A
  test opens a schema 1 project and compares it with the original.
- **Drafts take schema 3** (R3); `docs/PLAN.md` says so.
- **The passes are the core's revision sets** (`jcj-features` revisions): their names and colors
  come from its color cycle, and its typing rules decide what is marked (in revision mode typed
  and pasted text takes the current pass; out of it, text typed next to revised text is not
  revised). The run mark uses the core's key, so the editor's typing marks text without any
  Manuscript code in the engine. The passes are kept for the whole project and given to each
  open editor.
- **Deletions are not marked.** The core marks a paragraph where text was deleted; nothing of the
  writer's is left there to mark, and the dialect would need a paragraph attribute for it. The
  mark is dropped when the text is written back.
- **The marks are working marks.** Exports leave them out, and so does the book view (Book never
  sees them), like comments.

## Consequences

Projects saved by 0.2 do not open in 0.1. The release notes for 0.2 say so.

## Revisit when

A writer needs deletions to stay visible after saving (a paragraph attribute in a later schema),
or the core's revision sets change shape in a minor release.
