# Continue ViperPilot development

Updated 2026-09-30. This brief is the starting point for a new coding session.

## Project and source

- Public development repository: https://github.com/spencer-life/viperpilot
- Catalog PR: https://github.com/spencer-life/viperpilot/pull/3; the focused implementation stack is listed below.
- Choose the owning branch from the table below; the current integration top is `codex/manual-validation-handoff` (documentation); UI/accessibility source remains on `codex/native-button-accessibility`. Fetch and reconcile the latest remote head before editing. The historical software checkpoint is `7cb0440a8b21f54154c1042757c810f3b9552c24`. Issue #12 adds a later local editor implementation; use the validation record below and reconcile the latest head rather than treating historical checks as validation of new code.
- PR #3 now targets `codex/profile-capabilities` (PR #16). The stack ultimately targets `main` through the original foundation PR #1.
- This is in-progress development. Keep PR #3 in draft until its documented gates pass. Do not merge, release, install the tray, enable startup, or reapply the historical ZIP as part of resuming work.

GitHub is the implementation source. A preserved original checkout or installed fixed-profile executable may be older. Verify the repository URL and branch before coding; do not assume a similarly named local folder is the current app source. The public repository is the primary development tracker; explicit issue mirrors do not make private/public repositories automatically synchronized.

## Focused review stack — 2026-09-29

The owner requested smaller PRs. Existing feature checkpoints were published as
prefix branches and linked with GitHub's official `gh-stack` extension. The
split preserves all existing commits and the runtime source tree; only branch
boundaries, PR bases, descriptions and this handoff were reorganized. No merge,
release or hardware operation occurred. PR #3 and all new implementation PRs remain draft.

