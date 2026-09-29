---
title: Define device-scoped capabilities and write gates for custom profiles
labels:
    - codex-created
    - enhancement
state: open
state_reason: null
synced_at: 2026-09-29T18:28:47.274435634Z
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
