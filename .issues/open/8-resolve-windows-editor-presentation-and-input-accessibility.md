---
title: Resolve Windows editor presentation and input accessibility
labels:
    - accessibility
    - bug
state: open
state_reason: null
synced_at: 2026-09-29T18:22:46.176477268Z
---

## Context

The optional settings editor needs real Windows desktop validation. A prior preview produced a mismatch between desktop capture and renderer output, and its keyboard/input behavior has not been fully established. Determine whether this is a display problem or a capture/compositor limitation before production integration.

Implementation baseline: [public PR #3](https://github.com/spencer-life/viperpilot/pull/3) (draft).

## Done when

- Observe the editor on an actual Windows monitor and distinguish a visible rendering problem from a screenshot/compositor limitation. Compare capture methods and record test conditions without publishing unredacted screenshots.
- Verify keyboard focus/activation, screen-reader names/roles, IME composition, paste, non-Latin profile names, high-DPI behavior, and narrow-window layout in Quick edit and All settings.
- Confirm disabled Apply cannot be activated and draft status remains clear.
- Update public evaluation notes with passing evidence or exact remaining blockers. Keep full screenshots and environment-specific records in private local records; publish only a redacted summary.
