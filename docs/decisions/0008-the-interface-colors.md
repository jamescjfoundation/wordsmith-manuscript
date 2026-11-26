# 0008: The interface colors

Date: 2026-11-24 · Status: accepted

## Context

The brand (`assets/brand/README.md`) gives six colors: Iron Gall Ink #1C2340 (logos, text), Page
White #FAFAF7 (pages), Card Red #D24B3E (#A8322A for text; the cursor and small accents only, one
per screen), Margin Blue #3B4FA8 (links, buttons, focus), Slate #5A6080 (secondary text) and
Night #12162B (dark backgrounds). `assets/brand/tokens.json` still held the placeholder claret
and brass palette. The core's theme (`jcj-theme`, TOKENS.md) asks each app for fourteen roles in
a light, a dark and an extra-dark theme, and the editor has its own page colors (light, dark and
sepia).

## Decision

**The roles.** The palette is written once under `palette` and the light theme names it:

| Role | Light | Dark | Extra dark | Why |
| --- | --- | --- | --- | --- |
| `canvas` | #ECEDF1 | Night | #080A14 | Behind the pages: a cool gray in light, the brand's Night in dark |
| `surface` | Page White | #191E36 | #0D1020 | Ribbon and panels: paper in light; one step up from Night |
| `surface_high` | #FFFFFF | #20264A | #13172D | A further step, in the same blue-black hue |
| `input` | #FFFFFF | #151A30 | #0A0D1A | Fields sit just below the surface in dark themes |
| `border` | #D8DAE3 | #2F365A | #222842 | Quiet lines in the ink hue |
| `text` | Iron Gall Ink | #E6E7EE | #D6D8E2 | Ink on paper; a cool near-white on Night |
| `text_dim` | Slate | #A3A9C4 | #8F95B0 | Slate, lightened for dark grounds |
| `accent` | Margin Blue | #93A2F2 | #8392E6 | Links, current tab and focus are Margin Blue, lightened to stay readable on Night |
| `accent_soft` | #E2E6F5 | #2A3366 | #1C2349 | Selection and controls that are on: a wash of Margin Blue |
| `hover` | #EEEFF3 | #252B4D | #181D37 | Between surface and `accent_soft` |
| `danger` | #A3213F | #F38BA0 | #E87B91 | A crimson, see below |
| `brand` | Iron Gall Ink | #2B3466 | #202752 | The File button and File page are the icon's ink tile |
| `on_brand` | Page White | Page White | #EDEDE8 | Paper on ink, as in the logo |
| `highlight` | Card Red | #E2604F | #D8574A | The one red mark on screen |

**Card Red appears once.** The core paints `highlight` only as the small bar beside the current
File page entry, so that is the one place Card Red goes; `brand` is ink, not red, because the File
button is on screen all the time and the README keeps Card Red to one small accent. The editor's
caret stays in the page's text color (`editor/draw.rs`): this change is about the theme, and a
red caret is a separate choice about the writing screen, left for its own decision.

**Danger is not Card Red.** A problem must never read as the brand accent, or the brand accent as
a problem. Danger is a crimson (hue about 346 degrees against Card Red's 5) and, in the light
theme, much darker than Card Red; in the dark themes it is a light rose, against the warmer
highlight. A test keeps the two at least 15 degrees of hue apart in every theme.

**Contrast.** Every text role (`text`, `text_dim`, `accent`, `danger`) is at least 4.5:1 against
every ground it sits on (canvas, surface, surface_high, input, hover) in all three themes, as is
text and accent on `accent_soft` and `on_brand` on `brand`; the focus ring (`accent`) is therefore
well over the 3:1 non-text minimum, and the highlight bar is at least 3:1 against `brand`. The
lowest pairs: Slate on hover (light) 5.3:1, accent on accent_soft (dark) 4.9:1, highlight on
brand (dark) 3.4:1. Borders stay quiet (about 1.3:1, as before): fields are also told apart by
their fill, and focus is shown by the accent. `brand.rs` tests every pair, so a later edit to
`tokens.json` cannot quietly fall below AA.

**The editor's pages.** Light is the brand's page: Page White with Iron Gall Ink text, dim marks
in a light slate (#868BA3, 3.2:1, for dividers and hints, not text) and a Margin Blue selection
(#DCE2F6). Dark is a page one step above Night (#171B31), cool near-white text, a slate dim
(#7D83A0) and a deep Margin Blue selection (#2F3A72). Sepia is unchanged (#F4ECD8, warm brown
text): it is the warm page by design, offered in any theme. Text is at least 7:1 on every page
and 4.5:1 on its selection.

## Consequences

- The interface reads as the brand: ink and paper, blue for what can be clicked, one red mark.
- No golden file changes: the reference PDF and DOCX outputs carry no interface colors, and the
  app has no screenshot references.
- A brand that changes its colors edits `tokens.json` and keeps the tests in `brand.rs` passing.

## Revisit when

Someone proposes a Card Red caret in the editor (the logo's cursor on the page, where no other red
shows), or the core paints `highlight` in more places than the File page.
