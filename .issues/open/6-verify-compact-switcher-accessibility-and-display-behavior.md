---
title: Verify compact switcher accessibility and display behavior
labels:
    - accessibility
    - codex-created
    - release-gate
state: open
state_reason: null
synced_at: 2026-09-29T18:28:49.047907042Z
---

## Context

The compact native switcher and tray flow need verification on an interactive Windows desktop. Synthetic actions and builds cannot establish keyboard, screen-reader, or real display behavior.

Implementation baseline: [public PR #3](https://github.com/spencer-life/viperpilot/pull/3) (draft).

## Done when

- Verify tray menu and compact/details labels, focus order, keyboard activation, accessible names/roles/patterns, and screen-reader announcements. Fix any control that cannot be used or understood without a mouse.
- Review legibility and hit targets at the display scales and narrow-window sizes used for the app, including reversed saved pairs, disconnected/unknown device state, and unavailable/corrupt library states. UI-only checks must not apply a profile.
- Record conditions, failures, and fixes in repository documentation. Publish no unredacted screenshots or device-identifying information. Keep the release gate open until the production UI passes.
