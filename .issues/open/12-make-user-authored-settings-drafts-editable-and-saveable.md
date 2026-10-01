---
title: Make user-authored settings drafts editable and saveable
labels:
    - codex-created
    - enhancement
state: open
state_reason: null
synced_at: 2026-09-29T21:45:10.524461402Z
---

## Context

The application has a local draft model for profile settings, but the user-facing editor is still a preview and does not yet provide a complete persistent editing workflow. The product direction allows users to save clearly marked drafts for unverified settings while keeping Apply unavailable until device-specific safety gates pass.

Implementation baseline: [public PR #3](https://github.com/spencer-life/viperpilot/pull/3) (draft).

## Done when

- Reconcile each enabled editor control with a versioned persisted setting. Values must round-trip, or be clearly marked illustrative and unsavable; edits must not be silently dropped.
- Provide a user-facing way to create, edit, rename, duplicate, delete, and save local drafts across restart.
- Preserve built-in recovery profiles and preserve corrupt or newer-format draft files without resetting them.
- Show which fields are draft-only and what evidence is missing. Saving must not invoke the planner, worker, or HID path. Keep Apply disabled until the relevant device-specific write gates pass.
- Cover CRUD, round-trip, invalid/corrupt/future data, and separation from configuration, aliases, diagnostic reports, and immutable baseline evidence in hardware-free tests. Confirm editor code is not loaded in tray-only idle mode.

Any full device evidence stays in private local records; only a redacted summary may be published.

## Checkpoint — 2026-09-29

Next implementation slice: real local create, edit, duplicate, rename, delete, and reopen of persisted drafts while offline. Save is currently disabled/disconnected; Apply must remain disabled until device-specific gates pass. The sequence continues through capability contract #4 and profile promotion #13. No interactive Windows UI or device tests were run; headless preview checks remain software-only.
