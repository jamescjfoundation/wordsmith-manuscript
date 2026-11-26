# 0004: The story bible in two layers

Date: 2026-11-24 · Status: accepted · Amended 2026-11-26 (the general layer is now the core's
`jcj-entries`, core decision 0016)

## Context

The story bible keeps characters, places, objects, groups, lore and plot threads beside the text.
Wordsmith Script keeps characters too, and the plan (section 5.6) settled that the bible's general
part becomes a core crate once Script uses it, built here first. Script must not depend on this
repository: it would wait on Manuscript's releases to change its own characters, and two apps
would write data one of them owns.

## Decision

- **A general entries layer** (`manuscript-model::entries`, marked `core-candidate` with
  open-source-core issue #14, C13). An entry is a record with an id, a type key, a name and
  aliases, field values by name, typed relations to other entries and a Markdown body. A template
  is data: a type key, a singular and plural name, and ordered fields of a type (text, long text,
  list, image, link). The layer gives the name list (names and aliases, longest first), lookup by
  name, the links in a text, whole-word mentions (names with a capital match case-sensitively)
  and backlinks split into links and mentions. Its interface names no manuscript concept.
- **Storage** is the project folder's `bible/<id>.md` (`docs/format.md`): type, name, aliases,
  fields and relations in TOML front matter, the body in the text dialect. A folder of such files
  can be shared by several projects (BIB-5, 1.0).
- **Manuscript's layer** on top: the six default types and their templates (SPEC 9.2) as data in
  `manuscript-model::bible`, project templates kept in settings (user-made from 0.2), the Story
  Bible area of the binder with a folder per type, the entry form and the body in the editor,
  backlinks, links from the text, the spelling word list and the inspector's linked entries.
- An entry's name and its binder title are kept the same by every command that changes either.

## Consequences

- When Script adopts entries, the general layer moves to the core as a `jcj-` crate with its tests;
  Manuscript then depends on it and keeps only its own layer.
- Templates being data means a new type (a ship, a spell) needs no code.

## Revisit when

Script's plan to use entries is written, or a second storage form (one file per bible) is needed.

## Amended 2026-11-26

The general entries layer moved to the core in core release 0.1.4 as `jcj-entries` (core issue
#14;
[core decision 0016](https://github.com/jamescjfoundation/open-source-core/blob/main/docs/decisions/0016-entries-and-translations.md)),
as "Consequences" foresaw, though before Script adopted entries. `manuscript-model::entries` is
now this app's layer over it: the core's entry and backlink types with Manuscript's ids and
text, and what reads Manuscript's text (the `wsm:` links). It carries no `core-candidate` marker
any more. Storage and Manuscript's own layer are as decided above; the entry file format stays
in this repository.
