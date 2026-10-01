# Fresh state after verification failure

Updated 2026-09-29. Earlier failure paths copied the last entry of
`values_observed` into `final_state` even when the final GET failed. That entry
could be a planning snapshot or an earlier per-field read. The report and tray
could therefore present historical data as the current mouse state.

Failure reports now retain historical observations as diagnostic evidence, while
`final_state` is populated only from a completed final read. A no-op whose live
GET fails has no final state. Rollback returns its final GET separately from the
history; failure of that GET leaves final state unknown even if prior field
reads succeeded. Fresh mismatched snapshots remain evidence and never make the
transaction successful.

The tray clears live values and disables switching when current state cannot be
verified or the fresh snapshot cannot be classified for the supported identity. A successful refresh is required to restore connected controls. No
write operation, retry policy, device support or hardware gate was expanded.
The production tray remains blocked by development isolation before effects;
these are source and synthetic-test improvements, not measured device behavior.

Regression coverage exercises failed no-op reads, disconnection during a write,
and loss of final verification after successful field reads. GUI logic is checked
headlessly. Manual Windows presentation, physical rollback and disconnect
behavior remain untested while the owner is away.

Software checks passed: core CI (128 library, 2 CLI, 3 capability, 6 catalog tests), preview CI (134 library, 2 CLI, 9 editor, 3 capability, 6 catalog tests), formatting, strict Clippy, native release and Windows MSVC production cross-build. No GUI or hardware was accessed.
