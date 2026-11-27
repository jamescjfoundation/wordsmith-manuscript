# 0009: The first public release is 0.9.0, in one shape with the other Wordsmith apps

Date: 2026-11-27 · Status: accepted

## Context

Manuscript's records spoke of releases nobody could find. The changelog had four sections, 0.1.0
to 1.0.0, each "prepared 2026-11-24; released when a maintainer approves it"; `Cargo.toml` said
0.1.0; the README said 1.0 was prepared; the guide said a feature needed "0.2 or later". The
repository had no release and no release tag. The four numbers were the stages of the build plan
(`docs/PLAN.md`), built in one stretch, and the plan itself still read as the list of what was
to come although every stage but the web build was done.

The release workflow written for those stages had never run, and would not have worked: it named
the macOS and Windows files in a way the update check does not find, wrote checksum lines the
updater does not read, and stopped at its first step, because the list of shipped programs named
a program where a package was wanted. The app installed an update and never started it.

Meanwhile the three Wordsmith apps had each made the same choices differently, by order of
creation and not by intent: how a release is cut, how the changelog is headed, what the
command-line program and the guide's pages are called, where the plan is kept. Someone who has
worked in one app should find the same thing in the same place in the next.

## Decision

**The first public release is 0.9.0**, a preview: a version people can download to try the app,
its downloads and its update check before 1.0.0. **1.0.0 follows it**, when the `v1` milestone
is done. The three Wordsmith apps do the same.

- **The workspace's version is 0.9.0.** 0.1.0, 0.2.0, 0.3.0 and 1.0.0 were stages of the build
  plan, never released. `CHANGELOG.md` keeps their notes under "Before the first release",
  marked as never released: they are the record of what each stage built. Where code comments,
  fixtures and older records say "0.1", "0.2" or "0.3" (`the test of v0.1`, "schema 2 came with
  0.2"), they mean those stages.
- **The changelog has the core's form.** `## Unreleased` is first. When a release is cut the
  section becomes `## <version> (<date>)` and its tag is `v<version>`; only a released version
  has a heading of that kind. The gate's `release-data` check holds the newest section to it.
- **Releases are made by the core's contract and workflow**
  ([core decision 0019](https://github.com/jamescjfoundation/open-source-core/blob/main/docs/decisions/0019-app-releases.md)):
  three files and `SHA256SUMS.txt`, named so that the update check, the launcher and the website
  find them; started by hand on `main`; the gate first, the tag made by the workflow. This
  repository has no release workflow and no packaging of its own: what a release carries is
  data in `xtask/src/main.rs`. The builds are unsigned until the organisation has certificates.
- **Every program is named after the app**: `wordsmith-manuscript` and
  `wordsmith-manuscript-cli` (it was `manuscript-cli`). Two Wordsmith apps installed into one
  folder then share no name, and the update check knows which programs are its own.
- **What is planned is the `v1` milestone** of this repository's issues. `ROADMAP.md` links to
  it and holds no list. `docs/PLAN.md` says in its first lines that it is history. A gap is
  closed by closing its issue; no file is edited to say so.

Where Manuscript differed from the other two apps by accident, it takes the common form:

| What | Before | Now |
| --- | --- | --- |
| The command-line program | `manuscript-cli` | `wordsmith-manuscript-cli` |
| The guide's page for the command line and agents | `docs/guide/command-line.md` | `docs/guide/command-line-and-agents.md` |
| The changelog | a heading for each planned version, "(prepared ...)" | `## Unreleased` first, in the core's form |
| The version | 0.1.0 under a changelog at 1.0.0 | 0.9.0 in both |
| What is planned | `docs/PLAN.md`, with finished stages reading as open | The `v1` milestone; `ROADMAP.md` links to it |
| A release | The app's own workflow and `packaging/`, started by a pushed tag | The core's shared workflow, from data |
| The brand files | Copied by hand; four icon sizes made here by a script | Written by the brand kit's tool, with `assets/brand/kit.json`; `cargo xtask brand` in the gate |

What was already the common form and stays: fixtures and golden files at the root
(`tests/fixtures/`, `tests/golden/`, each listed in `docs/fixtures.md` and `docs/golden.md`);
one switch to rewrite a reviewed file, `UPDATE_GOLDEN=1`, for the golden files and for the
guide's page of keys alike; `docs/guide/keys.md`; decision records with a date and a status.

What Manuscript keeps that the others lack: `docs/book-handoff.md` and the `book-view` tags,
because Wordsmith Book builds on this repository; and `docs/progress.md` as a log of every
finished step from the first, since its history was never split in two.

## Consequences

- Scripts and agent configurations that name `manuscript-cli` must be changed. Nothing was
  published under the old name, so only people who built from source are affected; the
  changelog says so.
- The wiki's address for the command-line page follows the guide's file name
  ([project decision 0005](https://github.com/jamescjfoundation/wordsmith/blob/main/project/decisions/0005-one-guide-per-app.md)),
  so it moves when the wiki is next built.
- A release, its tag and its changelog section agree from the first one on, because there is
  nothing earlier to agree with.
- The `book-view` tags go on as before between releases; a release tag that passed the gate and
  the Book consumer workflow is listed in `docs/book-handoff.md` as one Book may pin.
- The first plan stage's sign-offs that never happened (the specification, the contract with
  Book) are issues in the milestone, not lines in a finished plan.

## Revisit when

- The organisation has signing certificates: the guide, the README and the changelog stop
  saying the builds are unsigned, and Windows updates in place.
- The core's template gives apps a `tests/` layout or a changelog rule that differs from this.
