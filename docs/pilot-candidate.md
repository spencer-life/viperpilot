# ViperPilot portable pilot candidate

This is a supervised validation candidate, not a validated replacement for the
installed utility. The ordinary production launch and installer remain blocked.
No hardware test, installation, startup change or mouse write was performed to
create this package. Keep the old app and its data as the fallback.

## Try the redesigned interface without a mouse connection

Extract the entire ZIP to a new folder. Run `Preview.cmd` for the native compact
switcher/dashboard with synthetic state. Run `Editor.cmd` for the local draft
editor; its files stay in this package's `data/editor` directory. Editor Apply
remains disabled. These launchers do not install anything or change startup.

The isolated preview has measured layout, keyboard, popup-menu and pair-swap
coverage. For a separately labeled synthetic notification-area preview, run
`.\viperpilot-ui-preview.exe --tray-icon --pilot-read-only`. Its tooltip is
`ViperPilot PREVIEW — no device access`; Alt+F4 hides it and its menu Exit closes
it. It has no worker or device access. Actual notification-area appearance,
tooltip and Shell activation/hide-reopen remain unobserved because the UI tool
could not target the notification area. Production profile behavior still needs
approved live validation. Synthetic previews do not prove device persistence.

## Review an existing baseline offline

`.\viperctl.exe inspect-snapshot 'PATH_TO_EXISTING_BASELINE'` validates and prints
only that supplied file without accessing a mouse or writing application data.
Keep its identity-rich output private. Passing format validation does not prove
that the file is the original baseline or matches the connected device.

The new `observe` command performs GETs without creating a snapshot, baseline,
configuration or journal. It still requires separate approval before any live
device access. Do not substitute `snapshot`, which creates development storage
and can create a first baseline. Review the [phased device preparation](https://github.com/spencer-life/viperpilot/blob/feat/pilot-candidate-refresh/docs/device-review-preparation.md)
before any hardware command. No enumeration or GET capture has been performed.

## Prepare a supervised session

Use a dedicated pilot data folder. Privately review the current full mouse and
dongle identity, VID/PID, firmware, HID collection/path and transport. Review the
provenance and hash of the complete immutable first baseline; never replace it
with a new snapshot just to make a guard pass. Stage an explicitly reviewed copy
in the pilot folder's `backups/baseline-v1-FULL_SERIAL.json` (the existing utility
sanitizes unsafe filename characters). Verify the original and copy hashes.
Nothing automatically imports legacy records or captures a new baseline.

Open PowerShell in the extracted package folder. Run
`.\Start-Pilot.ps1 -FullSerial 'FULL_MOUSE_SERIAL' -Root 'ABSOLUTE_PILOT_FOLDER'`.
The default is device read-only: profile actions are disabled.
GET reports and development configuration/status writes can occur. The full
identity must match the reviewed baseline, including path and firmware; a
different transport/port/path requires review rather than weakening this guard.

The pilot does not register Ctrl+Alt+P, change any startup value, or expose the
startup control. Another development instance must be closed before selecting
different pilot options. The installed legacy process is never stopped for you.

## Enable the existing pair only after review

Before considering `-AllowEstablishedProfiles`, arrange a single device writer
with the owner, review the exact Developer/Gaming transition and recovery plans,
and satisfy the normal device/field/combination gates. The flag is a final
session launch guard, not evidence or approval by itself. No custom combination
is enabled. The shared apply route retains polling restrictions, fresh preflight,
serialized setters, journaling, independent readback and rollback.

Validate one logical field at a time, reread it independently, restore the
reviewed baseline and reread restoration before another field. Only then test
both complete profile directions through buttons, independently verify them,
and observe process-exit and owner-performed power-cycle behavior. Unexpected
identity, changed baseline bytes, disconnect or uncertain final state stops the
session. Keep raw identities and evidence private.

Global hotkeys, installation/startup and public release remain separate follow-up
decisions after their documented validation. This ZIP does not establish those
results or promise safe daily replacement tonight.

## Return to the old app

Close only the exact pilot instance. If device settings changed, restoration is
itself a guarded write requiring the reviewed identity, immutable baseline and
exact plan. Verify restoration before resuming the old utility. The package has
not modified its binaries, configuration, baseline, shortcuts or startup.
