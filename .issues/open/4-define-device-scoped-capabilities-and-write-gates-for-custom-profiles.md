---
title: Define device-scoped capabilities and write gates for custom profiles
labels:
    - codex-created
    - enhancement
projects:
    - ViperPilot dashboard
state: open
state_reason: null
synced_at: 2026-09-30T06:28:41.357994846Z
info:
    author: spencer-life
    created_at: 2026-09-29T18:22:37Z
    updated_at: 2026-09-30T02:34:47Z
---

## Context

User-authored settings need a versioned configuration and write contract before they can be applied. Field assessments may describe evidence, but they do not by themselves create a write plan or authorize hardware changes. Support remains limited to identities and field actions with measured evidence.

Implementation baseline: [public PR #3](https://github.com/spencer-life/viperpilot/pull/3) (draft).

## Done when

- Define a device-, firmware-, and transport-scoped capability contract and a versioned semantic request format. Specify accepted values, field combinations, independent readback, rollback, and recovery conditions without widening the current production allowlist.
- Reject unknown identity, stale or ambiguous state, unsupported values/actions/combinations, corrupt or future schema, and missing baseline before a setter or write plan can be dispatched.
- Keep local draft storage distinct from onboard slots and immutable baseline evidence.
- Hardware-free tests demonstrate fail-closed planning and preserve existing built-in profile gates. Record migration and rollback behavior.
- Keep physical validation separate. This issue alone cannot enable Apply.

Full device identity and experiment evidence stay in private local records; only redacted summaries belong in public issues or documentation.

## Checkpoint — 2026-09-29

Implemented in draft PR #3 at source `28b36f2` (contract `d28661c`): typed exact-scope registry, strict versioned semantic request, and read-only preflight. Missing/incomplete baseline, full identity/scope mismatch, stale/ambiguous state, unsupported values/actions/transitions, and malformed/future/unknown schema fail explicitly. Even individually evidenced values stop at the unapproved custom combination gate. No WritePlan, setter, allowlist expansion, onboard slot or Apply integration is added.

The optional editor shows offline evidence/blockers without implying a verified connection. Valid draft Save remains independent. Draft/library V1 serialization and storage remain compatible; unknown extra fields in all button-action variants now fail without reset. Contract details and rollback behavior: [docs/capability-contract.md](https://github.com/spencer-life/viperpilot/blob/codex/sanitized-quick-switch/docs/capability-contract.md).

Hardware-free validation passed: core CI (114 library + 3 integration tests, strict Clippy, formatting, native release and CLI help); preview CI (120 library + 9 editor + 3 integration tests); sequential production/editor Windows MSVC cross-builds. Independent final review found no remaining material issues after fixing unit-action unknown-field parsing. Tests cover all eight scope axes and metadata/order changes without inherited support or false stale-state rejection. Default dependencies exclude egui/eframe.

No manual Windows, tray/hotkey, startup, performance or mouse test ran; owner remains away from the PC. Caller-supplied snapshots are not proof of a fresh GET, immutable baseline or reviewed serials. Keep this issue open for review/delivery, PR #3 draft, and promotion #13 blocked until complete field/combination/readback/restoration evidence permits it. Hosted checks for the new published head require their own status review.

## Focused PR ownership — 2026-09-29

Capability software is now isolated in [PR #16](https://github.com/spencer-life/viperpilot/pull/16), based on editable drafts [PR #15](https://github.com/spencer-life/viperpilot/pull/15). Existing commits and runtime behavior were preserved when the owner requested smaller PRs. PR #3 and the new PRs remain draft; issues stay open until their delivery and validation criteria pass. No merge, release or mouse operation occurred. Earlier references to the combined PR #3 describe historical checkpoints.
