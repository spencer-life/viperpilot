---
title: Measure tray-only resource use and quick-switch latency
labels:
    - codex-created
    - enhancement
    - release-gate
state: open
state_reason: ""
synced_at: 2026-09-30T03:20:27.046207446Z
info:
    author: spencer-life
    created_at: 2026-09-29T18:22:47Z
    updated_at: 2026-09-29T21:45:08Z
---

## Context

Fast direct switching and low background overhead are product priorities. The current build needs repeatable tray-only and production quick-switch measurements before performance claims are made.

Implementation baseline: [public PR #3](https://github.com/spencer-life/viperpilot/pull/3) (draft).

## Done when

- Define a repeatable method and sample count for startup, tray-only idle CPU/working set, editor-open/editor-closed costs, and UI response. Confirm the resident tray does not load optional editor rendering or poll while idle.
- With owner-supervised device validation, time actual tray and hotkey switches separately from device commit/readback time; record environment and starting state in private test records.
- Compare with the installed baseline and any claimed external target under comparable conditions before making a speed claim.
- Keep detailed device/environment identifiers and raw measurements private. Publish a redacted summary with methodology, results, variance, and limitations in public documentation. Keep the release gate open while its performance checks remain incomplete.

## Checkpoint — 2026-09-29

Production tray-only resource use and hardware switch latency remain unmeasured. Fresh isolated Linux checks do not establish Windows tray overhead or physical switching time. Keep this performance release gate open pending repeatable measurements and owner-supervised device validation.


## Software preparation — 2026-09-29

[Performance validation method](https://github.com/spencer-life/viperpilot/blob/codex/sanitized-quick-switch/docs/performance-validation.md) now defines sample counts, recorded conditions, cold/warm launch boundaries, 3×5-minute idle sampling, separate tray/editor costs, UI versus device/readback timings, comparison conditions and redacted reporting. Actual device timing remains behind the complete-profile/hotkey supervised gates; no timing quota overrides readback/restoration requirements.

This is preparation only. No production Windows resource sample, UI latency, mouse timing or comparison was measured. The existing editor-only measurement script remains separate from production tray cost. Keep this issue and its release gate open.
