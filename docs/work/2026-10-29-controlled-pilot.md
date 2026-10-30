# Controlled portable pilot — 2026-10-29

The owner prioritized a downloadable Windows candidate for the established
Developer/Gaming pair. This source preparation does not authorize or claim live
mouse validation. Ordinary production launch and installation remain blocked.

`--pilot --root DIRECTORY --confirm-device FULL_SERIAL` is explicit and strictly
parsed. The default is device read-only. The optional established-profile flag
is a final launch guard after owner identity, baseline, exact-plan and recovery
review; it does not generate that evidence. The candidate never registers a
global hotkey and rejects/hides startup changes, including in write-enabled
sessions. An existing development instance is rejected rather than silently
reusing another session's storage or permissions.

The worker receives the selected storage root. Legacy directory components and
existing storage junction/symlink escapes are rejected. No baseline is created
or imported. The reviewed baseline's complete identity and original bytes are
bound for the session and checked on every command. Live identity is checked
before baseline lookup; all apply aliases and recovery routes converge on a
second authorization gate before journaling or setters. Existing polling,
preflight, serialized-write, independent-readback and rollback restrictions stay
in force. A different path, transport, firmware or descriptor serial needs fresh
review; there is no generic hardware-support expansion.

## Software evidence

Native Windows Rust/Cargo 1.99.1, locked dependencies:

- `ci-windows`: formatting, Windows Clippy, 150 library + 2 CLI + 3 capability +
  6 catalog tests, release build and GET-free CLI help passed.
- `ci-windows-preview`: combined-feature Clippy, 156 library + 2 CLI + 14 editor +
  3 capability + 6 catalog tests, native preview release build passed.
- New tests cover strict arguments, missing/full-serial baseline selection,
  read-only denial of every worker write route, startup denial even when profile
  writes are enabled, exact identity, complete baseline and original bytes,
  containment, and a real temporary NTFS junction escape.
- PowerShell packaging/launcher syntax and all 29 locally selected mise tasks
  validated. Packaging compiles binaries without executing them.
- Independent source review found two issues (existing-instance option reuse
  and lookup before identity validation); both were fixed and reviewed again
  with no remaining material findings.

WSL command execution repeatedly timed out. A native local clone of the exact
published checkpoint was used instead; WSL was not restarted and its original
checkouts were preserved. The restricted Windows filesystem sandbox denied ten
atomic-file tests; the authorized hardware-free run outside that sandbox passed.
An intermediate CI run overlapped source editing and failed its final compile;
the completed source received the full successful runs above. No failed run is
counted as passing evidence.

The compact switcher/dashboard previously passed isolated Windows preview QA.
The real notification-area icon/menu, live profile transitions, production
hotkey, process-exit/power-cycle persistence and performance remain unmeasured
for this candidate. The new pilot must not be presented as a validated daily
replacement. See [candidate instructions](../pilot-candidate.md).

## Distribution and rollback

Publish reviewed non-main source branches using the GitHub plugin and stage a
portable ZIP with source commit, build metadata and SHA256 manifests. The owner
requested GitHub download; keep it in a **draft** release while live gates remain
open. No PR/hosted Actions, merge, public release, install, startup change,
legacy-app interruption or mouse write is included. Package data stays separate;
the installed app remains available. Any later device restoration is itself a
guarded write requiring review and independent verification.
