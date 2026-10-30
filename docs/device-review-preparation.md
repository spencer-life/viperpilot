# Real-device review preparation

Updated 2026-10-29. This document prepares a future owner-present review and
authorizes no hardware action. The owner privately ran `viperctl
inspect-snapshot` against the existing baseline candidate; snapshot validation
passed and its hash remained unchanged. No identifiers, path, hash or snapshot
contents are recorded here. No baseline import or device access occurred.

The new `viperctl observe` command is intended for a later, explicitly approved,
read-only capture. It refuses extra arguments before device access, applies the
existing Synapse-running refusal, opens one unambiguous supported device,
performs the existing snapshot GETs, validates the snapshot, and prints only
JSON. It does not discover storage paths, read or write configuration, create a
snapshot or baseline, journal a plan, or call a setter. The separate
`viperctl inspect-snapshot PATH` command validates and prints a supplied local
snapshot without device access or file writes. Enumeration and observation
have not been run as part of this preparation.

Every hardware command below is a planning example and remains unexecuted,
including enumeration, observation and plan generation. The offline
`inspect-snapshot` review is the only command execution recorded here.
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

After Phase 1 is reviewed, obtain explicit approval for OS enumeration and a
GET-only observation in an owner-present session. Only after that approved
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

Planned, unexecuted plan commands:

```powershell
viperctl plan developer
viperctl plan gaming
```

Review the plans, baseline provenance, complete identities, recovery path and
exact serial-suffix guards together. A plan is not authorization. Obtain
separate explicit approval before any write session.

## Phase 4: one-field experiments, each separately gated

For every approved experiment, change only one logical field. Before the setter,
reconfirm the reviewed complete identity, immutable baseline, expected current
state and exact write plan. Independently GET the field after the setter. Then
restore that field to the recorded baseline and independently GET the restored
value before considering another field. Treat ambiguous acknowledgements,
disconnects, mismatches and failed readbacks as stop conditions; preserve the
diagnostics and resolve the actual state before any further action. Every
experiment and restoration requires its own explicit review and approval.

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
