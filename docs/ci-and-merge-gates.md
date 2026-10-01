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

Core job names in PR #1 and from PR #16 upward are `ci (ubuntu-24.04)` and
`ci (windows-2025)`. The preserved historical heads of split PRs #14 and #15
still emit `Core CI (ubuntu-24.04)` and `Core CI (windows-2025)`. Security
reports `Security checks` from #14 upward; #1 does not emit it. Renaming the
workflow does not rename required job contexts. The split did not backport or
rewrite the one-line alignment commit `aa17779`.

## Enforcement and merge order

On 2026-09-29, main had no legacy protection and no rulesets. The proposed
main-only ruleset requires the two Core contexts, each bound to the measured
GitHub Actions app ID `15368`, with no bypass actors. It is prepared only:
automatic approval review rejected enabling it without explicit authorization
for a persistent settings change that can block main updates. No ruleset was
created. The previous empty ruleset state is retained privately for rollback.

1. The official stack is #1 → #14 guarded switching → #15 editable drafts →
   #16 capability checks → #3 catalog. Each PR targets the branch immediately
   below it; #1 targets main. Keep #3 and all three added layers draft while
   their documented gates are open. Renovate #2 remains independent.
2. The earlier two-PR ruleset proposal is not ready for activation across this
   new stack: #14/#15 do not report its required `ci (...)` names. Resolve
   issue #5 by normalizing and validating those contexts on every layer that
   will be required to satisfy them, or by preparing an explicitly reviewed
   alternative staged policy. Do not enable known-missing required contexts,
   bypass checks, or treat prior approval preparation as authorization.
3. Native GitHub stacks evaluate requirements against the trunk for every
   layer. Review each layer's current integration checks and open gates. Any
   merge still requires separate authorization; targeting an upper PR can
   merge all unmerged PRs below it as one operation. Do not invoke a top-stack
   merge merely to land one feature.
4. After an authorized lower-layer merge, reconcile the surviving stack and
   verify its new ancestry, bases, current heads and checks. Do not retarget
   #3 directly to main while #14/#15/#16 remain unmerged; that would collapse
   the focused review boundaries again. GitHub's native stack rebasing changes
   surviving heads, so historical green checks cannot be carried forward.
5. Add Security as a required context only after every remaining affected
   layer emits and passes it, with explicit settings authorization and a
   captured rollback baseline. #1's historical head does not emit it.
   Hardware and manual release gates remain independent of hosted checks.

The prepared policy uses `strict_required_status_checks_policy: false` to avoid
requiring every stack layer to be up to date with main. This means Core checks need not be
from the latest main base; explicit stack reconciliation and fresh checks
remain required by this workflow. There is no merge queue. Do not enable one
without adding and validating `merge_group` handling first.

Before a settings mutation, capture the relevant current JSON, compare the
exact rules, and read back enforcement, ref pattern, sources and bypass actors.
If the proposed new ruleset is authorized, its rollback is deletion of that
specific new ruleset only; preserve unrelated rules. Adding Security later needs
its own before/after capture. No durable app authorization policy was changed.
GitHub's [repository rules API](https://docs.github.com/en/rest/repos/rules)
defines the required-check context and integration source fields.

## Observed hosted checkpoints

At `1db3cc7cce2a67faa8469980a730866a200e785d`, both historical `Core CI (...)`
jobs passed in [run 36660537944](https://github.com/spencer-life/viperpilot/actions/runs/36660537944),
and Security passed in [run 36660538070](https://github.com/spencer-life/viperpilot/actions/runs/36660538070).
This predates catalog code and the job-name alignment. Later heads require
separate observations. PR #1 head `a299997` reports both shared `ci (...)`
contexts successfully. No merge was performed.

Hosted Core Linux/Windows and Security also passed at runtime checkpoint
`4ed29f1` ([Core run](https://github.com/spencer-life/viperpilot/actions/runs/36664015564),
[Security run](https://github.com/spencer-life/viperpilot/actions/runs/36664015563)).
That observation predates the focused PR registration and continuation-only
commit. Newly created PRs and changed bases have their own checks.

GitHub's [native stack requirements](https://docs.github.com/en/pull-requests/reference/stacked-pull-requests)
explain trunk-based enforcement and cascading merge/rebase behavior. No repository
rules, merge queue or App permissions changed during this split.
