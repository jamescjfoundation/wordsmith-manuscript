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

## Rules for a public repository

- Never commit anyone's name, email address or account (other than your commit author line),
  local paths or machine details, files from a real machine, anyone's private writing, or secrets.
  `cargo xtask privacy` checks files and commit identities.
- Test fixtures are public-domain texts or written for the test. Private manuscripts may be used
  only in local runs from `.private/`, which Git ignores.
- Every name in the interface, help, templates and icons is our own. Do not use another product's
  names or distinctive feature names.
- Follow `SECURITY.md` for security problems.

## Commits and pull requests

- Contributions are licensed as the project is, MIT OR Apache-2.0, with no extra terms. No CLA.
- Use your GitHub no-reply address.
- Say what changed, why, and how you checked it; add a line to `docs/progress.md`.
- AI-assisted contributions are welcome; you are responsible for every line.
