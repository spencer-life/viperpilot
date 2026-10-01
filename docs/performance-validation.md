# Owner-present performance validation

Prepared 2026-09-29 for issue #9. No production Windows resource or physical
switch timing was measured here. This is a repeatable method for a later
owner-present session, not a performance claim or hardware-write authorization.

## Conditions and samples

Record commit and executable hash, Windows version, CPU/core count, display
scale, power plan, session state and companion software privately. Record full
device identity and immutable baseline privately for any later device operation.
Use the same conditions and tools for the candidate and comparison build; keep
startup settings unchanged. Do not kill Synapse. Record other activity, failed
runs, exclusions and deviations instead of silently removing slow samples.

| Measurement | Samples | Boundary |
| --- | --- | --- |
| Cold launch | 10 launches after documented reboot conditions | Process creation to usable tray/compact switcher; label actual cache conditions |
| Warm launch | 30 launches, existing caches, clean process exit each time | Same usable-UI boundary |
| Tray-only idle | 3 runs, 30-second warmup then 60 samples at 5-second intervals | Editor closed, no input/switching; 5 minutes measured per run |
| Editor open/closed cost | 3 matched runs of each state with the same idle sampling | Separate editor process and resident tray measured separately |
| UI response without a write | 30 compact/details openings or navigation actions | Input event to visible/accessible result, record observation resolution |
| Actual tray and hotkey switch | 10 paired round trips per input method, only after supervised profile gates pass | Separate user input, worker dispatch, device commit and independent readback timestamps |

Use process counters consistently: working set, private bytes, peak working set
and cumulative CPU time. Compute CPU time delta divided by elapsed wall time;
state whether percentages represent one core or are normalized to logical core
count. Report raw counters alongside the convention. Compare start/end private
bytes and editor-close behavior; do not describe an editor process as resident
tray cost. Default dependencies exclude egui/eframe, but only actual Windows
process observations can establish runtime resource use.

For UI boundaries, use one documented observation method throughout and report
its timing resolution. Do not subtract an assumed rendering or device latency.
If production timestamps do not separate dispatch, commit and readback, record
that separation as unavailable rather than inferring it from total duration.
The existing editor-only measurement script samples three isolated preview
runs; it cannot substitute for production tray or actual-switch measurements.

## Device gate and recovery

Do not run hardware timing until both complete profiles and the production
hotkey have passed the supervised checklist. Before each session, review the
full device identity, immutable baseline and exact permitted plans. Stop on
unknown state, partial failure or ambiguous readback; use only the documented
reviewed restoration path. New-field experiments remain single-field tests
with independent readback and baseline restoration before the next field.
No timing quota overrides those requirements.

Record process-exit and mouse power-cycle readbacks separately; elapsed switch
time does not prove persistence. Restore prior application/configuration/startup
state afterward. Keep raw results and identifying evidence private.

## Redacted report

Publish sample counts, median, minimum/maximum, p95 when enough samples exist,
variance and excluded/failed runs with reasons. Identify whether each number is
tray-only, editor, UI or device/readback time. For 10-sample sets, show all
redacted durations and avoid a stable-tail claim. Compare the installed baseline
under equivalent conditions before claiming an improvement. Any comparison to
another product needs the same task, device state and sample method.

Include limitations and unresolved failures. Keep issue #9 and release gates
open until owner-observed measurements exist; build times and headless tests
are not Windows performance evidence.
