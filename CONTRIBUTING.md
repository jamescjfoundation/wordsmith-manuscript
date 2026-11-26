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
```

`cargo xtask help` lists every task.

## Where things are

| Path | What it is |
| --- | --- |
| `crates/manuscript-model` | The project model: binder, roles, metadata, targets, bible, links |
| `crates/manuscript-format` | The project folder, the text dialect, packing, the book view |
| `crates/manuscript-io` | Import mapping, document sets, verification, exports |
| `crates/manuscript-ui` | The views, ribbon tabs and panels |
| `apps/manuscript`, `apps/manuscript-cli` | The desktop app and the command line |
| `docs/PLAN.md` | The build plan |
| `docs/SPEC.md` | How the app behaves |
| `docs/format.md` | The project folder and the text dialect |
| `docs/decisions` | Why things are the way they are |
| `tests/fixtures` | Public-domain and written-for-test inputs (`docs/fixtures.md`) |

<!-- BEGIN open-source-core: contributing -->
<!-- Owned by open-source-core (template/CONTRIBUTING.md). Change it there, never in a copy: `cargo xtask sync` rewrites the copies. -->
## Rules for a public repository

### Personal information

Never commit, or paste into an issue or pull request: anyone's name, email address or account
(other than your commit author line); local paths, user names, host names or machine details;
files from a real machine (preferences, recovery, auto-save or backup files, logs, desktop
screenshots); anyone's private writing; passwords, tokens or keys. Test fixtures are
public-domain texts or written for the test. `cargo xtask privacy` checks files and commit
identities.

### Security problems

Follow `SECURITY.md`: report privately and keep the problem out of public places until a release
fixes it.

### Private working notes

Reviews, audits and working notes go in `.private/` (ignored by Git). One is published only when
a maintainer says so, under `docs/history/`, with a first line saying it is history
([core decision 0012](https://github.com/jamescjfoundation/open-source-core/blob/main/docs/decisions/0012-working-in-the-open.md)).
Write the public part as a decision record or changelog entry, giving the reason itself rather
than "as discussed".

## Commits, pushes and pull requests

- While no app built on the core has a public release, a push to `main` is allowed once
  `cargo xtask ci` passes. A pull request is optional; its author may merge it once its checks
  pass
  ([core decision 0013](https://github.com/jamescjfoundation/open-source-core/blob/main/docs/decisions/0013-app-and-core.md)).
- A release is cut, and in the core an upstream pin is moved, only on a commit whose full gate
  has passed; a person or an agent may do either (core decisions
  [0004](https://github.com/jamescjfoundation/open-source-core/blob/main/docs/decisions/0004-follow-hold-own.md)
  and
  [0011](https://github.com/jamescjfoundation/open-source-core/blob/main/docs/decisions/0011-releases-and-versioning.md)).
- Contributions are licensed as the project is, MIT OR Apache-2.0, with no extra terms. No CLA.
- Use your GitHub no-reply address (`git config user.email <id>+<login>@users.noreply.github.com`).
- One concern per commit or pull request; say what changed, why, and how you checked it.
- Update `CHANGELOG.md` and the decision records when behavior changes. A core change that apps
  must act on says so under "What apps must do".
- AI-assisted contributions are welcome; you are responsible for every line.
<!-- END open-source-core: contributing -->

## Wordsmith Manuscript's own rules

- Private manuscripts may be used only in local runs from `.private/`, which Git ignores. Each
  fixture in `tests/fixtures` is recorded in `docs/fixtures.md`.
- Every name in the interface, help, templates and icons is our own. Do not use another product's
  names or distinctive feature names.
- Every finished step adds a line to `docs/progress.md`.

## Conduct

Follow `CODE_OF_CONDUCT.md`.
