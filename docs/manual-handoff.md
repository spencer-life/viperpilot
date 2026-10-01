# Software checkpoint and remaining manual gates

Updated 2026-09-30 after the owner requested all work that can proceed while they
are away. This checkpoint covers the current ViperPilot implementation scope;
roadmap expansion is not an implementation requirement or evidence of support.
All new implementation PRs remain draft. No merge, installation, startup edit,
production/preview GUI launch or hardware operation occurred.

## Completed without the owner

- Persisted local draft CRUD, validation, conflict protection and recovery;
  protected preset aliases and immutable baseline files remain separate.
- Exact-scope read-only capability assessments and offline catalog projection
  and export. Every custom combination remains unapproved; Apply stays disabled.
- Separate default development data and blocked installer/production tray entry
  points. Development Windows identities are distinct in source (PR #21);
  coexistence has not been measured.
- Fresh final-read reporting after no-op/write/rollback failures (PR #20).
  Generic worker errors also mark current state unavailable and disable profile
  controls/hotkeys. Successful refresh is required to restore connected controls.
- Editor duplicate-name and side-button action/key/description/modifier controls
  have associated accessible names. Headless role/name and editing tests pass.
- Reviewed correction of the manual checklist: no fixtures in legacy data,
  incidental exit of the installed app, or portable production launch assumed safe.

Independent review resolved the worker-error and editor-name findings. Final
handoff review also identified the already-recorded native owner-draw/UIA defect.
The native accessibility layer switches action/view/hotspot controls to standard
Windows push buttons. Its preview script now requires Button/Invoke semantics
and exercises UIA invocation; actual Windows execution remains deferred. Core checks pass **129 library + 2 CLI + 3 capability
+ 6 catalog tests**; preview checks pass **135 library + 2 CLI + 11 editor +
3 capability + 6 catalog tests**. Formatting, strict Clippy, native release and
Windows MSVC production/editor cross-builds pass without launching. A first new
duplicate-input test lacked focus; clicking/focusing the named input corrected
the test, and the full preview suite passed. This is synthetic software evidence,
not a Windows screen-reader or monitor result.

Hosted Linux/Windows/Security checks passed for PR #20 at `586f7a0`
([Core](https://github.com/spencer-life/viperpilot/actions/runs/36681122207),
[Security](https://github.com/spencer-life/viperpilot/actions/runs/36681122294)).
Newer layer checks belong to their own heads; do not reuse this hosted result as
proof of a later commit. New PRs remain draft even when checks pass.

## Remaining gates

| Issue / task | Boundary | Next observed step |
| --- | --- | --- |
| #6 native UI | Source remediation prepared; interactive Windows verification | Standard button semantics replace the known owner-draw pane/no-Invoke path. Run the isolated preview script and verify: labels, keyboard, roles/patterns, screen reader, scale, narrow windows and error states |
| #8 editor UI | Interactive Windows | Isolated editor: actual monitor vs capture, keyboard/IME/paste, non-Latin names, screen reader, scaling, Save/reopen and disabled Apply |
| #7 production tray | Owner-present Windows + mouse | First review the separate launch arrangement/hotkey ownership. Keep production blocked until safe prerequisites are agreed; then supervised complete-profile/readback/rollback/hotkey/persistence checks |
| #9 performance | Owner-present Windows | Record tray-only resources and production switching latency separately from editor metrics; blocked tray cannot provide valid measurements |
| #10 changed DPI | Supervised mouse experiment | One reviewed logical DPI field, independent readback and baseline restoration; no general range inferred |
| #11 side-button actions | Supervised mouse experiment | One reviewed button/action, independent readback and baseline restoration; no interception/injection |
| #4, #12, #13 delivery | Review/delivery and relevant evidence | Software portions implemented; keep issues open until acceptance and authorized delivery. Custom promotion waits for complete device-scoped evidence |
| #5 CI enforcement | Explicit settings authorization | Review draft PR #19's concrete staged A/B/C policy; capture current protection, reconcile selected heads and fresh checks before each authorized activation |
| Codex Cloud automation | Cloud environment access/credentials | Local CLI Project access works; use [Cloud access handoff](cloud-access-check.md) in the actual task. Environment/task URL still needed. No cloud environment or secret was changed |

## Resume sequence

1. Reconcile latest GitHub heads and checks, choose the owning layer, and read
   [development isolation](development-isolation.md).
2. With the owner at Windows, do UI-only preview/editor checks in temporary roots
   first. Do not point at legacy files or enable production startup/install.
3. Review safe production launch/hotkey coordination before considering the
   blocked tray. Separate folders/Windows names do not isolate a physical mouse.
4. Only under normal immutable-baseline, full identity and exact-plan gates,
   perform the [supervised checklist](quick-switch-supervised-checklist.md).
   Keep new DPI and side-button experiments separate from complete-profile tests.
5. Preserve raw evidence privately and publish redacted measured results. Close
   issues or release only after their actual gates and delivery authorization pass.

The installed legacy app stays untouched. No unobserved checkbox is a passing
result, and no automatic approval or merge is implied by this handoff.

## Delivery review checkpoint — 2026-09-30

At integration head `b176656`, live GitHub checks were green for every
implementation PR in stack #17: #1, #14, #15, #16, #3, #18, #20, #21, #22,
#23 and #24. Each has its own successful Linux/Windows results; all layers from
#14 upward also passed Security. This observation does not carry forward after
a new push or base change. Draft policy PR #19 and Renovate #2 do not have those
implementation checks and require their own delivery review.

Read-only CI proposal review confirmed A/B/C templates target only main, have
no bypass actors, and preserve the historical required-context differences.
Live main still had no rulesets or legacy protection. Settings activation and
merges remain separately authorized steps; no rules were applied. The final
handoff audit corrected the checklist to separate isolated preview/editor work
from future production tray tests and label tray-route regression as planned.

## Native button remediation — 2026-09-30

The historical pane/no-Invoke result remains recorded in quick-switch status.
Source remediation uses Windows-standard button styles without adding a resident
UI framework or custom accessibility provider. Windows system COM annotations
provide semantic names for numbered hotspots while retaining their compact
visual labels; initialization, partial failures and teardown have scoped cleanup. The native appearance replaces
custom-colored button painting; current profile and selected assignment remain
visible in status text. Compact profile names wrap through the multiline style.
Real roles, Invoke, keyboard, focus, contrast, scaling and legibility must still
pass on Windows. The preview script fails on pane/no-Invoke instead of merely
logging it; it remains an owner-present test, not an unattended launch.

This is a prepared software fix, not a claimed passing Windows accessibility
result. No production gate or hardware policy changed.

The annotation follows Microsoft’s [accessible-name sample](https://learn.microsoft.com/en-us/accessibility-tools-docs/items/win32/edit_name). UIA name propagation and teardown must be measured on Windows; cross-compilation alone does not establish either.

Final native-layer validation passed formatting/diff checks, production and native
preview Windows MSVC cross-builds, and Windows-target all-target Clippy using the
repository's Windows policy (`-D warnings -A clippy::pedantic`). Initial annotation
compilation exposed missing Windows features; enabling only system COM and
Accessibility resolved it. Independent review found no remaining material software
blocker in the scoped delta. The headless core/editor results above are from
`7f4dfac`; the final layer changes Windows-only controls and their manual script.
PowerShell/UIA runtime validation is deferred; no script or GUI was launched.
