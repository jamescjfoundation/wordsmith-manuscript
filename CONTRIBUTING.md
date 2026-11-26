# Contributing

Thank you for helping. This repository is public: everything you commit, push, or write in an
issue, pull request or review can be read by anyone and stays in the history for good.

## Building and checking

```sh
cargo xtask install-hooks        # once: pre-commit and pre-push hooks
cargo xtask ci                   # the full gate (CI runs the same)
cargo xtask ci --changed         # only the packages your change affects, and their dependents
cargo run -p wordsmith-manuscript                # the desktop app
cargo run -p wordsmith-manuscript-cli -- help    # the command line
cargo xtask screenshots                          # draw the README's pictures again (docs/images/)
```

`cargo xtask help` lists every task. `AGENTS.md` has the rules: the project's, the core's and
Manuscript's own.

## Where things are

| Path | What it is |
| --- | --- |
| `crates/manuscript-model` | The project model: binder, roles, metadata, targets, bible, links |
| `crates/manuscript-format` | The project folder, the text dialect, packing, the book view |
| `crates/manuscript-io` | Import mapping, document sets, verification, exports |
| `crates/manuscript-ui` | The views, ribbon tabs and panels |
| `apps/manuscript`, `apps/manuscript-cli` | The desktop app and the command line |
| `apps/manuscript-web` | A placeholder for the web build, which waits for the core |
| `assets/brand` | Copies from the Wordsmith brand kit, and the app's own theme and derived icons (`AGENTS.md`, own rule 5) |
| `assets/i18n` | The interface's translations |
| `packaging` | What the release needs on Linux, macOS and Windows |
| `tools` | Scripts that make the fixtures and the derived icons |
| `xtask` | The gate; the shared modules in it are the core's |
| `docs/PLAN.md` | The build plan |
| `docs/SPEC.md` | How the app behaves |
| `docs/format.md` | The project folder and the text dialect |
| `docs/book-handoff.md` | The contract with Wordsmith Book, which builds on the model and format crates |
| `docs/guide` | The user guide |
| `docs/decisions` | Why things are the way they are |
| `docs/core-candidates.md` | Every core change asked for and every `core-candidate` marker, with its core issue and state |
| `docs/progress.md` | One line per finished step |
| `tests/fixtures`, `tests/golden` | Public-domain and written-for-test inputs (`docs/fixtures.md`); reviewed reference outputs (`docs/golden.md`) |

<!-- BEGIN open-source-core: contributing -->
<!-- Owned by open-source-core (template/CONTRIBUTING.md). Change it there, never in a copy: its tooling rewrites the copies. -->
## Rules for a public repository

### Personal information

Never commit, or paste into an issue or pull request: anyone's name, email address or account
(other than your commit author line); local paths, user names, host names or machine details;
files from a real machine (preferences, recovery, auto-save or backup files, logs, desktop
screenshots); anyone's private writing; passwords, tokens or keys. Test fixtures are
public-domain texts or written for the test. The repository's privacy check
(`cargo xtask privacy` where it has `cargo xtask`) checks files and commit identities.

### Security problems

Follow `SECURITY.md`: report privately and keep the problem out of public places until a release
fixes it.

### Private working notes

Reviews, audits and working notes go in `.private/` (ignored by Git). One is published only when
a maintainer says so, under `docs/history/` where the repository has a `docs/` folder, with a
first line saying it is history
([core decision 0012](https://github.com/jamescjfoundation/open-source-core/blob/main/docs/decisions/0012-working-in-the-open.md)).
Write the public part as a decision record or changelog entry, giving the reason itself rather
than "as discussed".

## Commits, pushes and pull requests

- While no app built on the core has a public release, a push to `main` is allowed once the
  repository's gate passes (`cargo xtask ci` where it has `cargo xtask`). A pull request is
  optional; its author may merge it once its checks pass
  ([core decision 0013](https://github.com/jamescjfoundation/open-source-core/blob/main/docs/decisions/0013-app-and-core.md)).
- A release is cut, and in the core an upstream pin is moved, only on a commit whose full gate
  has passed; a person or an agent may do either (core decisions
  [0004](https://github.com/jamescjfoundation/open-source-core/blob/main/docs/decisions/0004-follow-hold-own.md)
  and
  [0011](https://github.com/jamescjfoundation/open-source-core/blob/main/docs/decisions/0011-releases-and-versioning.md)).
- Contributions are licensed as the repository's license files say (MIT OR Apache-2.0 for
  code), with no extra terms. No CLA.
- Use your GitHub no-reply address (`git config user.email <id>+<login>@users.noreply.github.com`).
- One concern per commit or pull request; say what changed, why, and how you checked it.
- A pull request opened by Dependabot is merged by anyone with write access once its checks
  pass.
- Update the changelog and the decision records, where the repository keeps them, when behavior
  changes. A core change that apps must act on says so under "What apps must do".
- AI-assisted contributions are welcome; you are responsible for every line.
<!-- END open-source-core: contributing -->

## Wordsmith Manuscript's own rules

These add to the block above; `AGENTS.md` has them in full.

- Private manuscripts may be used only in local runs from `.private/`, which Git ignores. Each
  fixture in `tests/fixtures` is recorded in `docs/fixtures.md`.
- Every interface name, menu label, feature name, template and icon is our own, never another
  product's name or its distinctive feature name. Naming another program or file format to state
  a fact, such as what the app opens, is fine
  ([project decision 0003](https://github.com/jamescjfoundation/wordsmith/blob/main/project/decisions/0003-naming-other-companies-programs-and-formats.md)).
- Every finished step adds a line to `docs/progress.md`. That is beside the block's
  `CHANGELOG.md` line, not instead of it: the changelog is for what a writer will notice.
- Wordsmith Book builds on `manuscript-model` and `manuscript-format`. Check a change to them,
  to the project schema or to the book view against `docs/book-handoff.md` before it lands. The
  contract test in the gate and the "Book consumer" workflow, which runs Book's own gate against
  the change, do most of that.

## Conduct

Follow `CODE_OF_CONDUCT.md`.
