# Detailed Timer Pull Request Images

These screenshots and animations were generated with LiveSplit Core's native software renderer, using identical fixture runs, settings, fonts, and component dimensions for each before-and-after pair.

- Before: upstream master at `fb62dd42505b16e813d2779468d81bbad7f01b5e`.
- After: `detailed-timer-auto-fit` at `324c23ff8f53f44d90d7e7f4d6d97ceb19ce0dbe`.

The preview generator is temporary and is not part of the feature branch. These assets are kept on a separate branch so the pull request contains only implementation changes and regression tests.

The start-and-splits animation uses comparison segment times ranging from one second to one hour. The comparison reservation uses the exact maximum known segment value, without a forecast buffer. Live timer hints retain their existing 20% buffer.

The ticking animation advances from seconds through one hour without changing the expected time hints. The comparison pixels are identical in every source frame.

Screenshots in the first figure are displayed at their rendered dimensions, except for the narrow 140 × 78 example, displayed at twice its rendered size. The start-and-splits animation renders 300 × 140 components displayed at 1.5×; the ticking animation renders a 420 × 150 component displayed at 2×. Scaling is only for presentation; no layout or text is edited in the rendered components.
