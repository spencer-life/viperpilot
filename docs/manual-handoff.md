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

Independent UI review found two concrete issues and reviewed their fixes with no
remaining material findings. Core checks pass **129 library + 2 CLI + 3 capability
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
| #6 native UI | Interactive Windows | Isolated native preview: labels, keyboard, roles/patterns, screen reader, scale, narrow windows and error states |
| #8 editor UI | Interactive Windows | Isolated editor: actual monitor vs capture, keyboard/IME/paste, non-Latin names, screen reader, scaling, Save/reopen and disabled Apply |
| #7 production tray | Owner-present Windows + mouse | First review the separate launch arrangement/hotkey ownership. Keep production blocked until safe prerequisites are agreed; then supervised complete-profile/readback/rollback/hotkey/persistence checks |
| #9 performance | Owner-present Windows | Record tray-only resources and production switching latency separately from editor metrics; blocked tray cannot provide valid measurements |
| #10 changed DPI | Supervised mouse experiment | One reviewed logical DPI field, independent readback and baseline restoration; no general range inferred |
| #11 side-button actions | Supervised mouse experiment | One reviewed button/action, independent readback and baseline restoration; no interception/injection |
| #4, #12, #13 delivery | Review/delivery and relevant evidence | Software portions implemented; keep issues open until acceptance and authorized delivery. Custom promotion waits for complete device-scoped evidence |
| #5 CI enforcement | Explicit settings authorization | Review draft PR #19's concrete staged A/B/C policy; capture current protection, reconcile selected heads and fresh checks before each authorized activation |
| Codex Cloud automation | Cloud environment access/credentials | Local CLI Project access works; verify actual cloud task credentials/project access separately. No cloud environment or secret was changed |

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
