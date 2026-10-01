# Offline profile catalog and promotion boundary

Implemented 2026-09-29 for the software portion of issue #13. This is an opt-in
projection/export, not a new production store or approval of a custom profile.
The resident tray still reads existing preset aliases and accepts only its
existing preset-typed worker commands. Apply remains disabled in the editor.

## Preview and export

Run the native `viperctl` CLI on Windows or the hardware-free host build:

```text
viperctl catalog-preview --root DIRECTORY
viperctl catalog-export --root DIRECTORY
```

Supply an existing local directory containing `profiles-v1.json` and/or
`profile-drafts-v1.json`. Missing libraries use their established in-memory
defaults; preview creates no files or directories. Invalid, corrupt or future
sources abort without repair or reset. Both commands run before hardware
platform dispatch and never enumerate, open or write a mouse.

Preview prints a validated catalog as JSON. Export prints the same catalog and
creates only `profile-catalog-v1.json` in that directory. It refuses an existing
target, including corrupt or future data. It never replaces source libraries,
configuration, diagnostics, reports, or immutable baselines. No command is
run automatically on startup or editor Save.

## Identity, selection and intent

| Entry kind | Catalog identity | Behavior |
| --- | --- | --- |
| Preset alias | Exact existing alias ID, name and preset source | Existing protected built-ins and selected pair preserved |
| Local draft | `draft:<original-draft-id>` plus source draft ID | Frozen copy of semantic intent; never a selectable hardware profile |

Names may coincide across the two categories; type and ID preserve the
distinction. Draft IDs cannot shadow aliases. Catalog decoding reuses both
source libraries' validators, including bounds, stable identity, built-in
protection and the requirement for two distinct preset sources in the pair.
A decoded catalog cannot put a draft in that pair or introduce an approval flag.
Unknown fields and versions fail explicitly, including nested action semantics.

Editing or deleting a source draft afterward does not modify an exported copy.
To inspect current data, regenerate the preview. To export again, preserve or
explicitly remove the old derived export first; the tool will not overwrite it.
This freezing decision applies only to an offline export. A future production
promotion workflow still needs reviewed approval, invalidation and lifecycle
rules; this slice does not commit to live draft references for production.

## Publication and rollback

Export validates and round-trips the complete representation before touching a
temporary file. It writes a unique sibling file, syncs it, then uses a hard link
to publish a complete target without replacement. Concurrent exporters cannot
clobber one another. Unsupported hard-link filesystems abort rather than fall
back to an overwrite. Temporary files are cleaned on ordinary failures. A crash
may leave a derived temporary file; preserve source/evidence and inspect it
before any manual cleanup. This is not a universal power-loss durability claim.

Exact source bytes are checked while loading and immediately before publication.
That detects ordinary edits; it is an optimistic check, not a source-writer lock
or proof that no writer changed a file after the final check. Close editors for a
repeatable export. The result stays unverified derived data in all cases.

Rollback is to ignore/remove only the derived catalog and return to the previous
executable. Original V1 alias/draft files, selected pair, configuration and device
baseline are untouched. No migration replaces those files. Do not delete reports
or baseline evidence while cleaning a catalog export.

## Promotion gate

The pure catalog API can form `DraftRequestV1` from a frozen draft and an explicit
capability scope. Its read-only preflight delegates to the exact-scope contract;
missing, mismatched, stale, ambiguous and unsupported inputs reject. Even all
individually evidenced values end at `CustomCombinationUnapproved`. No WritePlan,
packet encoder, setter or tray/worker integration is added.

Caller-supplied snapshots are not proof of a fresh independent GET, immutable
baseline or reviewed serial identity. See [capability contract](capability-contract.md).
Actual promotion remains blocked until complete-profile, readback, recovery,
process-exit and mouse power-cycle evidence permits it. New DPI/actions and
other mice need their own supervised evidence. No manual Windows or mouse
validation was run; PR #3 remains draft and issue #13 remains open.

## Software checks

Core checks passed 122 library, 2 CLI, 3 capability and 6 catalog integration
tests. Feature checks passed 128 library, 2 CLI, 9 editor, 3 capability and
6 catalog integration tests. Both include strict Clippy; core includes formatting,
native release build and CLI help. Sequential production/editor Windows release
cross-builds passed after a Windows-only duplicate import was corrected in
`b034547`. That import correction did not affect the tested Linux code.

Independent review found no material issues. Tests include actual CLI subprocess
behavior, protected built-ins, draft selection rejection, strict unknown fields,
config/baseline/source isolation, existing-target preservation and concurrent
no-clobber publication. Manual Windows/filesystem behavior remains unverified.
