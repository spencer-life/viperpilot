---
title: Validate production tray and hotkey switching with the owner
labels:
    - codex-created
    - hardware-test
    - release-gate
projects:
    - ViperPilot dashboard
state: open
state_reason: ""
synced_at: 2026-09-30T07:11:16.411945959Z
info:
    author: spencer-life
    created_at: 2026-09-29T18:22:43Z
    updated_at: 2026-09-30T06:59:40Z
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

## Checkpoint — 2026-09-29

The production tray, both complete profiles, and the production hotkey have not passed the documented owner-supervised validation. The user is away from the Windows PC, so these manual checks are deferred. Keep this release gate open; no new hardware tests were run.

## Development isolation — 2026-09-29

Draft [PR #18](https://github.com/spencer-life/viperpilot/pull/18) separates default development data from the installed legacy app and blocks installer/production tray entry points before process, startup, hotkey or HID effects. Hardware-free tests and Windows cross-builds passed without launching. This does not satisfy the owner-supervised release gate. A separately reviewed executable/startup/hotkey identity and development installer are prerequisites for future installation or concurrent use. The currently installed app is untouched.

## Fresh final-state reporting — 2026-09-29

Draft [PR #20](https://github.com/spencer-life/viperpilot/pull/20), above isolation PR #18, removes stale final-state fallbacks after failed reads. Historical observations remain diagnostic evidence; unavailable or unclassifiable current state clears live values and disables profile controls/hotkeys until refresh. Core/preview CI and production Windows cross-build passed with synthetic mocks; no GUI/HID or physical disconnect/rollback test ran. This is a software safety improvement, not completion of the manual release gate.

## Software readiness — 2026-09-30

Draft PR #21 separates development window/mutex/show-message/startup identities in source and corrects the legacy-touching checklist. The production tray and installer remain blocked; source changes do not validate coexistence or authorize device tests. See [manual handoff](https://github.com/spencer-life/viperpilot/blob/codex/ui-readiness/docs/manual-handoff.md). Installed app unchanged; no GUI/HID or startup operation ran. Issue remains open for its manual and delivery gates.
