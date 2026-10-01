# Quick-switch implementation and validation status

Updated 2026-09-29. The historical foundation checkpoint below is public commit `7cb0440a8b21f54154c1042757c810f3b9552c24` in [draft PR #3](https://github.com/spencer-life/viperpilot/pull/3), branch `codex/sanitized-quick-switch`. Its hosted software checks passed; it is not release approval or physical-device validation. The issue #12 local editor now has a later software checkpoint; neither checkpoint is manual validation. Start a new session with [the continuation brief](continue-here.md) and track open work in the [public issue tracker](https://github.com/spencer-life/viperpilot/issues). GitHub is the implementation source; the old ZIP patch must not be reapplied.

## Goal and scope

The priority is fast access from the native Windows tray or hotkey, with no browser or settings navigation for an established profile pair and little background work. Users should eventually be able to create their own settings for DPI, buttons, polling, and other fields that are proven safe. The Developer and Gaming profiles are protected example/recovery presets; they do not define the full customization model. Full Synapse parity and support for every recent mouse are not requirements.

The application currently supports only its documented Viper V4 Pro device and field boundaries. Local names and drafts are not additional onboard slots, baselines, device authorization, or proof of current hardware state. Unsupported devices, fields, transitions, and ambiguous states must continue to fail closed.

## Implemented software

- A validated local profile-alias library supports protected built-ins, user-named aliases, and a two-entry quick-switch selection.
- Tray actions and the global hotkey reload the selection, verify current device state, and use the existing guarded profile-apply path. Pair selection and library reload do not write to the device.
- The native compact switcher opens first, with a Details view for the existing read-only inspector and recovery actions. The UI preview is isolated from the production tray and hardware worker.
- User-authored profile intent has versioned local draft storage separate from complete-preset aliases and device baselines. The optional egui editor now supports local create/edit/rename/duplicate/delete/save/reopen. Apply stays disabled. Default tray builds exclude both the editor session and eframe. Windows interaction is still unverified.
- Hardware-free tasks cover formatting, lint, tests, native build, and CLI smoke. Separate preview tasks exercise synthetic UI states and Windows accessibility metadata.

The guarded write engine also checks each planned field against the last trusted snapshot before writing, advances that snapshot only after a complete validated readback, verifies device identity on readback, and stops when identity or field state is ambiguous. A rejected write is classified using readback; an ambiguous acknowledgement cannot be treated as proof that an equal-value DPI write succeeded. These checks reduce stale-plan and rollback risk. They do not establish additional device support or authorize new writes.

## Validation evidence and limits

The synchronized candidate was checked locally with `mise run ci`: formatting, default Clippy, 108 tests, native release build, and CLI help smoke all passed. `mise run ci-preview` passed strict Clippy and three egui preview tests. Windows-target release builds for the utility, Win32 UI preview, and egui preview passed; Windows-target preview Clippy passed. `mise tasks validate` accepted all 25 tasks, and `mise run --skip-tools security` passed actionlint, a redacted full-history Gitleaks scan, and an offline zizmor audit. These were software-only checks; they did not launch the production tray or send device commands.

Hosted CI on public checkpoint `7cb0440` passed the [Ubuntu Core CI job](https://github.com/spencer-life/viperpilot/actions/runs/36612610509/job/109557553770), [Windows Core CI job](https://github.com/spencer-life/viperpilot/actions/runs/36612610509/job/109557553566), and [Security checks job](https://github.com/spencer-life/viperpilot/actions/runs/36612610273/job/109557552100). Their successful conclusions were rechecked through GitHub on 2026-09-29. The core jobs ran hardware-free source/test/build/preview tasks; the security job ran workflow lint, redacted history scanning, and workflow security analysis. These results do not validate the production tray on a desktop or any physical-device behavior.

A fresh detached worktree at that same checkpoint passed `mise run --skip-tools build-native`, `test` (**108 passed, 0 failed**), `test-egui-preview` (**3 passed, 0 failed**), `fmt-check`, `lint`, and `ci-preview` on Linux/Rust 1.98.1 on 2026-09-29. Compilation outputs and dependency caches were isolated; no Windows UI or device was accessed. Task validation found 25 repository tasks (30 including inherited global tasks). The earlier failed task startup could not register mise trust in the sandbox; its retry passed before any source changes. The [tooling decision](tooling-decision.md) records the assessment and its limits.

The preview UI's accessible names and enabled states were checked with Windows UI Automation in earlier development work. The owner-drawn Win32 preview controls were reported as panes without `InvokePattern`; screen-reader behavior and the production UI's Windows display-scale appearance remain unverified. The egui preview exposes standard control roles, but keyboard-only and screen-reader operation remain unverified. UI Automation was not rerun as part of the synchronized candidate checks.

No physical-device validation of the new tray pair-selection or hotkey path is recorded here. A build, unit test, UI preview, GET result, or planned write is not evidence that a hardware write succeeded. Use the [supervised validation checklist](quick-switch-supervised-checklist.md) when an owner can observe the Windows device session. Keep serials, local paths, raw reports, and screenshots containing private information in local evidence storage rather than this repository.

## Issue #12 software checkpoint — 2026-09-29

Source checkpoint [`6792c6f`](https://github.com/spencer-life/viperpilot/commit/6792c6fe8bc3774ab133a0b830fac8796b421e9d)
adds the persisted local editor, following session commit `f2273e5`.

- `mise run ci` passed formatting, strict default Clippy, **108 core tests**,
  native release build and CLI help smoke.
- `mise run ci-preview` passed strict feature-enabled Clippy, **114 library
  tests** (including six session tests) and **7 headless editor tests**.
  The editor tests include actual text editing and Save, disk reopen, disabled
  Apply, preservation of loaded values, modal isolation, dirty reload, failed
  save feedback and confirmed close state.
- `mise run build-windows` and `mise run build-egui-preview-windows` passed
  without launching either executable. Final `fmt-check`, whitespace and
  task validation passed (25 repository tasks; 30 including inherited tasks).
- Default normal dependencies exclude egui/eframe, and the editor session is
  feature-gated. Native tray, worker, planner, device support and Cargo
  dependencies are unchanged. Independent review found no remaining material
  findings after correcting confirmation isolation and dialog error feedback.

These commands used process-scoped mise trust and disabled automatic tool
installation. Concurrent xwin setup initially failed at a shared clang-cl
symlink; sequential cross-builds succeeded. Run the cross-builds sequentially.
The updated PowerShell scripts received static review only; no PowerShell
parser was available. Hosted checks for this new head must be checked separately
from historical green runs. No manual Windows UI, production tray/hotkey,
installation/startup, performance or physical-device test was run. PR #3 remains
draft, and issue #12 remains open pending delivery and owner-observed UI checks.

## Issue #4 software checkpoint — 2026-09-29

Source checkpoint `28b36f2` adds the editor's offline evidence view after the
read-only contract in `d28661c`. See [contract details](capability-contract.md).

- `mise run ci` passed formatting, strict Clippy, **114 core tests + 3 integration
  tests**, native release build and CLI help smoke.
- `mise run ci-preview` passed strict feature Clippy, **120 library tests,
  9 headless editor tests and 3 integration tests**.
- Sequential `build-windows` and `build-egui-preview-windows` passed without
  launching either executable. Default normal dependencies still exclude
  egui/eframe; no Cargo dependency or production planner/engine/tray change.
- Independent review found and fixed silently ignored unknown fields in the
  pass-through/unassigned JSON actions. All four action variants now reject
  extras without changing valid V1 serialization. Final review found no remaining
  material issues. All eight scope axes reject inherited evidence; timestamp,
  GET-only power and collection-order changes do not falsely mark state stale.
- Documentation links, whitespace and all 30 available mise tasks validated.

Checks used process-scoped mise trust with automatic installation disabled.
Supplied snapshots do not prove live GET or baseline immutability. Every custom
request remains rejected at the combination gate; no WritePlan or setter is
created. No manual Windows, tray/hotkey, performance, startup or mouse test was
run. PR #3 remains draft; #4 stays open for review/delivery and #13 remains gated.
Hosted checks for the published head must be checked separately.

## Next work and deferred tests

- [Issue #12](https://github.com/spencer-life/viperpilot/issues/12): local editing and persistence are implemented in the optional editor; review software checks and complete deferred owner-observed Windows UI validation. Every enabled setting maps to V1 intent; unsupported settings are explicitly unsavable.
- [Issue #4](https://github.com/spencer-life/viperpilot/issues/4): [read-only exact-scope contract](capability-contract.md) and strict request preflight implemented; all custom combinations remain blocked. Review and delivery remain; no production allowlist changed.
- [Issue #13](https://github.com/spencer-life/viperpilot/issues/13): later promote proven custom settings into quick switching, after storage, contract, field and combination evidence gates pass.
- [Issue #5](https://github.com/spencer-life/viperpilot/issues/5): software CI is green at the recorded checkpoint; required-check enforcement and the stacked PR merge order remain open. Public `main` was unprotected with no rulesets when rechecked on 2026-09-29.
- Manual desktop/device gates remain open in [#6](https://github.com/spencer-life/viperpilot/issues/6), [#7](https://github.com/spencer-life/viperpilot/issues/7), [#8](https://github.com/spencer-life/viperpilot/issues/8), [#9](https://github.com/spencer-life/viperpilot/issues/9), [#10](https://github.com/spencer-life/viperpilot/issues/10), and [#11](https://github.com/spencer-life/viperpilot/issues/11). The owner is away from the PC; no new manual tests are authorized by this documentation refresh.

## Release gates

Keep the tray integration in draft until the documented full-profile and hotkey paths pass with owner-observed evidence. Before a hardware write, review a complete immutable baseline, device identity, and exact plan; test one logical field at a time; independently read it back; and restore the reviewed baseline before the next experiment. Do not kill Synapse or use input interception/injection as a fallback. Record process-exit and mouse power-cycle results before claiming persistence.

For customization, unverified values may be saved only as clearly marked local drafts; Apply must stay disabled until the target device, field, and complete combination pass their documented evidence gates. Local saved profiles are not additional onboard slots.
