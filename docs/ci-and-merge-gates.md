# CI and stacked merge gates

Updated 2026-09-29. This records software checks and the prepared enforcement
policy; it does not permit merging, releasing or hardware testing.

| Workflow | Events | Hosted platforms | Local entrypoints |
| --- | --- | --- | --- |
| Core CI | PR to any base, push to main, manual dispatch | Ubuntu 24.04, Windows 2025 | `mise run ci`, `ci-preview`; on Windows `ci-windows`, `ci-windows-preview` |
| Security Checks | PR to any base, push to main, manual dispatch | Ubuntu 24.04 | `mise run security` |

PR events deliberately cover the stacked development base as well as main.
Checkout uses the PR merge candidate on PR events and the event commit on
push/dispatch. Thus the checks test integration with the selected base; they
are not mouse tests. The checkout credentials are not persisted, token contents
permission is read-only, and no workflow installs or launches the production
tray, changes startup or dispatches a device setter. Preview tests are headless.

Core job names are aligned with the foundation PR: `ci (ubuntu-24.04)` and
`ci (windows-2025)`. Security reports `Security checks`. Renaming the workflow
does not rename the required job contexts. Keep these names stable or update
requirements deliberately after observing the replacement jobs pass.

## Enforcement and merge order

On 2026-09-29, main had no legacy protection and no rulesets. The proposed
main-only ruleset requires the two Core contexts, each bound to the measured
GitHub Actions app ID `15368`, with no bypass actors. It is prepared only:
automatic approval review rejected enabling it without explicit authorization
for a persistent settings change that can block main updates. No ruleset was
created. The previous empty ruleset state is retained privately for rollback.

1. Keep PR #3 draft. PR #1 targets main; PR #3 targets PR #1's branch.
2. Enable only the two shared Core contexts first, after authorization. PR #1
   already reports those checks successfully; requiring Security now would
   block it because its historical workflow does not emit that job.
3. Merge PR #1 only when separately authorized. Then retarget PR #3 to main,
   reconcile its source and rerun checks against that base.
4. Add `Security checks` to main's requirements after PR #1 lands, and observe
   the final PR #3 head's Core and Security checks pass before any authorized
   merge. Do not bypass missing checks or the manual release gates.

The prepared policy uses `strict_required_status_checks_policy: false` to avoid
requiring the unmerged stack base on main. This means Core checks need not be
from the latest main base; explicit retarget/reconciliation and fresh checks
remain required by this workflow. There is no merge queue. Do not enable one
without adding and validating `merge_group` handling first.

Before a settings mutation, capture the relevant current JSON, compare the
exact rules, and read back enforcement, ref pattern, sources and bypass actors.
If the proposed new ruleset is authorized, its rollback is deletion of that
specific new ruleset only; preserve unrelated rules. Adding Security later needs
its own before/after capture. No durable app authorization policy was changed.
GitHub's [repository rules API](https://docs.github.com/en/rest/repos/rules)
defines the required-check context and integration source fields.

## Last observed hosted checkpoint

At `1db3cc7cce2a67faa8469980a730866a200e785d`, both historical `Core CI (...)`
jobs passed in [run 36660537944](https://github.com/spencer-life/viperpilot/actions/runs/36660537944),
and Security passed in [run 36660538070](https://github.com/spencer-life/viperpilot/actions/runs/36660538070).
This predates catalog code and the job-name alignment. Later heads require
separate observations. PR #1 head `a299997` reports both shared `ci (...)`
contexts successfully. No merge was performed.
