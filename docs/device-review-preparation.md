# Real-device review preparation

Updated 2026-11-02. This document defines staged owner-present review and
authorizes no hardware action by itself. Private offline `viperctl
inspect-snapshot` inspection of the existing baseline candidate passed validation
and its hash remained unchanged. No identifiers, path, hash or snapshot
contents are recorded here. No baseline import occurred. On November 2 the
owner separately approved OS enumeration and GET-only observation while baseline
provenance remained uncertain. That capture succeeded; it does not authorize
setters or establish the backup's original provenance.

The new `viperctl observe` command is intended for an explicitly approved,
read-only capture. It refuses extra arguments before device access, applies the
existing Synapse-running refusal, opens one unambiguous supported device,
performs the existing snapshot GETs, validates the snapshot, and prints only
JSON. It does not discover storage paths, read or write configuration, create a
snapshot or baseline, journal a plan, or call a setter. The separate
`viperctl inspect-snapshot PATH` command validates and prints a supplied local
snapshot without device access or file writes. Enumeration and observation
were run only under that later explicit owner approval.

Commands below describe gates, not blanket authorization. Enumeration and
observation have later private execution evidence; plan generation and setters
remain unexecuted in this session. See the dated owner-session record.
Source-level support statements are not evidence about the connected mouse. Do
not infer support for a device, transport, firmware, setting, or transition
from this document.

## Phase 1: private identity and baseline review

Before asking to access the device, privately establish the provenance of the
original immutable baseline and preserve its original bytes and cryptographic
hash. Review the full mouse and receiver serials, VID/PID, firmware, transport,
interface and collection identity recorded in that baseline against its
provenance. This is review of the existing baseline evidence; do not enumerate
or query the live device in this phase. Keep full identifiers, raw assignments,
paths, baseline data and hashes in private records. Stop on any missing,
ambiguous or mismatched baseline evidence. Never replace, recreate or
automatically import a baseline.

`inspect-snapshot PATH` reads only its supplied file, validates it with the
existing snapshot reader, and prints JSON to stdout; it does not open a device
or create files. Its output contains private identity and must remain in the
private review session. Schema validation does not establish baseline
provenance. The offline review described above is complete; no baseline path or
contents are recorded here.

```powershell
viperctl inspect-snapshot "<private reviewed candidate path>"
```

## Phase 2: separately approved read-only capture

Obtain explicit approval for OS enumeration and a GET-only observation in an
owner-present session. If baseline provenance is uncertain, the owner may approve
this diagnostic phase separately to help establish identity; provenance and
restore-point review remain mandatory before setters. Only after that approved
capture, compare the returned live identity privately against the reviewed
baseline and stop on any mismatch or ambiguity. Keep output in a reviewed
private capture location; do not pipe it to a repository file or use
`snapshot`, which creates storage files. Planned commands, not run:

```powershell
viperctl enumerate --json
viperctl observe
```

Preserve the exact binary hash, command, time, OS, connection mode, Synapse
state, output bytes and errors in private evidence. A GET result does not prove
a write, baseline provenance, persistence, or support beyond the exact recorded
identity and state.

## Phase 3: exact proposed-plan review

Only after read-only identity review, prepare and review exact plans for the
established pair. Both complete presets require current DPI of 1600 × 1600 and
an active stage set to 1600 × 1600. If the reviewed baseline does not record
both values, stop for review; do not continue to plan generation or a write
session. Do not describe this as preserving arbitrary baseline DPI.

The expected profile intent is Developer at 1000 Hz with
Mouse4 mapped to the exact Ctrl+Alt+F10 assignment and Mouse5 to the exact
Ctrl+Alt+F11 assignment; Gaming at 4000 Hz with both side buttons returned to
their recorded native assignments. Compare each exact plan and every raw
assignment to the private baseline; do not guess or infer a native value. These
expectations describe a plan to review, not a claim that either plan is
currently safe or applicable.

Plan commands read the baseline through `StoragePaths::discover`, which uses
the `ViperPilotDevelopment` data root. `inspect-snapshot` reads only its
explicitly supplied file and does not select or configure that root. Plan
generation therefore has a separate prerequisite: review the isolated
development data arrangement and baseline staging before running either plan
command. If the matching baseline is not available there, stop. Do not run
`snapshot` to create one or use an automatic import path as a workaround.

Planned, unexecuted GET-only plan commands:

```powershell
viperctl plan developer
viperctl plan gaming
viperctl plan-restore "<private reviewed target snapshot path>"
```

