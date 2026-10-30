# Repeatable native Enter regression — 2026-10-29

## Scope and local ownership

The owner requested local development and CI. No branches were pushed, no
PRs were created and no GitHub Actions were triggered. The separate Windows
Computer Use chat remains responsible for new visual/interactive requests;
this source chat performs software checks through WSL. The existing successful
Computer Use evidence is preserved and was not rerun for this test-only change.

This local test/native-enter-regression layer builds on 512bab4, which preserves
runtime fix 71a8010 and the earlier test-only Rust 1.99 repair 5bc0f13. The
production runtime, optional editor, protocol, storage and hardware gates
are unchanged. The original dashboard checkout remains preserved.

## Regression coverage

The native helper has a fixed enter action for the development preview only.
It keeps the existing PID, window-class, known-child, parent, visible, enabled,
Button-role and Invoke guards, rechecks child identity, and queries the specified
preview GUI thread's actual focus. An unfocused child is rejected. Only then
does it queue one Return key-down message to that checked child, allowing the
preview's normal GetMessage/IsDialogMessage loop to perform activation.

This is queued-message testing rather than physical input or a replacement
for the separate Computer Use observations. It adds no global SendInput, hooks,
thread attachment, arbitrary keys or runtime input interception. The helper is
feature-gated and excluded from default production builds. See Microsoft's
[GetGUIThreadInfo](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getguithreadinfo)
and [PostMessageW](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-postmessagew)
contracts for the target-thread focus query and queued dispatch.

Each enabled scenario begins at the profile opposite the first Compact slot.
The harness enters Buttons, returns to Device, queues Enter on restored Quick
switch, then queues Enter on the restored first profile without a Tab detour.
It independently waits for view and profile changes, so a no-op cannot pass.
The reversed scenario verifies slot interpretation; unavailable profiles remain
disabled while enabled navigation works. Enter targeting unfocused Customize
or a disabled profile is explicitly refused. Existing hidden-control guards,
UIA semantics, repeated routing, synthetic actions and clean-exit checks remain.

## Validation

- Local verify-linux passed: formatting, strict core/editor Clippy, core
  129 library + 2 CLI + 3 capability + 6 catalog tests; optional feature
  135 library + 2 CLI + 13 editor + 3 capability + 6 catalog tests, native
  release build and CLI-help smoke.
- Strict Windows default/combined-preview cross-lint and the release native
  test-client build passed using the repository's pinned tooling.
- PowerShell parsing and all three actual Windows synthetic harness scenarios
  passed, including the new queued Enter/refusal checks. Each owned preview
  exited with code 0 and only its temporary staging folder was removed.
- Independent read-only review found no material findings in the helper/harness
  diff. Runtime source equality was checked against the parent.
- git diff --check passed. The prior unchanged workflow/security configuration
  passed the actionlint, redacted history scan and zizmor tasks at 71a8010.

Tested preview SHA-256:
1cad136fb3aba657514e323df70ae3b421d04cb4b9efe3728d50f19418db79f2.
Tested new client SHA-256:
75237c8b67822994fd55040c5a5745a3b831b724d462d046c76b8511bc263c71.

Raw logs and artifacts remain local. The production tray, installer, startup,
legacy app and physical mouse were untouched. Screen-reader speech, actual
display scales/high contrast, production performance and physical-device
behavior retain their existing gates.
