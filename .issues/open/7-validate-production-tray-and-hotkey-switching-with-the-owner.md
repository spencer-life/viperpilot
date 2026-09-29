---
title: Validate production tray and hotkey switching with the owner
labels:
    - codex-created
    - hardware-test
    - release-gate
state: open
state_reason: null
synced_at: 2026-09-29T18:28:49.929850376Z
---

## Context

The production tray, compact switcher, saved-pair selection, and quick-switch hotkey require supervised validation on the supported Windows setup. Builds and synthetic previews do not validate a physical profile switch. The owner must be present for device testing.

Implementation baseline: [public PR #3](https://github.com/spencer-life/viperpilot/pull/3) (draft).

## Done when

- Record the tested build commit/hash, device identity, firmware/transport, companion-software state, complete immutable baseline, and exact reviewed write plan in private local records before any setter. Stop on missing or ambiguous evidence.
- Exercise each built-in complete profile through a top-level tray action, then activate the documented quick-switch hotkey in both directions. Independently reread the relevant state after each action and follow the documented restoration path.
- For any separate new-field experiment, test one logical field at a time and restore the reviewed baseline before another experiment.
- Record hotkey repeat behavior, errors/rollback, process-exit and mouse power-cycle readbacks, timing, and owner observations privately. Restore prior app/config/startup state.
- Publish only a redacted summary of measured conditions and results. Keep the release gate open if any required profile, hotkey, persistence, or rollback gate remains unproven. Do not run this as unattended CI.
