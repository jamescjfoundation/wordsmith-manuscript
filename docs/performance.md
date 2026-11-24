# Performance

The scale gates of plan section 11, measured by tests in the repository. Numbers are from the
build machine; CI records its own.

| Measure | Budget | Measured | Test | Date |
| --- | --- | --- | --- | --- |
| Typing in a stitched view of 200,000 words (400 items), median per frame | 16 ms (release, CI machine) | 0.6 ms (debug build) | `manuscript-ui/tests/stitch.rs`, `a_200000_word_view_types_within_the_frame_budget` | 2026-11-24 |
| A frame of the binder with 1,000 items | 50 ms (debug test) | under 50 ms | `manuscript-ui/tests/binder.rs`, `a_thousand_items_stay_responsive` | 2026-11-24 |

Only the paragraphs in view are laid out; laid-out paragraphs are kept until they change, and the
heights of the rest are estimated from their length until they come into view.
| Project search on a 563,000-word project in 1,000 scenes | 200 ms | about 2 ms a query (index built in the background in about 120 ms; debug build) | `manuscript-ui/tests/search.rs`, `a_300000_word_project_answers_within_200_ms` | 2026-11-24 |
