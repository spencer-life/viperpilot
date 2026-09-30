---
title: Promote verified custom profiles into quick switching
labels:
    - codex-created
    - enhancement
state: open
state_reason: ""
synced_at: 2026-09-30T03:20:14.758965643Z
info:
    author: spencer-life
    created_at: 2026-09-29T18:22:56Z
    updated_at: 2026-09-29T21:45:05Z
---

## Context

Saved entries currently select or rename built-in presets; local user-authored drafts are not yet equivalent to supported hardware profiles or onboard slots. The product goal is user-owned profiles for fields that have earned device-specific support.

Implementation baseline: [public PR #3](https://github.com/spencer-life/viperpilot/pull/3) (draft).

## Done when

- Define a reversible path from preset aliases and local drafts to first-class user-owned profiles. Preserve built-in recovery profiles, stable IDs, the selected quick-switch pair, existing files, and a rollback path.
- Only a complete profile whose requested fields and combinations pass the device-scoped capability contract and relevant supervised evidence can enter the production Apply/quick-switch path. Unsupported fields remain clearly draft-only; existing aliases continue to work.
- Hardware-free migration, planning, and failure tests pass, followed by owner-supervised complete-profile switching, independent readback, rollback, process-exit, and power-cycle checks before release.
- Coordinate with “Make user-authored settings drafts editable and saveable,” “Validate changed DPI targets on the supported wireless mouse,” and “Validate additional side-button actions on the supported mouse” for each exposed field.

Full device evidence remains in private local records; publish only redacted validation summaries.

## Checkpoint — 2026-09-29

Promotion follows draft editor #12 and capability contract #4. Only complete profiles that pass device-specific capability checks and supervised readback/rollback validation may enter production Apply or quick switching. Unverified drafts are not onboard slots; keep this issue open.


## Offline implementation — 2026-09-29

Implemented the reversible software portion in draft PR #3: a separate strict V1 catalog projects existing preset aliases and frozen semantic draft copies, preserving stable alias IDs, protected built-ins, the exact selected pair and every original production file. Draft entries use `draft:<source-id>` and cannot enter the quick-switch pair. Actual promotion/worker dispatch is unchanged and no custom combination is approved.

`viperctl catalog-preview --root DIRECTORY` previews without writing; `catalog-export` publishes only a separate `profile-catalog-v1.json`, atomically without replacing an existing target. Sources, config and immutable baselines are preserved. Export is opt-in, requires an existing directory and hard-link support, and uses optimistic source-byte checks without claiming a writer lock. Rollback is to ignore/remove derived data and return to the old executable; no V1 store is replaced. See [catalog details](https://github.com/spencer-life/viperpilot/blob/codex/sanitized-quick-switch/docs/profile-catalog.md).

Software source `26da843`, with Windows-only import correction `b034547`: core CI passed 122 library + 2 CLI + 3 capability + 6 catalog integration tests, strict Clippy/formatting/native build/help; feature CI passed 128 library + 2 CLI + 9 editor + 3 capability + 6 catalog integration tests. Sequential production/editor Windows MSVC release cross-builds passed after that correction. Independent implementation review found no material findings. Integration cases cover actual CLI subprocesses, protected built-ins, draft-pair rejection, nested unknown fields, unchanged source/config/baseline bytes, existing-output preservation and concurrent exactly-one-winner export. Default dependencies exclude egui/eframe.

All custom preflights still reject; no WritePlan/setter, device encoder, Apply or custom quick-switch integration was added. Supplied snapshots do not prove fresh GET, immutable baseline or full identity review. No manual Windows, startup or mouse operation ran. Keep PR #3 draft and this issue open: actual promotion remains blocked pending complete-profile, readback, recovery, process-exit and power-cycle evidence.
