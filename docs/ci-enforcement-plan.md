# Staged CI enforcement proposal

Prepared 2026-09-29 for the current public ViperPilot stack. This is a proposal,
not an applied setting or permission to merge. There are currently no repository
rulesets; that live read is only a checkpoint, not a permanent assumption.

## Why stages are necessary

The preserved stack is #1 → #14 → #15 → #16 → #3 → #18. PR #1 uses `ci (...)`
and has no Security workflow. PRs #14/#15 use historical `Core CI (...)` names
and Security. PR #16 and later use `ci (...)` and Security. Native GitHub stacks
apply trunk requirements to every layer; requiring all names simultaneously
would block them. This staged alternative preserves the existing commits without
force-pushes or cosmetic code restacking. Upper layers may be temporarily blocked
by a phase until their lower dependencies have landed.

| Phase | Only after | Required contexts | Eligible selected layers |
| --- | --- | --- | --- |
| A | Explicit authorization and fresh checks for #1 | `ci (ubuntu-24.04)`, `ci (windows-2025)` | #1 |
| B | #1 merged; surviving stack reconciled and fresh checks pass | `Core CI (ubuntu-24.04)`, `Core CI (windows-2025)`, `Security checks` | #14, then #15, or the reviewed group through #15 |
| C | Both #14/#15 merged; surviving stack reconciled and fresh checks pass | `ci (ubuntu-24.04)`, `ci (windows-2025)`, `Security checks` | #16, #3, #18 and compatible later work |

Phase templates are [A](ci-policy/phase-a-foundation.json),
[B](ci-policy/phase-b-switching-and-drafts.json) and
[C](ci-policy/phase-c-capability-and-later.json). They describe the same single
main-only ruleset; B and C update that rule's checks rather than layering a
second conflicting ruleset. All contexts are bound to observed GitHub Actions
app ID `15368`, no bypass actors, and strict-up-to-date enforcement is false.
Fresh integration validation is still required by the process. No merge queue
or deployment/release policy is introduced.

## Activation procedure and rollback

1. Obtain explicit authorization for the exact phase and its delivery-blocking
   effect. Prior broad development authorization and this document are not
   permission to activate rules. The earlier two-PR proposal is superseded.
2. Capture current main legacy protection and applicable repository/organization
   rulesets. Stop if additional rules or branch topology invalidate this plan;
   preserve unrelated rules. Confirm exact selected PR/head/main SHA, required
   context names, source app and successful fresh checks before applying.
3. Create A only if the captured state still permits this authorized addition.
   Save its returned rule ID and JSON. Apply B/C to that same rule ID only after
   the stated merged-dependency checkpoint and separately authorized settings
   change. Never weaken a requirement just to skip a failed/missing check.
4. Read back enforcement, main ref pattern, source app, bypass actors and exact
   contexts. A successful API call alone does not verify effective protection.
5. Roll back an update by restoring that rule's captured prior JSON. Roll back
   the newly added rule by deleting only its recorded ID, with authorization;
   do not remove other rules. Preserve evidence even if an operation fails.

No mutation/apply script is included. Each phase is a concrete reviewable JSON
proposal. Merge authorization, ready-for-review state, software review and all
manual Windows/mouse/tray release gates remain independent. No app installation,
startup edit or hardware operation is part of this plan.

## Measured context evidence

Read-only checks returned successful runs from app `15368` for:

- A: PR #1 head `a299997` ([Core](https://github.com/spencer-life/viperpilot/actions/runs/36495030723)).
- B: PR #15 head `6406e17` ([Core](https://github.com/spencer-life/viperpilot/actions/runs/36678102919), [Security](https://github.com/spencer-life/viperpilot/actions/runs/36678102963)); #14's new split checks also passed.
- C: PR #16 head `aa17779` ([Core](https://github.com/spencer-life/viperpilot/actions/runs/36678107083), [Security](https://github.com/spencer-life/viperpilot/actions/runs/36678107051)).

Those runs establish available names and sources at the recorded heads. They do
not validate a future rebased head or authorize a phase transition. Independent
PRs outside this stack need their own compatibility check before merging.

[GitHub native stack rules](https://docs.github.com/en/pull-requests/reference/stacked-pull-requests)
explain trunk-based requirements and selection of the target plus lower layers.
Select the intended lower layer when merging; an upper target can include all
unmerged layers below it. Draft layers cannot merge.
