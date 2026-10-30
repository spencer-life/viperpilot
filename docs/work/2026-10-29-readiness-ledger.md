# Readiness ledger — 2026-10-29

A passing row is scoped to its recorded software or automated fixture evidence.
This ledger is not release approval. The production entry point remains blocked.

| Area | Evidence now available | Remaining gate / next action |
| --- | --- | --- |
| Core/offline software | Local CI passes core/editor lint, tests, native builds and help smoke; exact counts are in the dated runtime/regression notes | Review/delivery; reassess hosted checks when available rather than inheriting old green runs |
| Windows compilation | Full default/combined-preview cross-check and release builds pass | Compilation does not establish native production behavior |
| Security | actionlint, redacted history scan and zizmor pass at the current runtime/test checkpoints | Preserve existing protections; no CI/rules changes are authorized |
| Native preview | Computer Use passed the corrected Enter sequence; default/reversed/unavailable harness also passes queued Enter, roles, names, state and refusal checks | Owner physical input, screen-reader speech, actual monitor scales/high contrast and production UI remain open |
| Editor storage and fields | Headless round trips and isolated Windows Save/reopen preserve stable identity and entered fields | Full acceptance and delivery are not implied |
| Editor input/CRUD | Extra automated session passed keyboard create/save/reopen, mixed Latin/Japanese tool text entry, duplicate/rename identity preservation, dirty Cancel/Save/Discard and minimum-size validation; independent Ctrl+C/Ctrl+V and reopen | Destructive UI deletion, IME, screen reader and other scales remain unverified |
| Editor conflict/corruption | Headless session/storage tests preserve invalid/future/conflicting bytes; Windows corrupt/schema99 error/Retry tests preserve their hashes with disabled editing/Apply | External concurrent-edit desktop observation remains distinct; misleading blocked-state badge was corrected and visually rechecked in the editor-status layer |
| Production launch arrangement | Concrete proposal identifies storage, incidental startup writes, hotkey and baseline prerequisites | Not yet a launchable/approved arrangement; read production-validation-proposal before source changes or live access |
| Complete profiles/hotkey/persistence | Guarded source and synthetic tests only for this integration | Owner-present full identity, immutable baseline, exact plan, independent readback/recovery, process-exit and power-cycle evidence |
| Performance | Methodology exists; preview observations are separately scoped | Production tray-only and switching measurements after safe launch is available |
| Additional settings/custom Apply | Exact-scope read-only capability contracts and offline catalog exist | No new custom combination is approved; field/combination/recovery evidence required before promotion |
| Remote delivery | Three development branch backups verified; issue 6/8/12 progress bodies updated and dashboard memberships verified | New PR events/Actions deferred while local CI is used; implementation layers remain draft, no merge/release |
| Legacy app and hardware | Preserved throughout all recorded development/QA sessions | Continue no production tray, install, startup changes, Synapse interference or mouse writes without their explicit gates |

## Current source sequence

- Published dashboard draft #29: 11bb982.
- Rust 1.99 test repair: fix/rust199-ci at 5bc0f13.
- Runtime Enter fix and status: fix/native-enter-focus at 512bab4, retaining
  runtime commit 71a8010.
- Repeatable Enter coverage: test/native-enter-regression at d30547a.
- This separate documentation/readiness layer retains those runtime/test sources.

## Evidence handling and issue updates

The original QA reports/fixtures and the new editor session remain private.
The user confirmed the scratch-only Discard action at action time; it passed
and all four saved entries remained intact. The owned editor closed, as did
the independent clipboard and corrupt/future fixture sessions. Earlier native
harness sessions also closed cleanly. Prior reports and invalid bytes remain
preserved; the badge mismatch is recorded as a separate usability finding.

Fresh issue-sync mirrors omitted V2 Project metadata despite successful scoped
Project reads. No incomplete mirror was pushed. Only issue bodies were updated
through the API after checking their fresh remote content. A separate GraphQL
read verified all three issues still belong to ViperPilot dashboard; that live
membership was preserved in the committed local mirrors. Labels and open states
were retained. Do not force-push stale issue metadata or assume a full-sync call
alone proves Project membership.

The historical automatic publication rejection is preserved in the earlier
records. The owner subsequently authorized remote repository updates; only
non-main branch backups and measured issue progress were published. No PR
events or hosted workflows were triggered, and no protections were disabled.

The supervised-production proposal received independent read-only review with
no material findings. Source/runtime/tooling equality was checked for this
documentation layer, and its relative links and whitespace were validated.
The proposal's execution prerequisites remain open.

The editor-storage-status follow-up passed 14 headless editor tests, local
core/editor CI, strict Windows cross-lint and the release editor build.
Independent review found no material findings. A separate Windows recheck
confirmed the longer blocked-state badge fits at normal/minimum sizes,
preserves invalid/future bytes and keeps normal new-library state intact.
The dated editor-storage-status record contains its own hash and limits.
