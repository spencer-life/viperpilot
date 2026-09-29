---
title: Validate additional side-button actions on the supported mouse
labels:
    - codex-created
    - hardware-test
state: open
state_reason: null
synced_at: 2026-09-29T18:28:44.528790344Z
---

## Context

Current button evidence covers native Mouse4/Mouse5 pass-through and the exact hotkey assignments currently documented for the supported wireless model. A local mapping intent is not proof that a new action can be written.

Implementation baseline: [public PR #3](https://github.com/spencer-life/viperpilot/pull/3) (draft).

## Done when

- Name one candidate button/action and exact device identity for each experiment in private test records. Review full identity, immutable baseline, and the single-button write plan before any setter.
- Change one logical button/action at a time, independently read it back, then restore and verify the baseline before testing another action.
- Preserve raw evidence and failures privately. Publish only a redacted summary, without serials, report filenames, or local paths.
- Add only measured actions to the public capability contract and support claims. Keep unsupported actions as local draft intents. Do not use runtime input interception or injection as a fallback.

This issue does not authorize a write unless the project's hardware safety gates are satisfied.
