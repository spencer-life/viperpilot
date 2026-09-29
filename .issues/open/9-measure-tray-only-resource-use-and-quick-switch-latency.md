---
title: Measure tray-only resource use and quick-switch latency
labels:
    - codex-created
    - enhancement
    - release-gate
state: open
state_reason: null
synced_at: 2026-09-29T21:45:15.298815454Z
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
