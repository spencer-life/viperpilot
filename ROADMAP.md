# ViperPilot roadmap

> **Product name and scope, 2026-09-29.** ViperPilot is the project name for this independent Viper V4 Pro utility. Existing executable and Cargo package identifiers may remain for compatibility. This roadmap separates product work from hardware research because most requested configurability is not yet proven safe to write.

Live work: [public GitHub Issues](https://github.com/spencer-life/viperpilot/issues).

Planning views: [HTML roadmap](artifacts/viperpilot-roadmap.html). The Figma design source is not mirrored in this public repository. The [egui evaluation](docs/egui-evaluation.md) records the UI options and validation gates.

**Design direction selected 2026-09-28:** Start with a compact native quick switcher and direct tray profile actions, using a dark rose/charcoal palette and a pink V tray mark. The visual direction is a design reference; example values and future controls are proposals, not measured hardware support or completed UI behavior. The detailed dashboard follows the quick-switch flow.

## Product goal

Build a lightweight native Windows control app for supported Viper devices. The quick-switch path should make current state and profile selection easy to understand, with editable controls added only when they have a safe device-specific contract. Developer and Gaming are protected recovery examples, not the product's complete profile model. Users should eventually create profiles with their own DPI, button, polling, and other settings. Unverified values may be saved as clearly marked drafts, but drafts do not authorize device writes. Compare startup, idle cost, and switching workflow against the official Razer app under the same conditions before making speed or resource claims.

## Evidence key

| State | Meaning |
| --- | --- |
| **Shipped / measured** | Implemented, with reproducible conditions and results in a sanitized status document. Keep serials and raw reports in private local evidence. |
| **Planned** | Product or engineering work. No new hardware support is implied. |
| **Research** | A control or transport that needs protocol evidence and isolated hardware tests. |
| **Blocked** | Cannot ship until the stated safety or validation gate passes. |

The measured scope is limited to one wireless Viper V4 Pro device/firmware/transport combination. See [quick-switch status](docs/quick-switch-status.md) for current software validation and open gates. Do not infer support for other models, transports, fields, or assignments from source similarity.

## Delivery sequence

The milestones are ordered by dependency, not calendar date. Each closes only when its exit criteria and relevant release gates pass. A milestone may ship without later research tracks.

### M0 — Preserve the measured core · shipped / measured

- Keep the protected Developer and Gaming recovery examples available. Preserve the first immutable baseline and raw diagnostic evidence locally; do not commit full serials or private reports.
- Keep unsupported device identities, unknown raw states, and unmeasured polling transitions fail-closed. The wired path stays a recovery investigation, not a general configuration path.
- Keep the native app single-instance and idle without polling. Do not add a browser runtime, background service, driver, input interception, or injection without a separate design review and measured need.

**Exit evidence:** Current support boundaries and software validation remain traceable to sanitized project docs; package checks and cross-builds remain green.

### M1 — Define the configuration and write contract · next / planned

- Define a typed allowlist, ranges, and device/transport support for each editable field. Distinguish profile intent from raw protocol bytes and reject unsupported combinations before any write plan is created.
- Version the local config schema. Specify migration from `UtilityConfigV1`, atomic save, corrupt-file handling, stale data, and recovery without touching the immutable hardware baseline. Keep the original config as a rollback copy during migration.
- Make the proposed write plan inspectable: device identity, exact changed fields, packet intents, expected readback, recovery path, and journal location. A preview is not proof of a write.
- Add offline fixtures for valid, malformed, stale, and cross-device data; plan and rollback tests must prove no unsupported setter can be reached.

**Exit gate:** Schema migration is reversible; unsupported input fails before planning; the automated protocol, config, plan, and rollback suite passes with no hardware connected.

### M2 — Editable profiles for proven fields · planned, hardware-gated

- Let users create, name, duplicate, edit, select, and delete their own profiles. Keep built-in profiles read-only and clearly marked as measured presets.
- Expose only values and combinations supported by the per-field contract from M1. Do not turn Developer/Gaming examples into fixed user choices. A field stays fixed, draft-only, or unavailable until its changed values and relevant combinations have independent evidence. A readback of an unchanged value does not prove a changed target or range.
- Show a complete change preview before applying. Journal the operation, verify each changed logical field with an independent GET, and report partial failure and recovery steps without declaring success prematurely.
- Perform one logical-field hardware experiment at a time: immutable baseline, full VID/PID and serial review, exact plan, suffix command guard, independent readback, then baseline restoration before the next field.

**Exit gate:** Each exposed value or supported range, and each material cross-field combination, has source/protocol evidence plus a changed-value write, independent readback, and baseline restoration for the supported wireless identity. Built-in and edited profiles pass complete-profile switching, interrupted-write recovery, process-exit, and power-cycle tests. A field without this evidence remains fixed, read-only, or unavailable.

### M3 — Fast daily desktop workflow · planned

- Design a focused dashboard for identity, connection, battery when available, current DPI/polling/profile, and clear stale or unknown states. Keep the main profile action reachable without navigating through device settings.
- Add profile list/editor, explicit unsaved-change behavior, accessible keyboard flow and focus states, readable errors, and a guided recovery view. Document interaction states in Figma before implementation, then capture the running UI back for visual QA.
- Measure cold and warm launch, idle CPU and memory, profile-switch elapsed time, and clicks/steps to switch. Record machine, OS, firmware, connection mode, Synapse state, sample count, and method. Compare the same workflow against the official Razer app before publishing any superiority claim. **Unproven target:** at least 2× faster median cold launch to an actionable UI and fewer user actions to switch an existing profile; report device commit latency separately.
- Keep package size and steady-state work visible in release review. Prefer native controls and event-driven updates unless measurements justify more.

**Exit gate:** Keyboard and screen-reader review, high-DPI and narrow-window review, no-polling idle check, benchmark record against the target, and visual QA pass. No speed claim ships without reproducible comparative evidence.

### M4 — Portable profile library and distribution · planned

- Define which profile fields are portable and which are bound to the measured device identity. Import/export must validate schema, ranges, supported transport, and provenance before offering a plan. Imported files never carry a trusted baseline or authorize a hardware write.
- Add clear per-device profile association only after identity semantics are stable. Test malformed, stale, future-version, and cross-device files offline.
- Review installer/update/uninstall behavior, config rollback, diagnostics export, and a path back to the last verified profile. Keep startup opt-in until the production app passes its own startup and exit gates.

**Exit gate:** Round-trip import/export, migration and downgrade/recovery fixtures, clean install/update/uninstall, and documented rollback pass. No automatic cross-device application is allowed.

## Research tracks — independent of delivery milestones

These are candidates, not promised controls. A GET result or source inspection alone does not enable a setter. Record failed experiments and ruled-out explanations as carefully as successful runs.

| Candidate | Current status | Required evidence before UI write support |
| --- | --- | --- |
| DPI stage count and stage values | Research | Exact encoding and supported ranges; one field at a time with readback and baseline restoration. |
| Additional button assignments | Research | Per-action raw mapping, OS behavior, persistence, and rollback; no arbitrary raw editor. |
| Battery, idle sleep, low-power threshold | GET-only / research | Setter protocol, supported range and persistence; current optional reads are not setter evidence. |
| Sensor options | Research | Identify real fields and firmware behavior before designing controls. |
| Wired configuration or 4000 Hz | Blocked | Separate wired identity, transport, and polling transition evidence. |
| Other mouse models | Blocked | A per-model capability matrix and independent baselines/tests; no support inherited by similarity. |

For every proposed control, record the date, reviewed device/firmware/transport identity, starting state, source hypothesis, GET-only discovery, exact single-field plan, independent readback, restore result, and persistence checks where relevant. Store full serials and raw reports in private local evidence; do not publish them. Stop on ambiguous identity or state. Treat every setter as a possible flash write.

## Release decision checklist

1. **Offline correctness:** Protocol, config, migration, validation, plan, rollback, and UI-logic checks pass using the repository `mise.toml` tasks and pinned Windows cross-build tools.
2. **Hardware safety:** Every newly writable field has a complete immutable baseline, reviewed full VID/PID and serials, exact plan, one-field experiment, independent readback, and verified baseline restoration. No hardware `apply` or `restore` runs from a planning or design task.
3. **Regression:** Both complete built-in profiles, the production hotkey, process exit, and mouse power-cycle pass after relevant app, transport, or packaging changes. Tray startup remains off until its documented gates pass.
4. **Product quality:** Accessibility, visual QA, installation rollback, and measured performance meet the milestone's stated criteria.
5. **Claims:** Public docs list only support claims that can be reviewed without publishing personal device identifiers or raw diagnostics. Roadmap candidates remain labelled planned, research, or blocked until their gates pass.
