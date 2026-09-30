# Continue ViperPilot development

Updated 2026-09-29. This brief is the starting point for a new coding session.

## Project and source

- Public development repository: https://github.com/spencer-life/viperpilot
- Current implementation PR: https://github.com/spencer-life/viperpilot/pull/3
- Starting branch: `codex/sanitized-quick-switch`; fetch and reconcile the latest remote head before editing. The historical software checkpoint is `7cb0440a8b21f54154c1042757c810f3b9552c24`. Issue #12 adds a later local editor implementation; use the validation record below and reconcile the latest head rather than treating historical checks as validation of new code.
- PR #3 is stacked on the foundation branch `feat/quick-switch-profiles`: https://github.com/spencer-life/viperpilot/pull/1
- This is in-progress development. Keep PR #3 in draft until its documented gates pass. Do not merge, release, install the tray, enable startup, or reapply the historical ZIP as part of resuming work.

GitHub is the implementation source. A preserved original checkout or installed fixed-profile executable may be older. Verify the repository URL and branch before coding; do not assume a similarly named local folder is the current app source. The public repository is the primary development tracker; explicit issue mirrors do not make private/public repositories automatically synchronized.

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

Issue #12 now implements local create/edit/rename/duplicate/delete/save/reopen in the optional editor. Review [local editor details](local-draft-editor.md) and complete owner-observed Windows UI checks when available. The next software slice is the capability contract in issue #4; promotion in issue #13 remains hardware-gated. Keep the resident tray lightweight and the existing native architecture. Do not turn continued work into new hardware writes, extra onboard slots, broad mouse support, or a JavaScript migration.

Passing means:

- Enabled editor fields round-trip through create/edit/duplicate/rename/delete/reopen without silently discarding values; stable identities and clear validation errors are preserved.
- Draft storage remains separate from complete preset aliases, configuration and immutable baselines; corrupt/future schema behavior continues to fail explicitly.
- Clearly marked unverified drafts remain local. Apply remains disabled; draft operations do not reach HID, register input interception/injection, or change startup.
- Meaningful headless state/storage checks pass alongside the repository's relevant mise checks. Real keyboard, accessibility and Windows presentation remain separate manual gates.
- Documentation and issue progress distinguish completed software work from still-open desktop/device checks. Commit coherent validated slices and update the same draft PR.

Follow-on work remains: capability contract https://github.com/spencer-life/viperpilot/issues/4, then proven custom-profile promotion https://github.com/spencer-life/viperpilot/issues/13. Promotion also requires device field/combination evidence; implementing the contract alone does not permit Apply.

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

## Tests available now

Use the existing task entrypoints after inspecting their definitions in `mise.toml`:

```sh
mise run ci
mise run ci-preview
mise tasks validate
```

On this local environment, `--skip-tools` plus disabled auto-install was used for isolated hardware-free measurements; it does not change the tasks' commands. Windows cross-build tasks are available but do not validate Windows interaction. `install-windows`, interactive preview tasks and device commands have side effects; do not include them in an unattended check run.

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
