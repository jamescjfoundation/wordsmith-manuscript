# Attribution

Every asset in this repository that we did not write is listed here with its source and license.
`cargo xtask assets` fails if a file under `assets/` is not named in this file, and
`cargo xtask licenses` if any dependency's license is outside the allowlist (open-source-core's
decision 0001).

Add the entry in the same commit as the asset.

The logos, icons and palette card in `assets/brand/` are the Wordsmith brand kit's files (the
`brand/` folder of the wordsmith repository), written here unchanged by the kit's tool.
`assets/brand/kit.json` records the kit commit they were taken at, and `cargo xtask brand` fails
when one differs from that record. "Ours" in their rows means the project's own work, kept in
the kit.

## Assets

| Path | What it is | Source | License | Added |
| --- | --- | --- | --- | --- |
| `assets/brand/tokens.json` | Wordsmith Manuscript's design tokens | Ours | MIT OR Apache-2.0 | 2026-11-24 |
| `assets/brand/README.md` | How to use the brand assets | Ours | MIT OR Apache-2.0 | 2026-11-24 |
| `assets/brand/kit.json` | The record the brand kit's tool writes: the kit commit the brand files were taken at, and what the folder holds | Written by the Wordsmith brand kit's tool (`brand/` in the wordsmith repository) | MIT OR Apache-2.0 | 2026-11-27 |
| `assets/brand/favicon.svg` | The app icon tuned for small sizes | Ours | Trademark of The James C. J. Foundation; use unmodified (`assets/brand/README.md`) | 2026-11-24 |
| `assets/brand/icon-1024.png` | The app icon, 1024 px | Ours | Trademark of The James C. J. Foundation; use unmodified (`assets/brand/README.md`) | 2026-11-24 |
| `assets/brand/icon-512.png` | The app icon, 512 px, from the brand kit | Ours | Trademark of The James C. J. Foundation; use unmodified (`assets/brand/README.md`) | 2026-11-24 |
| `assets/brand/icon-256.png` | The app icon, 256 px, from the brand kit | Ours | Trademark of The James C. J. Foundation; use unmodified (`assets/brand/README.md`) | 2026-11-24 |
| `assets/brand/icon.ico` | The Windows program's icon (16 to 256 px in one file), from the brand kit | Ours | Trademark of The James C. J. Foundation; use unmodified (`assets/brand/README.md`) | 2026-11-24 |
| `assets/brand/icon-32.png` | The app icon, 32 px, from the brand kit | Ours | Trademark of The James C. J. Foundation; use unmodified (`assets/brand/README.md`) | 2026-11-24 |
| `assets/brand/wordsmith-manuscript-mark-ink.svg` | The mark (M and cursor), dark | Ours | Trademark of The James C. J. Foundation; use unmodified (`assets/brand/README.md`) | 2026-11-24 |
| `assets/brand/wordsmith-manuscript-mark-paper.svg` | The mark, light | Ours | Trademark of The James C. J. Foundation; use unmodified (`assets/brand/README.md`) | 2026-11-24 |
| `assets/brand/wordsmith-manuscript-wordmark-ink.svg` | The primary logo, dark | Ours | Trademark of The James C. J. Foundation; use unmodified (`assets/brand/README.md`) | 2026-11-24 |
| `assets/brand/wordsmith-manuscript-wordmark-paper.svg` | The primary logo, light | Ours | Trademark of The James C. J. Foundation; use unmodified (`assets/brand/README.md`) | 2026-11-24 |
| `assets/brand/wordsmith-manuscript-stacked-ink.svg` | The stacked logo, dark | Ours | Trademark of The James C. J. Foundation; use unmodified (`assets/brand/README.md`) | 2026-11-24 |
| `assets/brand/wordsmith-manuscript-stacked-paper.svg` | The stacked logo, light | Ours | Trademark of The James C. J. Foundation; use unmodified (`assets/brand/README.md`) | 2026-11-24 |
| `assets/brand/wordsmith-manuscript-palette.svg` | The app's color swatches | Ours | Trademark of The James C. J. Foundation; use unmodified (`assets/brand/README.md`) | 2026-11-24 |
| `assets/brand/wordsmith-and-ink-icon.svg` | The parent brand's icon | Ours | Trademark of The James C. J. Foundation; use unmodified (`assets/brand/README.md`) | 2026-11-24 |
| `assets/brand/wordsmith-and-ink-mark-ink.svg` | The parent brand's mark, dark | Ours | Trademark of The James C. J. Foundation; use unmodified (`assets/brand/README.md`) | 2026-11-24 |
| `assets/brand/wordsmith-and-ink-mark-paper.svg` | The parent brand's mark, light | Ours | Trademark of The James C. J. Foundation; use unmodified (`assets/brand/README.md`) | 2026-11-24 |
| `assets/brand/wordsmith-and-ink-imprint-ink.svg` | The imprint lockup, dark | Ours | Trademark of The James C. J. Foundation; use unmodified (`assets/brand/README.md`) | 2026-11-24 |
| `assets/brand/wordsmith-and-ink-imprint-paper.svg` | The imprint lockup, light | Ours | Trademark of The James C. J. Foundation; use unmodified (`assets/brand/README.md`) | 2026-11-24 |
| `assets/brand/wordsmith-and-ink-family-ink.svg` | The family lockup, dark | Ours | Trademark of The James C. J. Foundation; use unmodified (`assets/brand/README.md`) | 2026-11-24 |
| `assets/brand/wordsmith-and-ink-family-paper.svg` | The family lockup, light | Ours | Trademark of The James C. J. Foundation; use unmodified (`assets/brand/README.md`) | 2026-11-24 |
| `assets/i18n/es.toml` | The interface in Spanish | Ours | MIT OR Apache-2.0 | 2026-11-24 |

## Fonts

The app bundles no font files of its own. The interface uses the core's default faces (Inter and
JetBrains Mono) and the editor uses Source Serif 4, all bundled with the core under the SIL Open
Font License 1.1 and credited in the core's ATTRIBUTION.md. Every release package carries their
license texts under `licenses/assets/fonts/`, taken from the core's source when the package is
made (`cargo xtask package`).
