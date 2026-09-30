# Draft capability contract

Updated 2026-09-29 for issue #4. This is a read-only software contract for
semantic draft requests. It records historical field evidence and rejects
requests that lack the required scope or state. It does not create a write plan,
send a setter, or permit Apply. No new mouse model or hardware behavior was
validated in this session.

## Registered scope and rules

The registry has one evidence scope: Razer Viper V4 Pro, VID `1532`, wireless
PID `00e6`, firmware `1.4`, transport `WirelessV2`, interface `3`, usage page
`000c`, usage `0001`. Every scope component must match. Wired transport, other
models, firmware and collections receive no inherited evidence. Adding a
registry entry requires a reviewed code change and independently recorded
hardware evidence; loading a profile or request cannot register support.

| Field | Valid local intent | Recorded evidence in that exact scope |
| --- | --- | --- |
| Name | Validated local display name | Local metadata; never a device field |
| DPI | Positive X/Y targets | Only unchanged `1600 × 1600` observed/read back; no changed-value proof |
| Polling | Positive target | Only `1000 → 4000` and `4000 → 1000` Hz transitions |
| Mouse4 | Schema-valid semantic action | Native pass-through and exact Ctrl+Alt+F10 |
| Mouse5 | Schema-valid semantic action | Native pass-through and exact Ctrl+Alt+F11 |
| Other actions/settings | Only fields representable by draft V1 can be saved | Unassigned, unmodeled and other actions remain research-only |

Typed rules are shared by field classification and request preflight. A target
of 4000 Hz does not make an 8000→4000 transition proven. Unchanged values are
distinct from changed-value evidence. Individually recorded fields do not
establish a safe complete custom profile: there are no approved custom-profile
combinations in this contract.

## Request format and preflight

`DraftRequestV1` wraps the unchanged `ProfileIntentV1` with its own schema
version, contract revision and explicit target scope. Parsing validates the
request and nested intent. Malformed, unknown and future schema data is
rejected; a request is never silently migrated or repaired. Reusable scope and
requests contain no full serials, paths, raw assignments or baseline data.

`preflight_draft_request` accepts the request and caller-supplied baseline,
expected state and current state. It checks missing/incomplete snapshots,
exact scope, full private identity agreement, ambiguous state, stale logical
state and unsupported values/actions/transitions. Logical comparison treats
collection ordering as metadata and ignores capture timestamps and GET-only
power observations; changes to relevant DPI, polling or button state remain
blocking. Request preflight always ends in rejection, even if individual field
checks pass, because custom combinations and promotion are unapproved.

These supplied snapshots do not prove that a fresh GET occurred, that the
baseline is immutable, or that an owner reviewed the full identity. The
future dispatch boundary must independently enforce those conditions. The
contract does not replace the production engine's per-setter readback checks,
the serial-suffix command guard, or the supervised checklist.

## Readback, rollback and recovery

Before any future setter, independently reread the selected device and verify
its reviewed full identity and expected field state. After each setter, perform
an independent GET before considering another write. An ambiguous response or
readback stops further writes; it is never treated as success. Recovery requires
the reviewed original baseline and a supported, inspectable restoration plan;
never guess raw assignments or retry through uncertain state.

The existing documented side-button GET-mode opacity does not authorize copying
that byte into a setter. Existing production handling stays unchanged. Baseline
evidence is preserved, and a contract assessment cannot certify an unproven
rollback or process-exit/power-cycle result.

## Editor, migration and rollback

The optional editor requests an offline assessment with no verified device scope.
It shows field evidence and blockers plus an explicitly labeled V4 Pro reference
summary. That reference is not a connected-device identification. Save continues
to persist valid local drafts independently of capability or write eligibility;
Apply remains disabled.

Draft intent/library schemas and paths remain V1. No alias, configuration,
diagnostic, report or immutable baseline format changes. The request is a
separate data type and is not automatically saved or promoted. Older application
versions can still read valid draft files. Unknown extra fields in every button
action variant now fail explicitly, including pass-through and unassigned; files
previously accepted only by silently ignoring such fields are preserved and
rejected, rather than rewritten. Future request/schema revisions fail
explicitly; retain their original bytes and use a compatible version rather than
downgrading or resetting them. Reverting this code slice restores the earlier
assessment behavior without rewriting stored drafts or evidence.

Hardware-free tests cover the software guards. Manual Windows accessibility,
presentation and mouse experiments remain deferred. Issue #13 owns later
promotion, after its device, field, combination, restoration and lifecycle gates
pass. PR #3 remains draft.

## Software validation

At source `28b36f2` (contract `d28661c`), hardware-free core CI passed 114 library
tests plus 3 integration tests, release build and CLI help. Feature CI passed
120 library, 9 headless editor and 3 integration tests. Sequential production
and editor Windows MSVC cross-builds passed without launching either executable.
Independent final review found no remaining material issues; an unknown-field
parsing defect was fixed and tested for all four button-action variants.
The default dependency graph still excludes egui/eframe. These checks do not
establish desktop accessibility, hardware behavior or release approval.
