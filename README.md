# Wordsmith Manuscript

An open-source app for writing novels and other long work, from the J. C. J. Foundation: a binder
of parts, chapters and scenes, a corkboard and an outline for planning, a story bible beside the
text, word targets, snapshots, and exports for editors and for typesetting in Wordsmith Book.

It is built on [open-source-core](https://github.com/jamescjfoundation/open-source-core), the
foundation's shared base, alongside
[Wordsmith Script](https://github.com/jamescjfoundation/wordsmith-script).

**Status:** in development toward 0.1. The build plan is [docs/PLAN.md](docs/PLAN.md); progress is
recorded in [docs/progress.md](docs/progress.md).

## License

MIT OR Apache-2.0, at your option. Built on open-source-core, which is based on WordCraft by the
ArtCraft team (see [NOTICE](NOTICE)).

## Building

```sh
cargo run -p wordsmith-manuscript             # the desktop app
cargo run -p wordsmith-manuscript-cli -- help # the command line
cargo xtask ci                                # the full gate
```
