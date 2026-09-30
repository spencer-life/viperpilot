---
title: Resolve Windows editor presentation and input accessibility
labels:
    - accessibility
    - bug
    - codex-created
projects:
    - ViperPilot dashboard
state: open
state_reason: ""
synced_at: 2026-09-30T09:22:48.801631224Z
info:
    author: spencer-life
    created_at: 2026-09-29T18:22:45Z
    updated_at: 2026-09-30T07:11:11Z
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

## Owner-present editor smoke — 2026-09-30

The isolated editor passed window rendering, screenshot pixel checks, actual UIA Profile name/Edit, enabled Save and disabled Apply, and exit code 0 on Windows 11 Pro build 26200. An owner-focused screen capture shows readable initial profile/sensitivity controls and visible Save/disabled Apply. The first screen capture was occluded; Windows rejected automatic foreground focus, and the owner brought the preview forward for the successful capture. Full images remain private.

A dedicated temporary draft root is open for owner Save/reopen testing; that result is not recorded yet. Mouse5 needs scrolling in the initial viewport. Keyboard-only navigation, IME/paste/non-Latin names, screen-reader output, other scales/window sizes and production integration remain unverified. See [manual results](https://github.com/spencer-life/viperpilot/blob/codex/native-uia-test-client/docs/manual-handoff.md). No installed app or mouse change; issue remains open.

### Owner-present persistence result — 2026-09-30

The owner saved and closed the isolated preview. Reopening the same staged editor
with the same temporary draft library preserved `Manual Test`, DPI X/Y
`1200`/`1200` and `1000 Hz` polling. Saved JSON and actual reopened UIA values
agreed; Apply remained disabled. Keyboard, IME, screen-reader and scaling checks
remain open. No legacy app or hardware state was changed.

The owner then reported all requested controls reachable with visible focus
using Tab/Shift+Tab: profile name, DPI X/Y, Mouse4, Mouse5 and Save draft. An
owner-supplied screenshot showed Mouse5 and the pinned footer after scrolling.
This observation applies to the original isolated editor hash recorded in the
manual handoff; styling changes must preserve it and receive their own check.
IME, screen-reader speech and scaling remain unobserved.

The owner requested closer Figma alignment. A focused editor styling layer now
uses the reference rose/charcoal palette, readable labels, consistent card widths,
scrollable navigation and measured pinned footer layout. Linux offscreen renders
at normal/minimum sizes, including an invalid draft and a populated library,
were inspected. Strict preview checks and Windows cross-lint/build passed.
The newly styled Windows binary still requires owner visual/focus validation;
compact/detailed view alignment remains a separate check.
