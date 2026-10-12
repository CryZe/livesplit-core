# Detailed Timer Icon Pull Request Visuals

These images come from LiveSplit Core's native software renderer.

- Original first-row renderer used in the fitting comparison: `a397e6ee0b9bda7620e1c0a1a9b9b7f7f37eb74d`.
- Updated renderer: `e7ee57c48c36fd9b2de642eea0b4356a62c180ee` (`Reclaim Detailed Timer Space for Missing Icons`).
- Both are based on master at `8cd6ed361adff03ae517a7c2633cefea77f3c752`.

The mode comparison shows Hidden, BothRows, and FirstRow at 360 by 140 pixels. The fitting comparison covers wide, narrow, tall, and reduced-first-row-share layouts, with identical run data, values, settings, fonts, and dimensions in each before/after pair. The reduced share uses attempt/segment heights of 23/42, approximately 35%/65%.

The missing-icon comparison shows the same name and values with and without an image in both-row and first-row modes. Duplicate run names, one with an icon and one without, provide the same run-wide reservations in each pair. Both timers retain their font sizes and positions. First-row mode gives the empty space to the name; both-row mode also translates comparison names and times together, preserving their fonts and vertical spacing. The wider example shows the reclaimed name fitting in full.

The animation shows the updated first-row and both-row modes side by side. It starts before the run, ticks from 1.11 to 8.88 seconds, splits through comparison segment durations of one second, one minute, one hour, and five seconds, ends, and resets. Alternating segments have no image. The timers remain stable while the companion text reclaims missing-icon space. Comparison values differ between segments but their reserved geometry stays unchanged.

The fixture uses an apple icon from the repository's public LiveSplit 1.0 sample run, the default embedded Core fonts, configured green attempt-timer text, and a plain dark background. Only presentation labels and backgrounds around the rendered components are composed externally; the rendered component pixels are not edited. The narrow fitting example is displayed at 2x, and the animation at 1.5x, for readability. Other screenshots are displayed at their rendered dimensions.

The temporary preview generator is not part of the feature branch. Images are hosted on this separate branch so the pull request contains only implementation and regression-test changes.
