# Quick-switch supervised validation checklist

Updated 2026-09-29 for supervised validation of the native quick-switch integration. **Planning only: no step in this checklist is a recorded result.** The current PR 3 candidate remains a draft; green Linux and Windows Core CI plus Security checks establish software validation only. They do not close any desktop or device gate below. Use this checklist with the device owner at a Windows PC. Keep full serials and personal paths in local diagnostic records, not this repository.

The corresponding owner-present issues remain open: [#6 compact switcher accessibility](https://github.com/spencer-life/viperpilot/issues/6), [#7 production tray validation](https://github.com/spencer-life/viperpilot/issues/7), [#8 editor presentation](https://github.com/spencer-life/viperpilot/issues/8), [#9 production performance](https://github.com/spencer-life/viperpilot/issues/9), [#10 changed DPI values](https://github.com/spencer-life/viperpilot/issues/10), and [#11 additional button actions](https://github.com/spencer-life/viperpilot/issues/11). Checklist items stay unchecked until observed and recorded. DPI and button experiments must remain separate one-field sessions with readback and baseline restoration.

## Before launching the PR build

- [ ] Confirm the PR commit being tested and rerun its hardware-free `mise` checks and Windows build. Record commit and binary hash.
- [ ] Record the currently installed utility version, whether its tray is running, the saved Start with Windows setting, and the exact `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` value for this utility. Save a reversible copy of the config and local profile library. Do not alter the first immutable baseline or delete any report.
- [ ] Exit the existing utility through its tray menu before starting the PR build; its named mutex prevents a second instance. Do not kill Synapse. Keep the existing installed executable available for rollback.
- [ ] Review the connected VID/PID, **full** device and dongle serials, firmware/transport identity, and complete immutable baseline. Stop for missing or ambiguous identity, baseline, or raw state. The serial suffix is only the final command guard and does not replace this review.
- [ ] Perform GET-only discovery and inspect the exact plan for each proposed complete-profile regression. A plan is not proof that a write succeeded. Do not proceed if the plan contains any unsupported device, field, polling transition, or unexplained value.

A portable PR tray launch is not automatically side-effect-free: startup refresh reconciles the Run value to the current executable when Start with Windows is enabled. Preserve the original Run value first, and restore it after testing. Launching the portable binary does not require installing it or enabling startup.

## UI-only checks before a device write

- [ ] Observe the tray menu and status window. The two selected quick-switch names should be directly visible in the tray; Developer/Gaming recovery choices remain available. Labels must distinguish local aliases from onboard slots.
- [ ] Open the 900×520 compact switcher and Details view. Check the mouse silhouette, actual connection/profile/DPI/polling readbacks, error text, six read-only hotspots, focus order, and visibility at the Windows display scale in use. A screenshot, Figma planning file, or build alone cannot establish legibility.
- [ ] Reverse the saved quick-switch pair without applying either preset. Check the two compact button labels, their Windows accessible names, and active highlight all follow the saved names and source presets rather than button position. When the library is unavailable, both compact buttons must be disabled and announced as unavailable; Details must still identify its built-in recovery actions.
- [ ] To exercise pair selection with more than the two built-ins, back up `%LOCALAPPDATA%\ViperV4Utility\profiles-v1.json` (or record that it was absent), then stage the tested `tests/fixtures/profiles/work.json` local alias fixture. It names the existing Developer preset; it is not a new onboard profile.
- [ ] Change the local quick-switch pair and reload the library. Confirm neither action applies a mouse profile. Check that an incompatible pair is unavailable or rejected clearly. Restore the original library bytes, or remove the test file if none existed before.
- [ ] Check an unavailable, corrupt, or externally changed library fails closed and offers a recovery path without silently replacing the file. Preserve the original bytes for restoration.
- [ ] Check disconnected/unknown device state shows no verified profile and does not write. A hotkey request in that state must fail closed. Restore normal connection before any write test.
- [ ] Confirm normal close/hide, tray Exit, second-launch behavior, menu readability, keyboard access, and no idle polling. Record the method and sample interval for idle CPU and memory.

## Observed complete-profile regression

This section uses only the already measured complete Developer/Gaming presets through the new tray/hotkey route. Any **new** field or action needs its own one-logical-field experiment, independent readback, and restoration to the recorded baseline before another field is tested.

- [ ] With the owner present and the reviewed baseline and exact plan in hand, apply one complete preset through a top-level tray action. Independently reread the device with the separate CLI, compare every relevant field to the expected complete preset, and save the report. An app notification alone is not proof.
- [ ] Repeat with the other preset and with `Ctrl+Alt+P` in both directions. Check repeated hotkey input does not queue duplicate writes. Record observed click count, elapsed time, Synapse state, and any device commit delay separately from UI time.
- [ ] Repeat the verified state check after the configuring process exits and after a mouse power-cycle. Record the owner’s physical observation and independent readback for each condition.
- [ ] On any mismatch, lost acknowledgement, disconnect, ambiguous identity, or failed rollback: stop further writes, preserve diagnostics, identify the final independently read state, and restore the reviewed baseline only under the normal `AGENTS.md` gates.

## End of session and evidence

- [ ] Exit the PR tray. Restore the original Run value and any changed local config/library data; leave the immutable baseline and diagnostic reports untouched. Verify the original installed app can be started again if it was running before the test.
- [ ] Record commit/binary hash, OS, full device/firmware/transport identity, connection mode, Synapse state, starting raw state, exact plan, report paths, independent readback, process-exit and power-cycle results, screenshots, timing method/sample count, failures, and ruled-out explanations in private evidence. Summarize the measured conditions and redacted results in the README.
- [ ] Keep the PR draft until its documented profile, hotkey, UI/accessibility, persistence, and performance gates have recorded passing evidence. Do not infer success from compilation or a GET-only plan.
