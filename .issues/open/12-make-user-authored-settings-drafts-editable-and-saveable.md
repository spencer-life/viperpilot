---
title: Make user-authored settings drafts editable and saveable
labels:
    - codex-created
    - enhancement
projects:
    - ViperPilot dashboard
state: open
state_reason: null
synced_at: 2026-09-30T09:22:48.030307461Z
info:
    author: spencer-life
    created_at: 2026-09-29T18:22:54Z
    updated_at: 2026-09-30T00:06:57Z
---

## Context

The optional local draft editor now provides a persistent editing workflow on the existing versioned intent model. It saves clearly marked unverified drafts while keeping Apply unavailable until device-specific safety gates pass. Production tray launch integration and owner-observed Windows presentation remain separate gates.

Implementation baseline: [public PR #3](https://github.com/spencer-life/viperpilot/pull/3) (draft).

## Done when

- Reconcile each enabled editor control with a versioned persisted setting. Values must round-trip, or be clearly marked illustrative and unsavable; edits must not be silently dropped.
- Provide a user-facing way to create, edit, rename, duplicate, delete, and save local drafts across restart.
- Preserve built-in recovery profiles and preserve corrupt or newer-format draft files without resetting them.
- Show which fields are draft-only and what evidence is missing. Saving must not invoke the planner, worker, or HID path. Keep Apply disabled until the relevant device-specific write gates pass.
- Cover CRUD, round-trip, invalid/corrupt/future data, and separation from configuration, aliases, diagnostic reports, and immutable baseline evidence in hardware-free tests. Confirm editor code is not loaded in tray-only idle mode.

Any full device evidence stays in private local records; only a redacted summary may be published.

## Checkpoint — 2026-09-29

Software implemented in the same draft PR #3: local create, edit/rename, duplicate, delete, Save and reopen using stable identities. Enabled fields round-trip name, X/Y DPI, polling and semantic Mouse4/Mouse5 actions; unsupported settings are unsavable. Saves use validated copies, atomic replacement, a short-lived editor lock and external-change checks. Failed saves preserve the buffer and prior data. Corrupt, invalid and future-schema files block editing without reset. Default production builds exclude the editor and eframe; draft operations have no planner, worker or HID path.

Validation and review results are recorded in [the local editor document](https://github.com/spencer-life/viperpilot/blob/codex/sanitized-quick-switch/docs/local-draft-editor.md) and [the continuation brief](https://github.com/spencer-life/viperpilot/blob/codex/sanitized-quick-switch/docs/continue-here.md). This issue remains open while the implementation PR is draft. The next software slice is capability contract #4, followed by hardware-gated promotion #13. No manual Windows UI, production tray/hotkey, installation/startup or physical-device tests were run; the owner remains away from the PC.

Software checkpoint `6792c6f`: `mise run ci` passed 108 core tests and lint/build/smoke; `mise run ci-preview` passed strict Clippy, 114 library tests and seven editor tests. Production and editor Windows cross-builds passed without launching. Independent review found no remaining material findings. These are local software results; hosted CI for the new head must be checked separately.

## Focused PR ownership — 2026-09-29

Editable persisted drafts are now isolated in [PR #15](https://github.com/spencer-life/viperpilot/pull/15), based on guarded switching [PR #14](https://github.com/spencer-life/viperpilot/pull/14). Existing commits and runtime behavior were preserved when the owner requested smaller PRs. PR #3 and the new PRs remain draft; issues stay open until their delivery and validation criteria pass. No merge, release or mouse operation occurred. Earlier references to the combined PR #3 describe historical checkpoints.

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
