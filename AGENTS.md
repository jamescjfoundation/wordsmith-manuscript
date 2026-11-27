# Notes for coding agents (and people)

This app is built on [open-source-core](https://github.com/jamescjfoundation/open-source-core),
The James C. J. Foundation's shared base. The core holds what every app needs; this repository
holds only what is about this app.

The blocks below marked `open-source-core` are the core's rules, which every app carries word for
word. `cargo xtask sync` writes them from the pinned core release and `cargo xtask sync --check`
fails when a copy differs, so they are changed in the core, never here. The Wordsmith project
block is owned by the project map in the wordsmith repository and is not edited here either.
Everything else is this app's own.

<!-- BEGIN wordsmith project block -->
## The Wordsmith project

Wordsmith is five repositories and two forks, all under [jamescjfoundation](https://github.com/jamescjfoundation). The full map, the steps of every route and the project's rules are in the [project map](https://github.com/jamescjfoundation/wordsmith/blob/main/project/README.md), which the wordsmith repository owns.

| Repository | Owns |
| --- | --- |
| [wordsmith](https://github.com/jamescjfoundation/wordsmith) | The project map and rules (`project/`), the brand kit (`brand/`), the website (`website/`), the wiki (`website/wiki/`), and Wordsmith Launcher (`launcher/`), which installs and updates the apps from their releases |
| [wordsmith-script](https://github.com/jamescjfoundation/wordsmith-script) | Wordsmith Script, the screenplay app, and the `.wss` file format |
| [wordsmith-manuscript](https://github.com/jamescjfoundation/wordsmith-manuscript) | Wordsmith Manuscript, the novel and long-form app, the `.wsm` project format, and the book view that Book reads |
| [wordsmith-book](https://github.com/jamescjfoundation/wordsmith-book) | Wordsmith Book, the book design and typesetting app, and the `.wsbk` project format |
| [open-source-core](https://github.com/jamescjfoundation/open-source-core) | The shared base: the `jcj-` crates and the settings and tooling every app carries. It is general and names no product |
| [wordcraft](https://github.com/jamescjfoundation/wordcraft), [craft-libs](https://github.com/jamescjfoundation/craft-libs) | Our forks of the upstream projects the core builds on |

- Each app builds on open-source-core at one release tag and carries its shared files (`cargo xtask sync`). Only the core reaches the forks.
- Wordsmith Book also builds on Wordsmith Manuscript's model and format crates, at a book-view tag. Wordsmith Manuscript reads Wordsmith Script's `.wss` files.
- The launcher reads each app's public releases by the core's release contract, so what an app's release holds must keep to that contract.
- Each app takes its logos, icons and palette from the brand kit, unchanged, by the kit's tool. Each app's `docs/guide/` is its one user guide: the wiki's pages for the app are built from it. The website and the wiki read each app's latest release from its GitHub releases.

| The change | Where it goes | Route |
| --- | --- | --- |
| A fix in a `jcj-` crate, or something any app would share | open-source-core first, then the app's pin | [A](https://github.com/jamescjfoundation/wordsmith/blob/main/project/README.md#route-a) |
| Something one app needs from another app | An issue in the providing app's repository | [B](https://github.com/jamescjfoundation/wordsmith/blob/main/project/README.md#route-b) |
| A change users will see in an app | The app's `docs/guide/` and `CHANGELOG.md`; then wordsmith builds the wiki's pages from the guide | [C](https://github.com/jamescjfoundation/wordsmith/blob/main/project/README.md#route-c) |
| A wiki page | About one app: that app's `docs/guide/`, never the wiki's copy. About the project: wordsmith, `website/wiki/content/docs/start/` | [D](https://github.com/jamescjfoundation/wordsmith/blob/main/project/README.md#route-d) |
| Website text, or the words for an app's state on the site | wordsmith, `website/tools/build.py` and `website/data/apps/` | [E](https://github.com/jamescjfoundation/wordsmith/blob/main/project/README.md#route-e) |
| A logo, icon or brand color | wordsmith, `brand/source/`; then its tool writes the copies into each app's `assets/brand/` | [F](https://github.com/jamescjfoundation/wordsmith/blob/main/project/README.md#route-f) |
| Screenshots | Drawn in the app into `docs/images/`; wordsmith's tool carries them | [G](https://github.com/jamescjfoundation/wordsmith/blob/main/project/README.md#route-g) |
| A security problem | Never public: `.private/` and a maintainer | [H](https://github.com/jamescjfoundation/wordsmith/blob/main/project/README.md#route-h) |
| A core release, or moving an app to a newer core | open-source-core, then each app | [I](https://github.com/jamescjfoundation/wordsmith/blob/main/project/README.md#route-i) |
| An app release | The app; the website and wiki pick it up by themselves | [J](https://github.com/jamescjfoundation/wordsmith/blob/main/project/README.md#route-j) |
| A change in WordCraft or craft-libs | Our fork, then the core | [K](https://github.com/jamescjfoundation/wordsmith/blob/main/project/README.md#route-k) |
| A change that spans two repositories | The provider first; each change names the other | [L](https://github.com/jamescjfoundation/wordsmith/blob/main/project/README.md#route-l) |
| Working with all the repositories, or with only one | Side by side as sibling folders | [M](https://github.com/jamescjfoundation/wordsmith/blob/main/project/README.md#route-m) |
| A rule or instruction that several repositories carry | The block's owner: the core's `template/`, or the project map for this block | [N](https://github.com/jamescjfoundation/wordsmith/blob/main/project/README.md#route-n) |

Before changing a repository, read its own `AGENTS.md`. Its rules apply inside it; this block applies in all of them. The block is owned by the project map and copied from it by a tool: it is never edited in a copy.
<!-- END wordsmith project block -->

<!-- BEGIN open-source-core: app -->
<!-- Owned by open-source-core (template/AGENTS.md). Change it there, never in a copy: its tooling rewrites the copies. -->
## Before you commit

```sh
cargo xtask install-hooks   # once: pre-commit (fmt, privacy, no dev-link in a tracked file) and pre-push (ci --changed)
cargo xtask ci --changed    # what CI will run for your change; `cargo xtask ci` for everything
cargo xtask sync            # after moving to a new core release: shared files, blocks and patch block
```

What the core owns is changed in the core, never here:

- whole files: `rust-toolchain.toml`, `rustfmt.toml`, `clippy.toml`, `deny.toml` (but for the
  app's own sources in it, which `sync` writes from `xtask/src/main.rs`), `CODE_OF_CONDUCT.md`,
  `xtask/Cargo.toml`, every file in `xtask/src/` but `main.rs` and the modules the app adds,
  `.github/workflows/ci.yml` and `core-pin.yml`, `.github/pull_request_template.md`,
  `.github/dependabot.yml` and `hooks/`;
- the blocks marked `open-source-core` in files that are otherwise the app's: in `AGENTS.md`,
  `SECURITY.md` and `CONTRIBUTING.md`, and in `Cargo.toml` (lints and build profiles),
  `.cargo/config.toml`, `.gitignore` and `.gitattributes`;
- the patch block in `Cargo.toml`.

`cargo xtask sync --check` fails if any of them differs from the pinned core release.

`xtask/src/main.rs` is the app's own and holds only what is the app's: its data (the product's
name, the programs it ships, the repositories it builds on besides the core) and its own tasks.
The gate, its list of checks and how each is run are the core's (`xtask/src/gate.rs`), so a
check the core adds runs here after `cargo xtask sync`, with no edit. The app's layers are the
table marked `layer table` in this file: `cargo xtask app-layers` reads that table, so the table
is the rule, and a layer is changed there in the same change as the code. The two workflow
files only say when CI runs: the jobs are the core's shared workflows, which they call, so a fix
to CI is made in the core and reaches the app without a change here.

## House rules for code

- Every crate root: an SPDX line, `#![forbid(unsafe_code)]`, and `#![deny(...)]` of the six
  panicking lints. Product code returns errors; it never unwraps or panics.
- Depend only on the core's `jcj-` crates and, under a contract, another app's crates
  ([core decision 0015](https://github.com/jamescjfoundation/open-source-core/blob/main/docs/decisions/0015-apps-that-depend-on-apps.md));
  never on `jcj-upstream`, `wordcraft-*` or `craft-*` (`cargo xtask layers`).
- The app's brand (its name, logo, icons, fonts and colors) lives in the app's repository, or in
  its project's brand kit with copies in the app, never in the core. The app's interface theme
  is its own `tokens.json`, which the core's theming code reads.
<!-- END open-source-core: app -->

<!-- BEGIN open-source-core: public -->
<!-- Owned by open-source-core (template/AGENTS.md). Change it there, never in a copy: its tooling rewrites the copies. -->
## What is public

This repository is public, history included, forever
([core decision 0012](https://github.com/jamescjfoundation/open-source-core/blob/main/docs/decisions/0012-working-in-the-open.md)).
Before you write a commit, branch name, issue, pull request or comment, check it holds none of:

- anyone's name, email, account names, or details of their machine (local paths such as a home
  folder, user names, host names, IP addresses);
- what the user told you in conversation, quoted or summarised; write the reason for a change as
  its own argument;
- an open security problem (follow `SECURITY.md`: keep it in `.private/` and tell the user);
- secrets, or files from a real machine (preferences, recovery, auto-save, backups, logs,
  screenshots of a desktop);
- anyone's private writing, even as test data. Fixtures are public-domain or written for the test.

Commit with a GitHub no-reply address or a foundation bot identity. Where a repository has
`cargo xtask`, `cargo xtask privacy` checks its files and every commit. Never push with `--all`
or `--mirror`.

The Foundation's name is always written "The James C. J. Foundation", with its capital "The"
wherever it stands in a sentence, never in a shorter form.
<!-- END open-source-core: public -->

<!-- BEGIN open-source-core: app-and-core -->
<!-- Owned by open-source-core (template/AGENTS.md). Change it there, never in a copy: its tooling rewrites the copies. -->
## Building between an app and the core

"The core" is [open-source-core](https://github.com/jamescjfoundation/open-source-core); "an app"
is any repository built on it. A record of the core is cited from anywhere as "core decision
0011", with its repository named, never as a bare number: an app has records with the same
numbers.

### Step 1: decide where the change belongs

Ask in order; the first "yes" decides.

1. **Does it change how we use WordCraft, craft-libs or another upstream project?** The core's
   adapters (`jcj-upstream` and the `jcj-` crate that wraps it). If upstream code itself must
   change, it goes in our fork first (the core's `docs/UPSTREAM.md` and
   [core decision 0004](https://github.com/jamescjfoundation/open-source-core/blob/main/docs/decisions/0004-follow-hold-own.md)).
2. **Would it look or work the same in any foundation app?** The window, ribbon, panels, dialogs,
   File page, preferences, auto-save, updates, saving and backups, spelling, find and replace,
   undo, format import and export, PDF and printing, cards and boards, theming, entries kept
   beside documents, interface translations, agent control: the core.
3. **Does a second app already need it, or have a written plan to use it?** The core.
4. **Otherwise** the app. Screenplay elements, Fountain, scene numbers, (MORE)/(CONT'D), a
   manuscript binder's compile rules and a book's themes are app code.

When unsure, put it in the app: moving code into the core later is cheap; taking a wrong general
interface back out is not.

| Example | Where | Why |
| --- | --- | --- |
| Redesigned ribbon, new panel docking | Core (`jcj-shell`) | Every app has a ribbon and panels |
| A new app-specific ribbon tab | The app | Only the tab's contents are the app's |
| Faster auto-save, better backups | Core (`jcj-project`, `jcj-shell`) | Same for every app |
| Index cards a second app also wants | Core (`jcj-board`) once it needs them | Second app |
| DOCX import fix | Core (`jcj-interchange`), offered upstream too | General |
| Colors, fonts, logo | The app's repository or its project's brand kit | The core holds theming code, never a brand |

### Step 2: build it

- **How a change lands**
  ([core decision 0013](https://github.com/jamescjfoundation/open-source-core/blob/main/docs/decisions/0013-app-and-core.md)):
  every change, in the core and in an app, lands on `main` through a pull request whose
  required checks pass, and its author, a person or an agent, merges it. GitHub refuses a direct
  push to `main`. No approval is asked for: run the gate before you push the branch.
- **App-only change:** one change in the app.
- **Core change an app needs:**
  1. Open a core issue describing the general need, not the app's feature.
  2. Check out both side by side. In the app: `cargo xtask dev-link ../open-source-core`
     (`cargo xtask dev-unlink` removes it). The link changes no file Git tracks: it is a folder
     Git ignores, `.cargo/dev-link/`, and `Cargo.lock` is left alone. While linked, builds,
     tests and the editor use the local core, and `cargo xtask ci --linked` runs the gate
     against it. The plain gate refuses to run linked, because CI builds the pinned release.
  3. Build the general part in the core with its own tests and a use in `apps/core-testbed`, so it
     is proven without any product.
  4. Build the app's use of it in the app.
  5. To have CI check the app's half before the core is released, give the app's branch a
     temporary pin. Push the core change (a branch will do). On the app's branch, on every
     `jcj-` line, write `rev = "<that core commit, all 40 characters>"` in place of the tag; then
     `cargo metadata --format-version 1 >/dev/null` and `cargo xtask sync`. The gate passes on
     that branch and in its pull request, with a warning. It fails on `main`, on a tag and in a
     release, so the pull request is not merged while it holds the pin.
  6. Land the core change first, with the core's gate green. Then release the core.
  7. Move the app's pin to that release's tag (`cargo xtask sync`), then land the app change,
     which names the core issue it needed.
- **Prototype first, promote later.** If the general shape is not clear yet, build it in the app,
  mark it `// core-candidate: open-source-core#N` and open that core issue, labelled
  `core-candidate`. Move it once it works and a second app wants it. A promotion is finished only
  when the app the code came from uses the core's copy and its own copy is deleted: the core
  issue stays open until then, and the marker on the app's copy names it. Never copy code
  between app repositories.
- **Something one app needs from another app**
  ([core decision 0015](https://github.com/jamescjfoundation/open-source-core/blob/main/docs/decisions/0015-apps-that-depend-on-apps.md)):
  open an issue in the providing app's repository. The provider's change lands and is tagged
  there first; the consumer then moves its pin. Never edit the provider as part of the consumer's
  change. To try the two together, the consumer names the provider in its `xtask/src/main.rs`
  (`providers`) and links a checkout of it like the core: `cargo xtask dev-link <name>=<folder>`.

### Step 3: let it reach the other apps safely

- Everything in the core is shared by default.
- A change that alters what users see in other apps goes behind an option (a shell setting or a
  Cargo feature) that defaults to the old behavior; each app turns it on when ready.
- Versioning
  ([core decision 0011](https://github.com/jamescjfoundation/open-source-core/blob/main/docs/decisions/0011-releases-and-versioning.md)):
  while at 0.x a patch release only adds or fixes; a minor release may break and its changelog
  says exactly what each app must do. A breaking change to a public `jcj-` interface says so in
  the title of its commit or pull request, and every app is updated in follow-up changes.
- A core change that breaks another app is fixed in the core, not worked around in the app.
- An app moves to the newest core before new work starts on it and before its own release. Every
  `jcj-` line names one tag. `cargo xtask core-pin` says how far behind an app is, and a
  scheduled check opens an issue when it is more than three releases behind.

### Releasing an app

One contract and one workflow for every app
([core decision 0019](https://github.com/jamescjfoundation/open-source-core/blob/main/docs/decisions/0019-app-releases.md);
the contract, with what each release file is named and holds, is the core's
[`docs/app-releases.md`](https://github.com/jamescjfoundation/open-source-core/blob/main/docs/app-releases.md)).
The gate runs first and the workflow makes the tag, as for the core.

1. Move the app to the newest core. Write what the release changes under `## Unreleased` of
   `CHANGELOG.md`.
2. `cargo xtask release X.Y.Z` makes the changes of the release commit: the version in
   `Cargo.toml` and `Cargo.lock`, and the changelog heading with the date. Run `cargo xtask ci`,
   commit, and land the commit on `main` through a pull request.
3. `gh workflow run release.yml --ref main`. It checks the release commit, runs the whole gate
   on Linux, macOS and Windows, builds and checks the package of each system and
   `SHA256SUMS.txt`, and only then makes the tag `vX.Y.Z` and publishes. A red job leaves no
   tag: land the fix on `main` and start it again.
4. To try it without releasing: `gh workflow run release.yml -f dry-run=true`. The same dry run
   starts by itself every week. `cargo xtask package` makes this computer's package in `dist/`.
   The app's whole gate also runs on macOS and Windows every night (`platforms.yml`; a failure
   opens an issue): an issue it opened is fixed before a release.

What a release carries is data in `xtask/src/main.rs` (`package`). The packaging and the
workflow are the core's: an app has no release workflow and no packaging script of its own.

### Things agents must not do

- Name WordCraft or craft-libs types in an app repository. Use the `jcj-` interfaces; if one is
  missing, add it to the core.
- Put a product name, color, logo, or screenplay, manuscript or book concept into the core's
  code, interface text or `template/`.
- Change a core interface from a change made in an app repository.
- Copy code from one app repository into another.
- Release the core, or move an upstream pin, on a commit whose full gate has not passed. A
  person or an agent may do either once it has (core decisions
  [0004](https://github.com/jamescjfoundation/open-source-core/blob/main/docs/decisions/0004-follow-hold-own.md)
  and
  [0011](https://github.com/jamescjfoundation/open-source-core/blob/main/docs/decisions/0011-releases-and-versioning.md)).
- Push a `v*` tag, or create or edit a release, by hand: in the core and in an app the release
  workflow makes both, after the gate. A tag cannot be taken back.
- Edit WordCraft or craft-libs code anywhere but our forks' topic branches.
- Edit a file or a marked block the core owns anywhere but in the core.
- Commit personal details, private files or secrets.

### Where each kind of record goes

| Record | Lives in |
| --- | --- |
| Decisions about shared interfaces, upstream, tooling | The core's `docs/decisions/` |
| Decisions about one app's behavior | That app's `docs/decisions/` |
| Pins, carried patches, follow / hold / own | The core's `docs/UPSTREAM.md` |
| What changed for apps in each core release | The core's `CHANGELOG.md`, with "What apps must do" |
| Ideas waiting to be promoted to the core | Core issues labelled `core-candidate` |
| A request to another app | An issue in that app's repository |
| Reviews, audits, working notes | `.private/`, or `docs/history/` once a maintainer publishes one |
<!-- END open-source-core: app-and-core -->

## Wordsmith Manuscript's own rules

Behavior comes from `docs/SPEC.md`; the files are specified in `docs/format.md`. Decisions about
this app go in `docs/decisions/`. Every finished step adds a line to `docs/progress.md`, which
is this repository's record of work. `CHANGELOG.md` is for writers: a step whose result a
writer will notice also gets its line there, under `## Unreleased` until the release that
carries it.

**What is planned** is the
[`v1` milestone](https://github.com/jamescjfoundation/wordsmith-manuscript/milestone/1) of this
repository's issues; `ROADMAP.md` links to it and holds no list. Work you find and will not do
now is an issue there (no milestone when it is for after 1.0.0), never a line in a file.
`docs/PLAN.md` is the plan the app was built from: finished, kept as history, and not the place
to look for what is open. The first public release is 0.9.0, a preview, and 1.0.0 follows it;
the plan's stages 0.1 to 1.0 were never released (decision 0009).

**The programs** are `wordsmith-manuscript` (the desktop app) and `wordsmith-manuscript-cli`
(the command line), each named like its package. What a release carries is the `package` data
in `xtask/src/main.rs`; `apps/manuscript/src/main.rs` gives the update check the same release
id and the window the same app id, and a test on each side holds them. A release is cut as the
core's block above says ("Releasing an app"). The builds are unsigned until the organisation
has certificates (issue 20).

1. **Our own names and design.** Every interface name, menu label, feature name, template and
   icon is our own: never another product's name or its distinctive feature name, even where the
   word would fit, and the app is never described as replacing, following or being inspired by
   another product. Generic vocabulary (binder, corkboard, outline, inspector, snapshot, target,
   keyword, label, status, collection) is fine. Code, documents, commit messages and fixtures
   may name another company, program, service or file format where that is a fact a reader
   needs: what the app opens or writes, where a convention comes from. People are never named.
   This is the project's rule
   ([project decision 0003](https://github.com/jamescjfoundation/wordsmith/blob/main/project/decisions/0003-naming-other-companies-programs-and-formats.md)).
   Where the spec is silent, design it fresh and record a decision.
2. **The writer's words are sacred.** Nothing the writer typed is dropped or rewritten by saving,
   importing, exporting, merging or splitting. Every importer and exporter proves it with a
   word-for-word test. Text is never "cleaned up" without the writer asking.
3. **Golden files are the judge.** Standard manuscript PDF and DOCX output and the format's round
   trips are compared with reviewed reference files. A change that alters them is deliberate,
   reviewed and recorded in `docs/golden.md`. One switch writes a reviewed file again,
   `UPDATE_GOLDEN=1`: the golden files in `tests/golden/` and the guide's page of keys
   (`docs/guide/keys.md`, made from the commands).
4. **Fixtures** are public-domain texts or written for the test, each recorded in
   `docs/fixtures.md`. One written for another Wordsmith repository's tests (Script's `.wss`
   files, one per format version) may be copied unchanged, with where it came from recorded.
   Private manuscripts stay in `.private/`.
5. **The brand is the kit's.** The logos, the icons at every size and the palette card in
   `assets/brand/` are files of the Wordsmith brand kit (the `brand/` folder of the wordsmith
   repository), byte for byte. They are never edited, recolored, redrawn or resized here: a
   change starts in the kit, and the kit's tool writes its files into this folder
   ([Route F](https://github.com/jamescjfoundation/wordsmith/blob/main/project/README.md#route-f);
   [project decision 0007](https://github.com/jamescjfoundation/wordsmith/blob/main/project/decisions/0007-brand-files-reach-the-apps-by-one-route.md)).
   - **To take the kit's files:** with the wordsmith repository checked out beside this one,
     `npm run brand -- --app manuscript` in its `website/wiki`. It writes the files and
     `assets/brand/kit.json`, the record of the kit commit they were taken at and of what the
     folder holds; commit the result here, naming the wordsmith commit. The record is never
     edited by hand.
   - **Manuscript's own files** in the folder are `tokens.json`, the interface theme in the
     core's token format (decision 0008), whose `palette` must hold the kit's seven colors, and
     `README.md`. A new file of the app's own is named in the kit's list first
     (`brand/apps.json` there, by an issue in the wordsmith repository).
   - **`cargo xtask brand`**, in the gate, fails when a carried file is not the one the record
     names, when the folder holds a file the record does not list, and when the theme's palette
     is not the kit's. **`cargo xtask brand --kit`** asks the wordsmith repository whether the
     kit has moved since; the workflow `brand-kit.yml` runs it every week and is red until the
     files are taken again.
   - Every file under `assets/` is listed in `ATTRIBUTION.md` (`cargo xtask assets`).
   - **Today:** a red `brand-kit.yml` run opens no issue; someone has to look at the Actions
     page, or at the wordsmith repository's own issue for the change.

### Manuscript and the other apps

Two other Wordsmith apps meet this one
([project decision 0004](https://github.com/jamescjfoundation/wordsmith/blob/main/project/decisions/0004-apps-that-depend-on-apps.md),
[Route B](https://github.com/jamescjfoundation/wordsmith/blob/main/project/README.md#route-b)).

- **Wordsmith Book builds on Manuscript.** Book depends on `wordsmith-manuscript-model` and
  `wordsmith-manuscript-format` at a tag of this repository, and reads `.wsm` and `.wsmz`
  projects through them. The contract is `docs/book-handoff.md`, which this repository owns. It
  lists every item of the two crates Book may use, what the book view gives, and the formats
  and versions Book reads and writes. Anything not listed may change without notice.
  - A change to either crate, to the project schema or to the book view can break Book. Two
    checks say so before it lands. The contract test,
    `crates/manuscript-format/tests/book_contract.rs`, is in the gate: it uses every listed
    item the way Book does and fails when its list and the document's differ. The workflow
    `book-consumer.yml` runs Book's own gate against this checkout when a change touches the
    two crates or the root `Cargo.toml`, and when run by hand.
  - An item joins the contract by its row in the document and its use in the test, in one
    change. For anything Book must follow, open an issue in the Book repository.
  - What Book needs from Manuscript arrives as an issue in this repository. The change is made
    here, as Manuscript's own change; a change in Book never edits this repository.
  - **Tags.** A tag Book may pin is named `book-view-<view version>.<count>`: the view version
    is `book::VERSION`, and the count rises by one with each tag (`book-view-1`,
    `book-view-1.1`, ...). Cut the next one when Manuscript moved to a newer core (Book holds
    one core release, so it can follow only through a tag), when the contract changed, or when
    a change Book asked for landed. A person or an agent cuts it, annotated, on a commit of
    `main` where the full gate and the Book consumer workflow both passed, and adds its row to
    the table in `docs/book-handoff.md`. A tag is never moved or deleted: the repository's
    ruleset refuses it for `book-view-*` and `v*`. The `book-view` tags are not releases; a
    release tag (`v0.9.0` is the first) that is listed in that table is one Book may pin too.
  - A change that breaks a listed item or the book view raises the view's version
    (`book::VERSION` in `crates/manuscript-format/src/book.rs`), is recorded in
    `docs/book-handoff.md` and `CHANGELOG.md` with what Book must do, and needs a follow-up in
    Book. A new project schema is a change Book must follow as well: the document says how.
  - **Today:** the contract's list is kept in step with Book's code by hand. Right after a core
    move the workflow moves its own copy of Book to the new core (the pin move and
    `cargo xtask sync`, nothing else) and says so in its summary; when Book needs more than
    that, it can pass only against a branch of Book that has moved (its `book-ref` input). Nothing makes a release of Manuscript that writes a new schema wait for a Book that
    reads it. A release tag joins the contract's table only when someone adds its row.
- **Manuscript reads Wordsmith Script's `.wss` files** (`crates/manuscript-io/src/screenplay.rs`)
  for screenplays kept in a project. Script owns the format (`docs/format.md` in the Script
  repository) and numbers its versions; the reader here is Manuscript's own.
  - The reader accepts format 1 up to the version it was written for
    (`screenplay::NEWEST_FORMAT`) and refuses a newer file with a message naming both versions.
    It never guesses at a format it does not know: a wrong guess shows the writer wrong words.
  - A new format version arrives as an issue in this repository
    ([Route B](https://github.com/jamescjfoundation/wordsmith/blob/main/project/README.md#route-b)).
    The change that answers it reads Script's `docs/format.md`, teaches the reader the version,
    raises `NEWEST_FORMAT` and adds that version's fixture, in one change.
  - `tests/fixtures/scripts/` holds one file per format version the reader accepts and one for
    the first version it refuses (`docs/fixtures.md`);
    `crates/manuscript-io/tests/screenplay.rs` fails when a version has no file.
  - What Manuscript needs of the format is asked for in an issue in the Script repository. Code
    is never copied from Script; its fixtures may be, with the source recorded.
  - **Today:** nothing tells this repository when Script adds a version except that issue.
    Until the reader is taught the version, a script saved by the newer Script shows the
    refusal instead of its text.

### Where each crate's code goes

<!-- BEGIN layer table (cargo xtask app-layers reads it) -->
| Crate | Layer | Holds | May depend on |
| --- | --- | --- | --- |
| `manuscript-model` | 1 | Project, binder, roles, metadata, targets, bible entries, links, the semantic text of an item | `jcj-doc`, `jcj-entries` (only here) |
| `manuscript-format` | 2 | The project folder, the text dialect, migrations, limits, packing, the read-only book view | model, `jcj-project` |
| `manuscript-io` | 3 | Import mapping, document sets, exports with their word-for-word check (the core's `verify`), the standard manuscript layout, the `.wss` reader | format, model, `jcj-interchange`, `jcj-layout`, `jcj-output`, `jcj-features`, `jcj-proof` |
| `manuscript-ui` | 4 | Views, commands, ribbon tabs, panels, agent tools | io, format, model, `jcj-engine`, `jcj-shell`, `jcj-theme`, `jcj-board`, `jcj-agents` |
| `apps/manuscript`, `apps/manuscript-cli`, `apps/manuscript-web` | 5 | The desktop app, the command line, a placeholder for the web build | the crates above |
<!-- END layer table -->

`cargo xtask app-layers` reads this table, so the table is the rule; there is no second copy in
code. A row is a layer, lowest first. A crate depends only on crates of the rows before its own,
and uses a core crate only when its own row or an earlier one names it in backticks:
`manuscript-io` may use what `manuscript-format` may, and so on up. `(only here)` keeps a crate
to its row: only `manuscript-model` uses `jcj-entries`. A crate of ours or of the core that the
table does not allow is a failure; other libraries are not its business. Every package of the
workspace but `xtask` is in one row, the programs in `apps/` included.
`manuscript-model` and `manuscript-format` never depend on interface or engine code: Wordsmith
Book builds on them.

### Code that waits for the core

`docs/core-candidates.md` lists every core change Manuscript has asked for and every
`// core-candidate:` marker here, with its core issue and state. Section 5.4 of the finished
plan (`docs/PLAN.md`) is the list as the plan was written; the ledger is what is true now.

- A marker is written `core-candidate: open-source-core#N` and names an open core issue that
  says what the core would need to offer. `cargo xtask docs` fails on a marker with no issue.
- A marker, its row in the ledger and its issue change together. When the core releases what a
  marker waits for, move onto the core's code and delete the copy here in the same change that
  moves the pin, or say in the ledger exactly why the copy stays and which open issue would
  free it.
- **Today:** nothing checks the ledger against the markers or the issues; it is kept by hand
  (issue 29).
