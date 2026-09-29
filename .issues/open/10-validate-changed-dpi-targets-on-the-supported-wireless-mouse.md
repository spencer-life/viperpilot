---
title: Validate changed DPI targets on the supported wireless mouse
labels:
    - codex-created
    - hardware-test
state: open
state_reason: null
synced_at: 2026-09-29T18:28:43.729681449Z
---

## Context

Changed DPI values and a general supported range are not yet established. Existing evidence is limited to the currently documented unchanged-value behavior on the supported wireless model. This issue is supervised device research, not an authorization to expose a DPI range.

Implementation baseline: [public PR #3](https://github.com/spencer-life/viperpilot/pull/3) (draft).

## Done when

- Select and record one candidate changed DPI value and the exact device, firmware, and transport identity in private test records. Review the full identity, immutable baseline, and a single-field plan before the experiment.
- Change only DPI, independently read the result back, and restore and verify the recorded baseline before another field or value.
- Preserve raw evidence, failures, ambiguous responses, and ruled-out explanations in private local records. Publish only a redacted summary without serials, report filenames, or local paths.
- Update public support claims only for values actually measured. If the experiment is unsafe or inconclusive, keep that value draft-only and explain the limitation in the redacted summary. Do not use unattended CI or guess a range.

This issue does not authorize a write unless the project's hardware safety gates are satisfied.