`plan-restore PATH` refuses all additional arguments before device work, applies
the existing Synapse refusal, reads and validates the supplied target snapshot,
opens one unique supported device and performs a fresh validated snapshot GET.
It requires the current device's matching immutable baseline, then uses full
identity equality and the existing restore planner. It prints the same JSON
plan shape as other planners, including packet intents, and creates no storage,
journal or configuration files and sends no setters. Unsupported plan
operations remain explicitly visible; a plan does not claim they can be
applied.

For every setter command (`apply`, `apply-dpi-1600`, `apply-button`,
`apply-polling`, and `restore`), pass both `--reviewed-plan PATH` and the usual
`--confirm-device SERIAL_SUFFIX`. Missing, duplicate or unknown guard arguments
are rejected before device access. Review the exact JSON plan output and retain
it as the artifact. The setter rereads that artifact once, recomputes the
command-specific plan from a fresh device GET and current immutable baseline,
applies the command's existing proven-polling and clean-start restrictions,
then compares the complete plan and packet intents. Only the fresh capture
timestamp and GET-only power field may differ; all identity, before-state,
target, operations, rollback operations, skipped fields, force choices and
packet intents must match. Unknown fields or any other difference fail before
journaling or setters. The reviewed file is an authorization check on the
recomputed plan; its operations are never executed directly. The supervising
reviewer records and verifies the artifact SHA-256 using .NET and holds the
reviewed handle against writes during the command.

Review the plans, baseline provenance, complete identities, recovery path and
exact serial-suffix guards together. A plan and matching artifact are not
authorization. Obtain separate explicit approval before any write session.

## Phase 4: one-field experiments, each separately gated

For every approved experiment, change only one logical field. Before the setter,
reconfirm the reviewed complete identity, immutable baseline, expected current
state and exact write plan, including the bounded retry and conditional recovery
scope below. Independently GET the field after the setter. Then
restore that field to the recorded baseline and independently GET the restored
value before considering another field. An ambiguous acknowledgement enters the
engine's bounded readback classification; it does not mean unconditional immediate
halt or unconditional restoration. Terminal uncertainty follows the engine's
stop classification. Preserve diagnostics and independently establish state
after failure before proposing another command.

### Transaction retry and recovery scope (2026-11-02)

Approval must identify the exact command, candidate binary and reviewed artifact,
including forward operations and conditional recovery operations. The current
source permits the following bounded behavior; these are not measured flash-write
counts or a single transaction-wide limit:

- Each physical SET packet may be sent up to eleven times including its initial
  send, with immediate retransmission only after a validated BUSY response.
  Each send may drain up to eleven acknowledgement reads; stale validated replies
  do not themselves cause another send.
- An ambiguous logical write is independently read back. An observed target may
  be accepted; an unchanged pre-write field may permit one logical resend.
  A single-report operation can therefore reach twenty-two sends before any
  separate recovery. Unchanged readback cannot prove an ambiguous forced
  equal-value DPI write succeeded.
- Wireless V2 polling uses two physical reports, each with the transport limit.
  An unverifiable partial operation aborts forward progress and may force both
  reviewed polling recovery reports, even if aggregate readback looks unchanged.
  This recovery validates identity/state preflight but permits the current
  polling field to differ from its expected pre-write value.
- Automatic rollback makes one reverse traversal of eligible attempted
  operations, with the same bounded write/readback machinery and no recursive
  rollback transaction. Recovery targets are the reviewed transaction-before
  values, which need not equal the immutable baseline. Recovery is conditional,
  not guaranteed; errors remain failures and earlier inverse operations may
  continue only when the existing failure classification permits it.
- Identity mismatch, failed validated readback, unexpected third-value state or
  failed logical resend stops transaction writes where classified by the engine.
  Final observational reads may still occur. A failed transaction is never
  permission to restart it or issue a manual recovery command automatically.

Every experiment and subsequent baseline-restoration command requires its own
reviewed artifact and explicit approval. Conditional recovery already reviewed
inside that transaction is distinct from approving another command. Source:
`src/windows.rs` transport exchange and `src/engine.rs` write/recovery handling.

## Phase 5: later complete-profile and product gates

Complete-profile validation can begin only after its combination and recovery
gates pass. It requires separately approved, supervised profile transitions
and independent readback of every relevant field. Production tray launch,
global hotkey testing, process-exit persistence, owner-performed mouse
power-cycle checks and production performance measurements are later,
independent gates. They require the owner present and their own reviewed
coexistence, storage, startup, hotkey and rollback arrangements. No tray launch,
hotkey registration, installation, startup change or performance measurement
is performed or authorized by this preparation.
