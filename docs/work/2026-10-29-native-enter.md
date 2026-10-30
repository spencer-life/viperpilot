# Native Enter routing follow-up — 2026-10-29

## Reconciled source and measured failure

Draft PR #29 remains at 11bb982, based on #28. Its remote head was fetched
and reconciled before this fix. This focused fix/native-enter-focus layer
builds on the local test-only Rust 1.99 follow-up 5bc0f13; the original
dashboard checkout and prior Windows QA binaries/data remain preserved.

The separate Windows QA chat recorded automated Computer Use results on Windows
11 Pro build 26300. Compact/Device/Buttons visual containment passed in inspected
states. Tab/Shift+Tab, Space and Enter after tabbing passed the tested routes.
After Buttons -> Device, Quick switch visibly had focus but Enter did nothing.
Space returned to Compact, where restored Developer focus also ignored Enter
until Tab then Shift+Tab. Retain this failed checkpoint; visible focus alone
does not establish activation. Screen-reader speech, other actual display
scales, high contrast and physical keyboard observations remain unmeasured.

Editor checks passed in that separate session: scrolling and pinned Save/disabled
Apply at 1080 x 790 and 850 x 640 client sizes, and Save/reopen preserving the
same draft ID, unequal DPI, polling and both side-button intents. These are
automated fixture observations, not hardware or production evidence. Raw reports,
screenshots and development data remain private.

## Source correction and regression boundary

The custom Win32 top-level window used IsDialogMessageW but did not answer
DM_GETDEFID. The new handler returns the currently focused button's identifier
with DC_HASDEFID, using the existing known-child, HWND identity, visible and
enabled command guards. Null, foreign, unknown, hidden or disabled focus returns
no default. The window retains native Enter processing instead of adding a
keyboard hook, input injection or a remembered stale default. Actual commands
still pass normal runtime/deferred dispatch guards. The production launch gate
remains at the first statement and is not relaxed.

Microsoft documents [IsDialogMessageW's default-button queries](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-isdialogmessagew)
and [DM_GETDEFID's result encoding](https://learn.microsoft.com/en-us/windows/win32/dlgbox/dm-getdefid).
Replacing SetFocus with WM_NEXTDLGCTL alone was not assumed correct for a
custom top-level window lacking dialog processing.

The isolated native test client queries the already PID/class/child-checked
preview with a bounded DM_GETDEFID call. Transition assertions require Quick
switch on Device, Device on Buttons and the first profile on enabled Compact.
Unavailable Compact accepts no default or the enabled Details navigation button,
and rejects disabled profile defaults. Independent review corrected the initial
null-only assumption because SetFocus on disabled Developer can retain Details
focus. The existing three scenario checks still cover UIA semantics, repeated
routing, hidden-control refusals, synthetic profile state and clean process exit.

## Validation record

| Check | Result |
| --- | --- |
| Independent source review | No material Rust-handler finding. Corrected the unavailable test expectation described above. |
| mise run verify-linux | Pass: formatting, strict core/editor lint, core 129 library + 2 CLI + 3 capability + 6 catalog tests; feature suite 135 library + 2 CLI + 13 editor + 3 capability + 6 catalog tests, native release build and CLI-help smoke. |
| mise run verify-windows-cross | Pass: formatting, strict default/combined-preview target lint and production/native-preview/client/editor release builds. Build-only production/editor artifacts were not launched. |
| PowerShell parser and unavailable expectation | Pass. |
| Native harness default/reversed/unavailable | All three pass new default-button assertions, standard Button/Invoke/name/enabled semantics, repeated views, hidden-control refusals, synthetic state checks and exit code 0. |
| Separate Windows Computer Use retest | Pass original Gaming -> Details -> Customize -> Device -> Enter -> Compact -> Enter -> Developer sequence, with no Tab detour. Tested Tab/Shift+Tab and Space pass with visible focus. |
| Unavailable Computer Use retest | Both profiles disabled; enabled Details/Quick switch Enter navigation works without changing the synthetic Developer state. |
| mise run security | Pass at local source 71a8010: actionlint, redacted Gitleaks history scan (51 commits, no leaks) and zizmor (no findings, one existing suppression). |
| git diff --check | Pass. |

The separate Windows QA chat verified the built hashes before staging and
performing the keyboard retest. Native preview SHA-256:
1cad136fb3aba657514e323df70ae3b421d04cb4b9efe3728d50f19418db79f2.
Native test client SHA-256:
587d557f09c168b39e393b9bcc2c07fe02ff9c29b68cb60d72bf0bc904157db1.
Runtime source was tested as the pending fix diff; the resulting source
checkpoint retains that same runtime and client code.

Windows Computer Use supplied input; these are automated desktop observations,
not physical owner keyboard or screen-reader results. The interactive QA
windows closed before the three harness scenarios ran, preventing focus
interference. Both QA and harness sessions closed their owned previews; the
harness reported clean exits and removed only its temporary staging folders.
Raw evidence/logs remain private. The editor runtime is unchanged by this fix
and was not rerun interactively; its earlier same-day isolated results above
remain separately scoped. No production release or hardware gate is cleared.

Two preparation errors were corrected before runtime testing: passing multiple
mise task names as positional arguments appended the second name to rustc;
the corrected invocation runs tasks sequentially. A shell argument initially
expanded PowerShell variables inside the new harness text; the repaired text was
sent through stdin and passed the PowerShell parser. Neither failed preparation
launched a program or reached hardware.

No production tray, installer, startup operation, Synapse change, device command
or legacy-app operation is authorized by this follow-up. All release, complete
profile, hotkey and hardware evidence gates remain in force.

## Delivery state

The validated runtime fix is committed locally as 71a8010. The test repair
remains a separate lower checkpoint, 5bc0f13. A combined attempt to push these
two branches and create draft PRs was rejected before execution by automatic
approval review: explicit user permission to publish the source/documentation
to spencer-life/viperpilot is required. No branch was pushed and no PR was
created. Preserve the local worktrees, evidence and separate layer boundaries;
do not bypass this publication decision. Later local documentation updates
synchronize the roadmap and continuation status without changing runtime code.
