---
title: Resolve Windows editor presentation and input accessibility
labels:
    - accessibility
    - bug
    - codex-created
state: open
state_reason: null
synced_at: 2026-09-30T07:11:20.708001134Z
---

## Context

The optional settings editor needs real Windows desktop validation. A prior preview produced a mismatch between desktop capture and renderer output, and its keyboard/input behavior has not been fully established. Determine whether this is a display problem or a capture/compositor limitation before production integration.

Implementation baseline: [public PR #3](https://github.com/spencer-life/viperpilot/pull/3) (draft).

## Done when

- Observe the editor on an actual Windows monitor and distinguish a visible rendering problem from a screenshot/compositor limitation. Compare capture methods and record test conditions without publishing unredacted screenshots.
- Verify keyboard focus/activation, screen-reader names/roles, IME composition, paste, non-Latin profile names, high-DPI behavior, and narrow-window layout in Quick edit and All settings.
- Confirm disabled Apply cannot be activated and draft status remains clear.
- Update public evaluation notes with passing evidence or exact remaining blockers. Keep full screenshots and environment-specific records in private local records; publish only a redacted summary.

## Checkpoint — 2026-09-29

The optional editor's Windows presentation and real input/accessibility behavior remain unverified. The user is away from the Windows PC, so monitor, IME, paste, screen-reader, and DPI checks are deferred. Keep the production integration gate open.

## Software readiness — 2026-09-30

Independent source/UI audit found and fixed unnamed duplicate input and indistinct side-button controls. Headless tests confirm associated names/roles and duplicate-name editing; all 11 editor tests and strict preview checks pass. Actual Windows monitor/capture, screen reader, keyboard/IME/paste and scaling remain unobserved. See [manual handoff](https://github.com/spencer-life/viperpilot/blob/codex/ui-readiness/docs/manual-handoff.md). Installed app unchanged; no GUI/HID or startup operation ran. Issue remains open for its manual and delivery gates.