| Order | PR | Branch | Scope / split checkpoint |
| --- | --- | --- | --- |
| 1 | [#1](https://github.com/spencer-life/viperpilot/pull/1) | `feat/quick-switch-profiles` | Existing saved-profile foundations; `a299997` |
| 2 | [#14](https://github.com/spencer-life/viperpilot/pull/14) | `codex/guarded-switch-foundation` | Sanitized guarded native switching, preview/model foundations and tracking docs; `462636f` |
| 3 | [#15](https://github.com/spencer-life/viperpilot/pull/15) | `codex/local-profile-drafts` | Persisted editable draft CRUD, issue #12; `6406e17` |
| 4 | [#16](https://github.com/spencer-life/viperpilot/pull/16) | `codex/profile-capabilities` | Read-only capability contract/evidence, issue #4, plus shared Core check-name alignment; `aa17779` |
| 5 | [#3](https://github.com/spencer-life/viperpilot/pull/3) | `codex/sanitized-quick-switch` | Offline catalog preview/export, issue #13 software slice, and continuation/gate documentation; runtime checkpoint `4ed29f1` |
| 6 | [#18](https://github.com/spencer-life/viperpilot/pull/18) | `codex/development-isolation` | Separate default data and block installer/tray effects before touching the legacy app; `25ab4fb` |
| 7 | [Current-state verification](https://github.com/spencer-life/viperpilot/tree/codex/current-state-verification) | `codex/current-state-verification` | Fresh final-read reporting and unavailable/wrong-device UI after failed verification; separate draft safety follow-up |
| 8 | [#21](https://github.com/spencer-life/viperpilot/pull/21) | `codex/development-identities` | Separate Windows identities and corrected isolated manual checklist; `e84f21e` |
| 9 | [UI readiness](https://github.com/spencer-life/viperpilot/tree/codex/ui-readiness) | `codex/ui-readiness` | Fail-closed generic worker errors, accessible editor controls and manual handoff |
| 10 | [#23](https://github.com/spencer-life/viperpilot/pull/23) | `codex/native-button-accessibility` | Standard Windows buttons, semantic hotspot names and strict manual UIA checks |
| 11 | [Development checks](https://github.com/spencer-life/viperpilot/tree/codex/development-check-tasks) | `codex/development-check-tasks` | Linux core/editor aggregate and sequential Windows cross-lint/build tasks |
| 12 | [Manual handoff](https://github.com/spencer-life/viperpilot/tree/codex/manual-validation-handoff) | `codex/manual-validation-handoff` | Audited preview/tray test boundaries, current hosted checks and Cloud access diagnostic handoff |

Official [stack #17](https://github.com/spencer-life/viperpilot/pull/3) is
registered on GitHub. The first sanitized import is still broad; its original
single sync commit is preserved rather than claiming every layer is small.
Editor, capability and catalog reviewers now see only their own layer's diff.
Renovate PR #2 is independent and is not part of this feature stack.

Use the layer table to choose the owning branch before editing. A new independent
concern should start a separate PR; a dependent concern should become a new layer.
Keep the immutable baseline and existing Apply, tray-release and manual validation
gates. Review/delivery can progress by layer; closing an issue still requires its
actual acceptance criteria, including owner-observed evidence where specified.

The extension and official agent skill are pinned to `v0.1.1` on the current
machine. A fresh environment can install them with:

```sh
gh extension install github/gh-stack --pin v0.1.1
gh skill install github/gh-stack --agent codex --scope user --pin v0.1.1
```

[Official skill and command guidance](https://github.com/github/gh-stack/tree/v0.1.1/skills/gh-stack)
describe safe non-interactive flags. The current registration used
`gh stack link` with existing PR URLs, without local stack tracking, rebasing,
force pushes or marking drafts ready. Local navigation requires a clean checkout
and adoption with `gh stack checkout` first. Do not run stack merge during
continuation. Rollback of this metadata-only split is to dissolve the stack,
restore PR #3's original base `feat/quick-switch-profiles` and its saved description,
and close the added PRs; retain branch commits until cleanup is authorized.

Hosted results below remain checkpoint-specific. Newly created PRs and the
changed PR #3 base have their own checks; do not inherit a green status solely
because a head commit or runtime source was previously tested.

## What the owner wants

Fast, direct switching through a lightweight native Windows tray or hotkey, with little background work and no browser/settings navigation for the established pair. The compact switcher opens first; detailed editing is on demand. Users should be able to author their own DPI, mappings, polling and other settings as support is proven. Developer/Gaming remain protected examples/recovery presets, not the only eventual choices.

Full Synapse parity and every recent mouse model are not requirements. Current support is limited to the documented Viper V4 Pro identities, fields and transitions. Unverified settings can be clearly marked local drafts; Apply stays disabled until evidence permits it. Local drafts and named profiles are not extra onboard slots.

## Current implementation

| Area | Present | Remaining |
| --- | --- | --- |
| Saved quick-switch pair | Validated local aliases, protected built-ins, selected pair, guarded tray/hotkey source | Production owner-observed switching and performance evidence |
| Storage/write guards | Corrupt/future files fail explicitly; baselines stay separate; stale/uncertain writes stop and use independent readback | No new device support or values are implied |
| Native UI | Compact/Details source and isolated Win32 preview | Actual keyboard/screen-reader/scaling checks |
| Customization | Versioned local draft model/storage and optional user-facing CRUD editor, separate from complete preset aliases | Owner-observed Windows UI; Apply stays disabled |
| Editor scope | Enabled controls round-trip name, X/Y DPI, polling and semantic Mouse4/Mouse5 intents | Stages/later settings are explicitly unsavable in V1 |
| CI/tooling | Cargo/rustup/mise; hosted Linux/Windows/security checks passed at the recorded checkpoint | Required-check protection and stacked merge order remain open |

The resident production path is native Win32. Egui/eframe 0.36.2 is a separate optional editor preview, excluded from default production builds. Cargo owns Rust dependencies; Aube was assessed and not adopted. Its Rust embedding guide was reviewed, but the embedded library was not compiled, linked or benchmarked. No Node frontend/runtime requirement was added.

## Next code assignment

Issue #12 now implements local create/edit/rename/duplicate/delete/save/reopen in the optional editor. Review [local editor details](local-draft-editor.md) and complete owner-observed Windows UI checks when available. Issue #4 now implements the [read-only capability contract](capability-contract.md); review and delivery remain. Issue #13 now adds [offline catalog preview/export](profile-catalog.md); production promotion remains hardware-gated. Keep the resident tray lightweight and the existing native architecture. Do not turn continued work into new hardware writes, extra onboard slots, broad mouse support, or a JavaScript migration.

Passing means:

- Enabled editor fields round-trip through create/edit/duplicate/rename/delete/reopen without silently discarding values; stable identities and clear validation errors are preserved.
- Draft storage remains separate from complete preset aliases, configuration and immutable baselines; corrupt/future schema behavior continues to fail explicitly.
- Clearly marked unverified drafts remain local. Apply remains disabled; draft operations do not reach HID, register input interception/injection, or change startup.
- Meaningful headless state/storage checks pass alongside the repository's relevant mise checks. Real keyboard, accessibility and Windows presentation remain separate manual gates.
- Documentation and issue progress distinguish completed software work from still-open desktop/device checks. Commit coherent validated slices and update the same draft PR.

Follow-on work is the hardware-gated remainder of https://github.com/spencer-life/viperpilot/issues/13. Opt-in catalog export preserves current production files and cannot admit a custom profile. Read the contract first: caller-supplied snapshots are not proof of a fresh GET, immutable baseline or full serial review. No custom combination is registered as approved. Do not connect drafts to setters or enable Apply until field, combination, readback and recovery evidence permits it. Other model owners or contributed supervised evidence are needed for other mice; the V4 Pro alone cannot validate them.

## Issue #12 software checkpoint — 2026-09-29

Source checkpoint [`6792c6f`](https://github.com/spencer-life/viperpilot/commit/6792c6fe8bc3774ab133a0b830fac8796b421e9d)
adds the persisted local editor, following session commit `f2273e5`.

- `mise run ci` passed formatting, strict default Clippy, **108 core tests**,
  native release build and CLI help smoke.
- `mise run ci-preview` passed strict feature-enabled Clippy, **114 library
  tests** (including six session tests) and **7 headless editor tests**.
  The editor tests include actual text editing and Save, disk reopen, disabled
  Apply, preservation of loaded values, modal isolation, dirty reload, failed
  save feedback and confirmed close state.
- `mise run build-windows` and `mise run build-egui-preview-windows` passed
  without launching either executable. Final `fmt-check`, whitespace and
  task validation passed (25 repository tasks; 30 including inherited tasks).
- Default normal dependencies exclude egui/eframe, and the editor session is
  feature-gated. Native tray, worker, planner, device support and Cargo
  dependencies are unchanged. Independent review found no remaining material
  findings after correcting confirmation isolation and dialog error feedback.

These commands used process-scoped mise trust and disabled automatic tool
installation. Concurrent xwin setup initially failed at a shared clang-cl
symlink; sequential cross-builds succeeded. Run the cross-builds sequentially.
The updated PowerShell scripts received static review only; no PowerShell
parser was available. Hosted checks for this new head must be checked separately
from historical green runs. No manual Windows UI, production tray/hotkey,
installation/startup, performance or physical-device test was run. PR #3 remains
draft, and issue #12 remains open pending delivery and owner-observed UI checks.

## Issue #4 software checkpoint — 2026-09-29

Source checkpoint `28b36f2` adds the editor's offline evidence view after the
read-only contract in `d28661c`. See [contract details](capability-contract.md).

- `mise run ci` passed formatting, strict Clippy, **114 core tests + 3 integration
  tests**, native release build and CLI help smoke.
- `mise run ci-preview` passed strict feature Clippy, **120 library tests,
  9 headless editor tests and 3 integration tests**.
- Sequential `build-windows` and `build-egui-preview-windows` passed without
  launching either executable. Default normal dependencies still exclude
  egui/eframe; no Cargo dependency or production planner/engine/tray change.
- Independent review found and fixed silently ignored unknown fields in the
  pass-through/unassigned JSON actions. All four action variants now reject
  extras without changing valid V1 serialization. Final review found no remaining
  material issues. All eight scope axes reject inherited evidence; timestamp,
  GET-only power and collection-order changes do not falsely mark state stale.
- Documentation links, whitespace and all 30 available mise tasks validated.

Checks used process-scoped mise trust with automatic installation disabled.
Supplied snapshots do not prove live GET or baseline immutability. Every custom
request remains rejected at the combination gate; no WritePlan or setter is
created. No manual Windows, tray/hotkey, performance, startup or mouse test was
run. PR #3 remains draft; #4 stays open for review/delivery and #13 remains gated.
Hosted checks for the published head must be checked separately.

## Hosted checkpoint before the contract slice

At head `6406e17df7a8c86b175613515b711d477379af10`, hosted
[Linux/Windows Core CI](https://github.com/spencer-life/viperpilot/actions/runs/36648753194)
and [Security](https://github.com/spencer-life/viperpilot/actions/runs/36648753154)
passed. Those results validate the prior editor checkpoint, not subsequent
capability code. Check the actual latest head separately.

## Issue #13 offline software checkpoint — 2026-09-29

Catalog source `26da843` adds frozen projection, preview and no-clobber export;
`b034547` corrects a Windows-only duplicate path import. See [catalog and rollback](profile-catalog.md).

- Core CI passed **122 library, 2 CLI, 3 capability integration and 6 catalog
  integration tests**, formatting, strict Clippy, native release and CLI help.
- Preview CI passed **128 library, 2 CLI, 9 editor, 3 capability integration
  and 6 catalog integration tests**, with strict feature Clippy.
- Sequential production and editor Windows MSVC release cross-builds passed
  after the import correction. The initial production build failed with E0252;
  this was a source import conflict, not protocol or mouse evidence. Linux code
  was unaffected by the cfg-only correction, so its suites were not repeated.
- Independent review found no material implementation findings. Distinct tests
  cover actual CLI preview/export, existing-output preservation, source/config/
  baseline isolation, strict catalog variants, protected built-ins, draft pair
  rejection and concurrent one-winner publication. Default dependencies still
  exclude egui/eframe.
- Workflow actionlint and security analysis passed. CI contexts are aligned
  with PR #1; see [CI and merge gates](ci-and-merge-gates.md). Required-check
  activation remains pending explicit authorization after automatic review rejection.

No automatic migration or production reader changed. No custom combination is
approved; Apply remains disabled. No desktop, startup or mouse operation was run.
Issues #6–#11 retain their owner-present gates; #13's production promotion remains
blocked by device/combination/recovery evidence. Hosted checks for later heads
must be assessed separately.

## Software-only preparation and remaining gates

[CI trigger/check names and the stacked merge policy](ci-and-merge-gates.md) are
documented. Required-check enforcement is prepared but not enabled: automatic
approval review requires explicit settings authorization. Keep PR #3 draft and
do not merge PR #1 as part of this continuation. [Performance methodology](performance-validation.md)
defines repeatable samples for a later owner-present session; no production
measurement was run. Desktop issues #6/#8, tray/device issue #7, performance #9
and field experiments #10/#11 retain their original manual gates.

## Tests available now

Use the existing task entrypoints after inspecting their definitions in `mise.toml`:

```sh
mise run verify-linux
mise run verify-windows-cross
mise tasks validate
```

See [development commands](tooling-decision.md#development-commands) for formatting,
quick checks, Windows host checks and security scanning. The 2026-09-30 aggregates
reuse existing tasks sequentially and keep GUI/install/device actions separate.
On this local environment, `--skip-tools` plus disabled auto-install was used for
isolated hardware-free measurements; it does not change the tasks' commands.
Nested `mise exec` also needs `MISE_EXEC_AUTO_INSTALL=false` when tools are already
prepared. Cross-build tasks do not validate Windows interaction. `install-windows`,
interactive preview tasks and device commands have side effects; do not include
them in an unattended check run.

At checkpoint `7cb0440`, fresh Linux checks passed 108 core and three egui tests plus formatting, lint and native build. Hosted checks passed:

- https://github.com/spencer-life/viperpilot/actions/runs/36612610509/job/109557553770
- https://github.com/spencer-life/viperpilot/actions/runs/36612610509/job/109557553566
- https://github.com/spencer-life/viperpilot/actions/runs/36612610273/job/109557552100

These results are software evidence, not mouse or desktop validation. Check later PR heads rather than assuming these URLs validate a subsequent code change.

## Deferred manual work

The owner is away from the PC. Wait for an observed session for:

| Public issue | Gate |
| --- | --- |
| https://github.com/spencer-life/viperpilot/issues/6 | Compact native UI accessibility, keyboard and display scaling |
| https://github.com/spencer-life/viperpilot/issues/7 | Production tray/hotkey, independent readback/recovery and persistence |
| https://github.com/spencer-life/viperpilot/issues/8 | Actual editor presentation, input and accessibility; white desktop capture versus renderer remains unresolved |
| https://github.com/spencer-life/viperpilot/issues/9 | Production tray overhead and switching latency |
| https://github.com/spencer-life/viperpilot/issues/10 | Changed DPI values, one logical field per experiment |
| https://github.com/spencer-life/viperpilot/issues/11 | Additional button actions, one logical field per experiment |

Every hardware write still requires immutable baseline, full identity and exact-plan review, the serial-suffix command guard, independent readback and baseline restoration. Never kill Synapse or add input injection/interception. Read `AGENTS.md` and `docs/quick-switch-supervised-checklist.md` before any observed test. Do not claim release readiness from a build or preview.

## Important files and design references

- `AGENTS.md`: working rules and hardware safety.
- `docs/quick-switch-status.md`: recorded evidence and remaining gates.
- `docs/custom-profile-capability-plan.md`: semantic intent versus proven write support.
- `docs/egui-evaluation.md`: optional editor decision and open Windows checks.
- `docs/tooling-decision.md`: Cargo/mise and Aube assessment limits.
- `.issues/README.md`, `.issues/open/`: issue-sync workflow and current backlog; pull before editing/pushing. Preserve `codex-created`; it describes authorship, not completion.
- `src/profile_intent.rs`, `src/storage.rs`, `src/profile_library.rs`: current local data/storage foundations.
- `src/draft_editor.rs`: feature-gated persisted editing session; no device path.
- `docs/local-draft-editor.md`: workflows, storage conflicts, recovery and deferred checks.
- `src/tray.rs`, `src/tray_logic.rs`, `src/engine.rs`, `src/planning.rs`: production switching and guarded work.
- `src/bin/viperpilot-egui-preview.rs`, `src/bin/viperpilot-ui-preview.rs`: isolated previews.
- `mise.toml`, `rust-toolchain.toml`, `Cargo.toml`, `Cargo.lock`, `.github/workflows/`: actual check/tool/dependency configuration.
- Approved dark compact-first app direction and pink V: https://www.figma.com/design/Ujs2cpFXYNnSI2YquOG1jF
- Established Figma workflow: https://www.figma.com/board/7ZCW50LM1czKUmPISbxukM/Figma---Development-Workflow
- Planning board: https://www.figma.com/board/1DhV61xcYdFD3SbtWbGBHN/ViperPilot-%25E2%2580%2594-Project-Brain---Roadmap

Figma/HTML roadmaps are planning and design references, not implementation or validation evidence. This documentation refresh does not claim that the Figma boards were changed.

## Installed-app preservation and CI proposal — 2026-09-29

Draft PR #18 adds [development isolation](development-isolation.md). The installed legacy app remains untouched. Default data now uses `ViperPilotDevelopment`; installer and production tray entry points reject before effects. Both core/preview checks and Windows production/editor cross-builds passed without launching. Installation or concurrent production use still requires separate executable/startup/hotkey identity and reviewed manual gates.

Independent draft [PR #19](https://github.com/spencer-life/viperpilot/pull/19) supplies the concrete staged CI proposal for issue #5. It preserves historical check names: A for #1, B after #1 lands for #14/#15, C after both land for #16 and later. No settings changed. Each activation needs explicit authorization, captured rollback and fresh reconciled checks; no merge is authorized.

The current-state safety layer above #18 fixes historical observations being presented as current after failed final reads. See [fresh verification](current-state-verification.md). Core checks passed 128 library + 2 CLI + 3 capability + 6 catalog tests; preview checks passed 134 library + 2 CLI + 9 editor + 3 capability + 6 catalog tests. Strict lint, formatting, native release and production Windows cross-build passed without launching. These checks do not satisfy manual device gates.

## Owner-away software review — 2026-09-30

Read [the software/manual handoff](manual-handoff.md) for completed work, exact validation and all remaining gates. The UI audit findings are fixed and independently reviewed; remaining desktop/device checks are unobserved. Keep all new layers draft, custom Apply gated, production tray/installer blocked and the installed legacy app untouched. No merge, settings activation or cloud credential setup occurred.

The final native accessibility layer prepares standard Windows button semantics for the recorded owner-draw UIA defect. Read the manual handoff and retain the old failed result; do not call the new Windows behavior measured until the isolated preview test and owner observations pass.
