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

## Isolated preview/editor checks first

2026-09-30 audit correction: previews expose synthetic window state and local
draft editing, not a production tray, global hotkey or live mouse readbacks.
Run these checks only with the owner present; leave every result unchecked until
observed. Do not use a preview result as production evidence.

- [ ] Run the isolated native preview scenarios (`mise run test-ui-preview-windows`) and editor smoke test (`mise run test-egui-preview-windows`). Review scripts and exact binary hashes first. Record UIA Button/Invoke/names/states, including reversed-pair and unavailable-library scenarios. These scripts launch only development previews.
- [ ] Open the preview's 900×520 compact switcher and Details view. Check the mouse silhouette, synthetic connection/profile/DPI/polling values, error text, six read-only hotspots, focus order, narrow windows and Windows scaling. A screenshot or build alone cannot establish legibility.
- [ ] Check reversed-pair synthetic labels and Windows accessible names follow saved-name/source-preset semantics. Verify unavailable compact actions are disabled and Details identifies built-in recovery actions. Standard buttons no longer use the historical custom-colored active painting; assess the visible profile status separately.
- [ ] In the isolated editor, create/edit/rename/duplicate/delete a draft and Save/reopen it using a temporary draft root. Check keyboard-only focus, paste, IME/non-Latin names, screen reader, monitor vs capture and scale. Confirm custom Apply stays disabled.
- [ ] In a temporary editor store, exercise corrupt/externally changed draft-library conflict and recovery paths without replacing original bytes silently. Preserve failed inputs and recovery evidence privately.
- [ ] Confirm each preview/editor closes cleanly. Preview resource measurements describe only that executable; record the method and sample interval and do not report them as tray performance.

## Future production tray checks before a device write

Only after the separately reviewed launch/hotkey/coexistence arrangement and a
deliberate change to the production launch gate. The currently blocked tray
cannot satisfy this section. UI-only here means no setter, not no device access:
production refresh can read the mouse. Do not infer authorization from this plan.

- [ ] Observe the tray menu and live status window. Both selected quick-switch names should be directly visible; Developer/Gaming recovery choices remain available. Labels must distinguish local aliases from onboard slots.
- [ ] Exercise pair selection using only an isolated development fixture root, with `tests/fixtures/profiles/work.json` where supported. Never stage fixtures in legacy data. Change the pair/reload the library and confirm neither action applies a profile; reject incompatible pairs clearly. Restore original development library bytes afterward.
- [ ] Check unavailable/corrupt/externally changed production alias libraries fail closed without silent replacement. Do not confuse this V1 alias library with the local draft store tested above.
- [ ] Check disconnected/unknown device state shows no verified profile; profile actions and hotkey requests must fail closed. Coordinate disconnects with the owner and restore normal connection before any write test.
- [ ] Confirm close/hide, tray Exit, second-launch behavior, menu readability, keyboard access and no idle polling. Record tray-only idle CPU/memory separately from preview/editor measurements.

## Planned complete-profile regression

The complete Developer/Gaming preset definitions have historical measurements;
the new tray/hotkey route has not passed this checklist. This section plans its
future supervised regression and is not recorded evidence. Any **new** field or
action needs its own one-logical-field experiment, independent readback, and
restoration to the recorded baseline before another field is tested.

- [ ] With the owner present and the reviewed baseline and exact plan in hand, apply one complete preset through a top-level tray action. Independently reread the device with the separate CLI, compare every relevant field to the expected complete preset, and save the report. An app notification alone is not proof.
- [ ] Repeat with the other preset and with `Ctrl+Alt+P` in both directions. Check repeated hotkey input does not queue duplicate writes. Record observed click count, elapsed time, Synapse state, and any device commit delay separately from UI time.
- [ ] Repeat the verified state check after the configuring process exits and after a mouse power-cycle. Record the owner’s physical observation and independent readback for each condition.
- [ ] On any mismatch, lost acknowledgement, disconnect, ambiguous identity, or failed rollback: stop further writes, preserve diagnostics, identify the final independently read state, and restore the reviewed baseline only under the normal `AGENTS.md` gates.

## End of session and evidence

- [ ] Close only the reviewed development processes. Preserve legacy binaries, Run value and data. Restore any owner-authorized mouse state under the baseline gates, retain diagnostic evidence, and verify the installed app remains usable.
- [ ] Record commit/binary hash, OS, full device/firmware/transport identity, connection mode, Synapse state, starting raw state, exact plan, report paths, independent readback, process-exit and power-cycle results, screenshots, timing method/sample count, failures, and ruled-out explanations in private evidence. Summarize the measured conditions and redacted results in the README.
- [ ] Keep the PR draft until its documented profile, hotkey, UI/accessibility, persistence, and performance gates have recorded passing evidence. Do not infer success from compilation or a GET-only plan.
