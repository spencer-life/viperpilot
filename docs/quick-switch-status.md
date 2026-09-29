# Quick-switch implementation and validation status

Updated 2026-09-29. This document describes the current ViperPilot development checkpoint. It is not a release approval or a claim that the synchronized public branch has passed hosted CI or physical-device validation. Track open work in the [public issue tracker](https://github.com/spencer-life/viperpilot/issues).

## Goal and scope

The priority is fast access from the native Windows tray or hotkey, with no browser or settings navigation for an established profile pair and little background work. Users should eventually be able to create their own settings for DPI, buttons, polling, and other fields that are proven safe. The Developer and Gaming profiles are protected example/recovery presets; they do not define the full customization model. Full Synapse parity and support for every recent mouse are not requirements.

The application currently supports only its documented Viper V4 Pro device and field boundaries. Local names and drafts are not additional onboard slots, baselines, device authorization, or proof of current hardware state. Unsupported devices, fields, transitions, and ambiguous states must continue to fail closed.

## Implemented software

- A validated local profile-alias library supports protected built-ins, user-named aliases, and a two-entry quick-switch selection.
- Tray actions and the global hotkey reload the selection, verify current device state, and use the existing guarded profile-apply path. Pair selection and library reload do not write to the device.
- The native compact switcher opens first, with a Details view for the existing read-only inspector and recovery actions. The UI preview is isolated from the production tray and hardware worker.
- User-authored profile intent has a versioned local draft model and storage separate from complete-preset aliases and device baselines. The egui editor remains a synthetic preview; its Save and Apply controls are disabled. The user-facing draft editor is incomplete.
- Hardware-free tasks cover formatting, lint, tests, native build, and CLI smoke. Separate preview tasks exercise synthetic UI states and Windows accessibility metadata.

The guarded write engine also checks each planned field against the last trusted snapshot before writing, advances that snapshot only after a complete validated readback, verifies device identity on readback, and stops when identity or field state is ambiguous. A rejected write is classified using readback; an ambiguous acknowledgement cannot be treated as proof that an equal-value DPI write succeeded. These checks reduce stale-plan and rollback risk. They do not establish additional device support or authorize new writes.

## Validation evidence and limits

The synchronized candidate was checked locally with `mise run ci`: formatting, default Clippy, 108 tests, native release build, and CLI help smoke all passed. `mise run ci-preview` passed strict Clippy and three egui preview tests. Windows-target release builds for the utility, Win32 UI preview, and egui preview passed; Windows-target preview Clippy passed. `mise tasks validate` accepted all 25 tasks, and `mise run --skip-tools security` passed actionlint, a redacted full-history Gitleaks scan, and an offline zizmor audit. These were software-only checks; they did not launch the production tray or send device commands. Hosted CI has not yet run against the synchronized candidate.

The preview UI's accessible names and enabled states were checked with Windows UI Automation in earlier development work. The owner-drawn Win32 preview controls were reported as panes without `InvokePattern`; screen-reader behavior and the production UI's Windows display-scale appearance remain unverified. The egui preview exposes standard control roles, but keyboard-only and screen-reader operation remain unverified. UI Automation was not rerun as part of the synchronized candidate checks.

No physical-device validation of the new tray pair-selection or hotkey path is recorded here. A build, unit test, UI preview, GET result, or planned write is not evidence that a hardware write succeeded. Use the [supervised validation checklist](quick-switch-supervised-checklist.md) when an owner can observe the Windows device session. Keep serials, local paths, raw reports, and screenshots containing private information in local evidence storage rather than this repository.

## Release gates

Keep the tray integration in draft until the documented full-profile and hotkey paths pass with owner-observed evidence. Before a hardware write, review a complete immutable baseline, device identity, and exact plan; test one logical field at a time; independently read it back; and restore the reviewed baseline before the next experiment. Do not kill Synapse or use input interception/injection as a fallback. Record process-exit and mouse power-cycle results before claiming persistence.

For customization, unverified values may be saved only as clearly marked local drafts; Apply must stay disabled until the target device, field, and complete combination pass their documented evidence gates. Local saved profiles are not additional onboard slots.
