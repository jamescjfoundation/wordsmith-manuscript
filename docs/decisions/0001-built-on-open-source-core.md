# 0001: Built on open-source-core

Date: 2026-11-24. Status: accepted.

## Context

Wordsmith Manuscript is the foundation's second desktop app. The first, Wordsmith Script, already
proved the shell, saving, backups, recovery, spelling, undo, format conversion, PDF, cards and
agent control, and those pieces now live in open-source-core for every app to share. A
long-form writing app needs all of them, plus a project made of many documents.

## Decision

- **The core gives the general parts.** The repository starts from the core's `template/` and
  uses only its `jcj-` crates, pinned at one release tag (`v0.1.1` at the start). The window,
  ribbon, panels, File page, preferences, atomic saves, backups, recovery, spelling, undo, format
  import and export, PDF, cards and boards, theming and agent control are the core's. This
  repository holds only what is about writing a manuscript.
- **Four crates and three apps**, in layers (plan section 5.2): `manuscript-model` (no files, no
  interface), `manuscript-format` (the project folder; free of interface and engine code because
  Wordsmith Book reads it), `manuscript-io` (import, export, verification), `manuscript-ui` (the
  views), and `apps/manuscript`, `apps/manuscript-cli`, `apps/manuscript-web` (a placeholder).
  `cargo xtask app-layers` checks the table.
- **A project is a folder of plain text files** (plan section 6): `project.toml`, `binder.toml`,
  one Markdown file per text item and per bible entry, media beside them. One change to one scene
  changes one file, so projects work with version control and synced folders and stay readable if
  the app disappears. A packed, deterministic zip of the folder (`.wsmz`) is the single-file form
  and the backup form. Wordsmith Script made the same choice for the same reason: its `.wss`
  stays readable text.
- **Shared ideas start here.** Pieces a second app may want (the binder tree, the outline table,
  the stitched view, the verifier, the entries layer of the story bible) are built here first,
  marked `// core-candidate:`, and move to the core once they work and a second app wants them.

## Consequences

- Core fixes reach this app by moving the pin; `cargo xtask sync --check` keeps the shared
  settings identical to the pinned release.
- The app cannot change a core interface; a missing one is a core change (plan section 5.4).
- Saving many small files needs care: each file is written atomically and the binder last, so an
  interrupted save leaves the previous tree pointing at whole files.

## Revisit when

The core changes its project container in a way that serves many-document projects directly, or a
third app needs the same folder format.
