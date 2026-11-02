# Owner-present continuation — 2026-11-02

The owner returned to the PC and requested completion with the existing
Computer Use chat. This record is in progress. It does not authorize setters,
production launch, installation, startup changes or legacy-app interference.

## Current work and boundaries

- Computer Use resumed against the unchanged packaged source `2df219c66db0`.
  Fresh inventory and preview activation succeeded; the previous activation
  failure is historical, not a continuing assumed blocker. Fresh packaged
  Compact/Device/Buttons keyboard and containment checks, editor Save/reopen,
  shortcut persistence, minimum-size scrolling, owner-approved disposable Delete
  and stale-save/discard/reload passed. The owner confirmed the uniquely labeled
  synthetic icon's hide/reopen and Exit/disappearance; the tool independently
  observed exit 0 but could not capture the notification area. Every owned
  preview/editor process from this pass exited 0. An unexpected close prompt
  after reload could not be attributed while the owner was also using the PC;
  preserve that uncertainty rather than declaring a reproduced defect.
- The existing baseline and packaged CLI hashes were rechecked against the
  private review packet. Baseline bytes are unchanged. The owner is unsure of
  original provenance and confirmed a wireless receiver, then separately
  authorized enumeration and GET-only observation. The sandbox saw no devices;
  native OS enumeration found one receiver descriptor across its collections.
  Observation succeeded with full live identity matching the retained backup.
  Polling and complete DPI state match; the current side-button assignments
  differ from the backup. Baseline staging and every mouse setter remain blocked
  during restore-point review. The owner then accepted the unchanged retained
  file as the restore reference based on its matching earliest recorded before
  snapshot, and manually exited the legacy utility. Native process inspection
  verified closure; a follow-up GET found no identity or controlled-state change.
  A hash-identical copy was explicitly staged read-only in the previously absent
  development root. Original legacy bytes remain unchanged. No setter has run.
- Fresh independent source review found no material blocker in the offline
  inspector or the separately approved enumeration/GET-only observation route.
  GET protocol requests use HID feature reports; they are not setter commands.
- Review found restore-target comparison omitted some full identity fields.
  A focused source fix now requires complete `DeviceIdentity` equality, with
  adversarial field-mismatch tests and a valid restoration test. Focused tests
  passed; independent source review resolved the finding. Combined verification
  and a fresh executable containing the fix were pending at that point; final
  native integration checks now pass as recorded below.
- The existing pilot cannot register the production shortcut. A plan for an
  explicit default-off `--pilot-hotkey` was accepted for implementation. It
  requires write-enabled pilot options, validated live initialization, owned
  registration and guarded dispatch. Conflict must stop only the candidate;
  it must never take another application's binding. No real registration has
  occurred. Source implementation and focused synthetic checks passed; fresh
  independent review found a path-only failure could be hidden by later profile
  classification. The repair carries an explicit safety-stop outcome through
  report conversion, worker latching and UI unregistration, with actual
  fake-device conversion regressions. Follow-up review resolved that P1. The
  ordinary launch/startup gates remain in force.
- Exact restoration preview and CLI transaction binding are being added:
  `plan-restore PATH` performs GET-only planning; every CLI setter will require
  `--reviewed-plan PATH` and a matching recomputed restricted transaction before
  journaling or writes. Only capture time and GET-only power values may differ.
  The supplied artifact is never an executable operation source. Review found
  duplicate JSON members could hide content, empty guard values reached GETs,
  and failure hints omitted the new guard. Recursive duplicate-key rejection,
  pre-HID validation and corrected hints resolved all three findings in follow-up.
- Independent architectural inspection mapped the engine's existing bounded
  retry/readback and conditional recovery behavior. Preparation documentation
  now requires that exact scope in each transaction review; it no longer claims
  an unconditional halt after every ambiguous acknowledgement. No retry/recovery
  behavior was changed and no recovery operation is presumed approved.

## Reviewed software checkpoints

- `fix/reviewed-device-boundaries`: complete restore identity matching and
  required CLI reviewed transactions; focused commits `cb63335` and `fcb21e9`.
- `feat/supervised-pilot-hotkey`: explicit default-off pilot hotkey, terminal
  disarming and ownership cleanup above that boundary layer; `ef2c5f4`.
- Final local native core CI passed 166 library, 15 CLI, 3 capability and
  6 catalog tests. Combined-preview CI passed 184 library, 15 CLI, 17 editor,
  3 capability and 6 catalog tests. Formatting, Windows-policy Clippy, release
  compilation and CLI help also passed. The initial default-sandbox run failed
  atomic-file/junction fixtures with Access Denied; the authorized native rerun
  passed without changing filesystem or OS security policy.
- The previously delivered `2df219c66db0` archive remains preserved. It does not
  contain these fixes. Fresh versioned packaging, static verification and
  measured live validation remain separate next steps.

Historical private evidence now maps prior isolated field operations on the
matching device scope, including independent inverse returns and nearby failed
combined attempts. Some historical tests did not start from the full retained
baseline. This is evidence for review, not an automatic new-candidate or
complete-profile pass. Do not repeat or broaden setters merely to fill a quota.

## Security-check capability gap

The native security-check attempt produced no new actionlint, full-history
Gitleaks or zizmor result. Those tools did not resolve natively and the project
pins Linux-only versions. Mise discovery also rejected the untrusted project
configuration in that subprocess. No global trust, installation, security
policy or WSL state was changed. This is not a new security pass; reuse of the
older source checkpoint must remain explicit. Hardware-free core/preview
evidence from October 29 has not been rerun merely because time passed.

## Next gates

Current packaged isolated UI and disposable-data checks are complete within the
recorded scope. Alternate scale/high contrast, Windows IME and spoken-reader
coverage remain open; the owner declined changing display settings, so leave
that environment unchanged. Next verify a fresh candidate, review reusable
historical field evidence and exact transactions/conditional recovery, then
perform only explicitly approved device/profile/pilot/hotkey tests with
independent readback, persistence and performance evidence. Keep private identifiers,
raw reports and owner paths in ignored local records. Preserve all original
baselines, previous candidate archives and the installed legacy app.
