# Detailed Timer Icon Pull Request Visuals

These images come from LiveSplit Core's native software renderer. The comparison pairs use identical run data, component dimensions, font settings, and first-row icon mode.

- Original first-row renderer: `a397e6ee0b9bda7620e1c0a1a9b9b7f7f37eb74d` (`Add Detailed Timer IconDisplayMode`, before the automatic fitting change).
- Final renderer: `7a1466e1e7ad37a55a27a75ca3178fbcdbeac3f4` (`Fit and Align Detailed Timer Icons`).
- Both commits are based on master at `8cd6ed361adff03ae517a7c2633cefea77f3c752`.

The mode comparison shows Hidden, BothRows, and FirstRow using the final renderer at 360 by 140 pixels. The fitting comparison covers wide, narrow, tall, and reduced-first-row-share layouts. The reduced share uses attempt/segment heights of 23/42, approximately 35%/65%.

The animation starts before the run, ticks from 1.11 to 8.88 seconds, splits through comparison segment durations of one second, one minute, one hour, and five seconds, ends, and resets. Alternating segments have no image, so the final renderer's reserved icon column can be seen to stay in place even while the image is absent. The attempt timer and both comparison names vary independently of the fixed sizing reservations.

The fixture uses an apple icon from the repository's public LiveSplit 1.0 sample run, the default embedded Core fonts, configured green attempt-timer text, and a plain dark background. Only presentation labels and backgrounds around the rendered components are composed externally; the rendered component pixels are not edited. The narrow example is displayed at 2x, and the animation at 1.5x, for readability. Other screenshots are displayed at their rendered dimensions.

The preview generation code is temporary and is not part of the feature branch. Images are hosted on a separate branch so the pull request contains only implementation and regression-test changes.
