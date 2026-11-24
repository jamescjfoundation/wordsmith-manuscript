# Attribution

Every asset in this repository that we did not write is listed here with its source and license.
`cargo xtask assets` fails if a file under `assets/` is not named in this file, and
`cargo xtask licenses` if any dependency's license is outside the allowlist (open-source-core's
decision 0001).

Add the entry in the same commit as the asset.

## Assets

| Path | What it is | Source | License | Added |
| --- | --- | --- | --- | --- |
| `assets/brand/manuscript/tokens.json` | Wordsmith Manuscript's design tokens | Ours | MIT OR Apache-2.0 | 2026-11-24 |
| `assets/brand/manuscript/mark.svg` | The mark: a page with lines of text | Ours | MIT OR Apache-2.0 | 2026-11-24 |
| `assets/brand/manuscript/app-icon.svg` | The app icon (the mark) | Ours | MIT OR Apache-2.0 | 2026-11-24 |
| `assets/brand/manuscript/icon-32.png` | The app icon, 32 px, drawn by `tools/brand-icons.py` | Ours | MIT OR Apache-2.0 | 2026-11-24 |
| `assets/brand/manuscript/icon-256.png` | The app icon, 256 px | Ours | MIT OR Apache-2.0 | 2026-11-24 |
| `assets/brand/manuscript/icon-512.png` | The app icon, 512 px | Ours | MIT OR Apache-2.0 | 2026-11-24 |
| `assets/brand/manuscript/icon-1024.png` | The app icon, 1024 px | Ours | MIT OR Apache-2.0 | 2026-11-24 |

## Fonts

The app bundles no font files of its own. The interface uses the core's default faces (Inter and
JetBrains Mono) and the editor uses Source Serif 4, all bundled with the core under the SIL Open
Font License 1.1 and credited in the core's ATTRIBUTION.md and in the release bundles' licenses
folder.
