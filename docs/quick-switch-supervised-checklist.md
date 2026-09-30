# Quick-switch supervised validation checklist

Updated 2026-09-29 for supervised validation of the native quick-switch integration. **Planning only: no step in this checklist is a recorded result.** The current PR 3 candidate remains a draft; green Linux and Windows Core CI plus Security checks establish software validation only. They do not close any desktop or device gate below. Use this checklist with the device owner at a Windows PC. Keep full serials and personal paths in local diagnostic records, not this repository.

The corresponding owner-present issues remain open: [#6 compact switcher accessibility](https://github.com/spencer-life/viperpilot/issues/6), [#7 production tray validation](https://github.com/spencer-life/viperpilot/issues/7), [#8 editor presentation](https://github.com/spencer-life/viperpilot/issues/8), [#9 production performance](https://github.com/spencer-life/viperpilot/issues/9), [#10 changed DPI values](https://github.com/spencer-life/viperpilot/issues/10), and [#11 additional button actions](https://github.com/spencer-life/viperpilot/issues/11). Checklist items stay unchecked until observed and recorded. DPI and button experiments must remain separate one-field sessions with readback and baseline restoration.

## Before any manual validation

2026-09-30 correction: the historical checklist used the legacy data folder and
shared Windows identity. Those steps are superseded. Preserve the installed
app's binaries, config, library, startup, process and mouse state.

- [ ] Confirm the exact reviewed commit and binary hash. Read
  [development isolation](development-isolation.md). Installer and production
  tray currently reject before effects; do not bypass the gates to use this checklist.
- [ ] Use only the isolated native UI preview or optional draft editor for initial
  UI-only checks, with temporary fixture/draft roots. Neither is production
  switching validation. Do not install, enable startup or point roots at legacy data.
- [ ] Before any later production tray test, review a separate launch/install
  arrangement and executable location, development-only window/mutex/startup
  identity, and global hotkey ownership. A source rename or cross-build is not
  validation of Windows coexistence. Keep the launch gate until this is reviewed.
- [ ] Before device tests, coordinate with the owner so two apps cannot change
  the same physical mouse. Do not stop the installed app or change its startup
  as an incidental test step; agree on the supervised session and rollback first.
- [ ] Review connected VID/PID, **full** device and dongle serials,
  firmware/transport, complete immutable baseline and exact proposed write plan
  privately. Stop for missing or ambiguous identity, baseline or raw state.
  A serial suffix is only the final command guard; a GET or plan is not write proof.

## UI-only checks before a device write

- [ ] Observe the tray menu and status window. The two selected quick-switch names should be directly visible in the tray; Developer/Gaming recovery choices remain available. Labels must distinguish local aliases from onboard slots.
- [ ] Open the 900×520 compact switcher and Details view. Check the mouse silhouette, actual connection/profile/DPI/polling readbacks, error text, six read-only hotspots, focus order, and visibility at the Windows display scale in use. A screenshot, Figma planning file, or build alone cannot establish legibility.
- [ ] Reverse the saved quick-switch pair without applying either preset. Check the two compact button labels, their Windows accessible names, and active highlight all follow the saved names and source presets rather than button position. When the library is unavailable, both compact buttons must be disabled and announced as unavailable; Details must still identify its built-in recovery actions.
- [ ] Exercise pair selection using only an isolated preview/development fixture root, with `tests/fixtures/profiles/work.json` where supported. Never stage fixtures in the legacy data folder. The fixture names the Developer preset; it is not a new onboard profile.
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

- [ ] Close only the reviewed development processes. Preserve legacy binaries, Run value and data. Restore any owner-authorized mouse state under the baseline gates, retain diagnostic evidence, and verify the installed app remains usable.
- [ ] Record commit/binary hash, OS, full device/firmware/transport identity, connection mode, Synapse state, starting raw state, exact plan, report paths, independent readback, process-exit and power-cycle results, screenshots, timing method/sample count, failures, and ruled-out explanations in private evidence. Summarize the measured conditions and redacted results in the README.
- [ ] Keep the PR draft until its documented profile, hotkey, UI/accessibility, persistence, and performance gates have recorded passing evidence. Do not infer success from compilation or a GET-only plan.
