---
title: Promote verified custom profiles into quick switching
labels:
    - codex-created
    - enhancement
state: open
state_reason: null
synced_at: 2026-09-29T18:28:46.34145338Z
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
