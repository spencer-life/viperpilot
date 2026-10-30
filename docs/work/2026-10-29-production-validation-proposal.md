# Supervised production validation proposal — 2026-10-29

## Decision and boundary

This is a reviewable preparation plan, not launch or write authorization. The
current production entry point remains blocked before effects. Do not bypass it,
install a candidate, change startup, register a hotkey, touch legacy data or send
a mouse command to carry out this document. The owner has authorized ongoing
software work, isolated UI checks and remote repository updates. Production
launch, installation/startup and mouse writes retain their separate gates.

The offline implementation and tested isolated UI paths have recorded evidence.
No additional scoped code defect was found in the readiness audit. Production
coexistence, complete-profile/hotkey behavior, performance, process-exit and
power-cycle persistence cannot be inferred from those results.

## Candidate inventory

| Item | Current status |
| --- | --- |
| Runtime source | Enter fix 71a8010, retained below local documentation 512bab4 and test layer d30547a |
| Blocked production release SHA-256 | e0299f1f1b41c5cd3178a38a5e200702dc9d283581d0d8a4ecfbe2652d077006; compiled only, never launched |
| Native preview SHA-256 | 1cad136fb3aba657514e323df70ae3b421d04cb4b9efe3728d50f19418db79f2; synthetic tests passed |
| Editor SHA-256 | c05b47880fef6ae0702961adb104c00447687a38215cf566ec615b7807569a0a; separate fixture observations |
| Future launchable candidate | Not built or authorized; any reviewed gating/source changes require a fresh inventory, hash and checks |
| Planned executable basename | viperpilot-development.exe in a new private staging folder, preserving the installed legacy basename/path |
| Runtime artwork | Inventory icon, compact mouse bitmap and dashboard bitmap from the same build; verify hashes in the private session manifest |
| Installation | None; future explicit staged launch only after prerequisites pass |

Actual private absolute paths belong in the session manifest, never this public
record. The planned filename alone does not isolate global hotkeys or the mouse.

## Prerequisites before any production launch proposal can pass

1. **Storage isolation must be real.** The tray calls StoragePaths::discover
   for worker commands; it does not have the editor's draft-root option.
   Current default is LOCALAPPDATA/ViperPilotDevelopment. A temporary editor
   root does not configure tray storage. Before a future session, either review
   explicit root plumbing with fail-closed legacy-path checks, or inventory and
   approve the exact development directory. Preserve existing development bytes;
   do not repoint process-wide LOCALAPPDATA as a shortcut. Never auto-import the
   legacy config, aliases or baseline.

2. **Refresh currently has a startup side effect.** refresh_state calls
   set_startup_value before opening the device, even when start_with_windows is
   false (which can delete the development Run value). An explicit
   SetStartWithWindows command also writes it. A no-startup-change test therefore
   needs a reviewed code boundary preventing all registry mutations in that
   session, including refresh, not just a false setting. The development Run
   value is ViperPilotDevelopment; the installed legacy value must stay untouched.
   Test any future boundary using synthetic dependencies before real launch.

3. **Resolve hotkey ownership.** The source registers Ctrl+Alt+P during launch.
   The legacy app or another app may own it. No registration probe was run.
   The owner must decide the coexistence arrangement before any registration.
   Do not stop the legacy app, change its startup, steal its binding or substitute
   an untested global shortcut incidentally. Initial observational validation
   should not arm a device-writing hotkey.

4. **Baseline provenance and identity are missing from this proposal.** Obtain
   explicit authorization for a later owner-present read-only session first.
   Privately record connected VID/PID, full mouse and dongle serials, firmware,
   transport and collection identity. Identify the complete immutable first
   baseline and provenance; preserve its original bytes and hash. An existing
   file, a caller-supplied snapshot, source inspection or a GET result alone
   does not prove immutability or device support. Do not overwrite a first
   baseline, copy legacy records automatically, guess raw fields or proceed
   through ambiguity.

5. **Separate observation from writes.** A future reviewed observation mode must
   fail closed on setters, profile actions, recovery actions and hotkeys, while
   preserving truthful unavailable/wrong-device state. Production refresh can
   read HID and save development configuration; a synthetic preview is not a
   substitute for documenting those effects. Keep the current launch gate
   until the exact observational arrangement is reviewed and authorized.

## Subsequent phases, each with its own evidence

- **Owner-observed desktop:** finish screen-reader speech, actual monitor
  scales/high contrast and IME/clipboard input checks. No setting is changed
  solely to make an automated test appear complete. Existing tool text entry
  is not an IME or independent clipboard-paste result.
- **Read-only production observation:** only with the safe launch/storage/
  startup/hotkey arrangement above, recorded identity and explicit permission.
  No setters are included.
- **One logical field experiment:** exact immutable baseline, full identity and
  exact proposed WritePlan must be reviewed. The utility's serial suffix is the
  final guard, not a replacement for review. Independently reread the field,
  restore the recorded baseline under the same gates, and reread restoration
  before another field. This proposal contains no approved values or write plan.
- **Complete profiles and production hotkey:** only after field and combination
  evidence permits them. Independently verify every relevant field for both
  directions; record process exit and owner-performed mouse power-cycle results.
- **Performance:** use the existing methodology after production launch is
  legitimately available. Record tray-only idle resources and switching latency
  separately from editor overhead; never reuse preview metrics as production.
- **Delivery:** local CI remains the current validation route. New PR events and
  hosted Actions are deferred while quota is exhausted; no protections or
  required checks are disabled. Preserve draft status, actual acceptance gates
  and separate merge/release/installation decisions.

## Cleanup and rollback

Only close the exact staged processes owned by the authorized session. Keep
legacy processes and Synapse untouched. Preserve original fixture/development
bytes, baseline, logs and failed experiments. Restore configuration or registry
only if that mutation and rollback were explicitly approved; a no-startup-change
session should need no registry rollback. Any mouse restoration is itself a
guarded write requiring the normal identity/baseline/plan review.

Keep all raw identity and diagnostic evidence private. Record sanitized measured
process-exit and power-cycle outcomes in README only after an actual supervised
test. Stop on unknown final state, mismatch, disconnect or failed recovery.

## Independent review

A fresh read-only review found no material findings in this proposal. The
storage, startup and hotkey constraints match the current source; the
authorization, baseline, phase and rollback boundaries are explicit. Review
does not authorize execution. The identified storage selection, startup
suppression, hotkey ownership, setter-blocking observation behavior and
private identity/baseline prerequisites remain unresolved for production.
