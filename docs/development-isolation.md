# Development without changing the current app

Updated 2026-09-29 at the owner's request. The currently installed legacy app
remains the daily-use app. No install, launch, startup or device operation was
performed during development.

## Data and builds

Default storage discovery now chooses `%LOCALAPPDATA%/ViperPilotDevelopment`.
It never falls back to `%LOCALAPPDATA%/ViperV4Utility`, reads its config/library,
or imports its immutable baseline. The new folder starts with in-memory defaults;
existing hardware gates still require their own reviewed baseline. No baseline
was captured or copied. Explicit `--draft-root` (editor) and `--root` (catalog)
remain caller-selected; use only separate development or fixture directories.

`mise run ci` and `ci-preview` are hardware-free. Builds stay under the checkout's
`target` directory; tests use temporary roots. Existing preview measurement/test
scripts stage separate executables and fixture roots. Do not point them at the
legacy installation or its data. Compile without launching the production tray.

## Install and manual gates

Both `mise run install-windows` and `scripts/install.ps1` now fail before any
build dependency, file replacement, shortcut edit, process launch or startup
change. The old replacement installer is preserved in Git history, not executed.
The production `tray::run` entry point also rejects before acquiring the
shared mutex, showing an existing window, creating a worker, touching startup,
registering hotkeys or reaching HID. Review found that normal refresh previously
could delete the legacy `ViperV4Utility` Run value before checking the device or
baseline. The new early gate prevents that path entirely. The installed old
executable is unchanged. Offline preview entry points remain available.

A separate reviewed development installer and executable/startup/hotkey identity
are prerequisites for any later installation or concurrent tray use.

Different folders cannot isolate two programs writing to the same physical mouse.
Owner-present manual Windows/mouse validation, full device identity and exact-plan
review, immutable baseline, single-field experiments, independent readback and
restoration remain mandatory. Do not kill or replace the running legacy app.

## Regression evidence and rollback

The regression test seeds a legacy config, library and immutable baseline with
sentinel bytes, loads/saves using the development default path, and confirms all
legacy bytes stay unchanged. The test uses temporary local files and a synthetic
snapshot; it never enumerates or writes hardware.

Rollback of these source changes is to revert this PR and rebuild; legacy files
need no rollback because development never touched them. Existing development
data is separate and must be preserved rather than silently migrated or deleted.
Reverting must not be used to bypass the owner's no-install/no-launch instruction.

## Software validation — 2026-09-29

Core CI passed formatting, strict Clippy, 124 library + 2 CLI + 3 capability +
6 catalog integration tests, release build and CLI help. Preview CI passed
strict feature Clippy, 130 library + 2 CLI + 9 editor + 3 capability + 6 catalog
integration tests. All 30 mise tasks validate. The disabled install task was
exercised and returned exit 1 before build or Windows invocation. The PowerShell
installer was source-reviewed only; no PowerShell runtime was available.

Production Windows MSVC cross-build passed without launching. Independent review
verified the startup finding is resolved: the production tray rejects at its
first statement before legacy process/window/registry/hotkey or HID effects.
These are software checks; native interactive Windows behavior remains deferred.
