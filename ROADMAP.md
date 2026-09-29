# ViperPilot roadmap

> **Current direction · 2026-09-29.** ViperPilot is a lightweight native Windows utility for the Razer Viper V4 Pro. The immediate product priority is a fast tray and hotkey path with minimal idle overhead. Users should also be able to create and save their own profile drafts; device writes remain limited to the documented Viper V4 Pro field scope and its evidence gates. This roadmap is planning, not proof of hardware support.

Live work: [public GitHub issues](https://github.com/spencer-life/viperpilot/issues). The current implementation candidate is [PR 3](https://github.com/spencer-life/viperpilot/pull/3), still a draft. Its last validated source checkpoint is [`7cb0440`](https://github.com/spencer-life/viperpilot/commit/7cb0440): the [Linux and Windows Core CI run](https://github.com/spencer-life/viperpilot/actions/runs/36612610509) and [Security run](https://github.com/spencer-life/viperpilot/actions/runs/36612610273) passed for that checkpoint. Later documentation revisions have separate CI/security status. These are software checks only; no production tray or physical mouse validation is claimed. See [quick-switch status](docs/quick-switch-status.md) for the validation boundary.

Planning views: [HTML roadmap](artifacts/viperpilot-roadmap.html), [Figma design](https://www.figma.com/design/Ujs2cpFXYNnSI2YquOG1jF), [Figma development workflow](https://www.figma.com/board/7ZCW50LM1czKUmPISbxukM/Figma---Development-Workflow), and [project brain and roadmap](https://www.figma.com/board/1DhV61xcYdFD3SbtWbGBHN/ViperPilot-%25E2%2580%2594-Project-Brain---Roadmap). These are planning references; this documentation refresh does not claim that any Figma file was updated or that its designs were validated in a running application.

## Product goal

Make switching an established profile pair quick and direct from the native tray or hotkey, without settings navigation and with little background work. Preserve a compact switcher and add a detailed editor for user-owned profile drafts. Developer and Gaming are protected recovery examples, not the product's only profile choices. Full Synapse parity and support for every recent Razer mouse are not established requirements.

The product's device scope is the Razer Viper V4 Pro and the field-level capabilities documented in the current project evidence. User intent, local drafts, aliases, plans, and GET results are not onboard slots and do not prove that a hardware write worked. Keep unsupported devices, fields, transitions, and ambiguous states fail-closed. See [quick-switch status](docs/quick-switch-status.md), [custom profile capability plan](docs/custom-profile-capability-plan.md), and [supervised checklist](docs/quick-switch-supervised-checklist.md) for the current boundaries.

## Evidence key

| State | Meaning |
| --- | --- |
| **Software checked** | Hardware-free implementation checks passed for the identified candidate. This does not establish desktop usability or device behavior. |
| **Planned** | Software or product work not complete. |
| **Owner validation pending** | Requires an observed Windows desktop or physical-device session. |
| **Research** | A field or transport needs additional evidence; no write support is implied. |
| **Blocked** | Do not ship or enable until the named gate passes. |

Foundation sources compile and their software suites pass: 108 core tests plus 3 egui preview tests have been reported, alongside the candidate's local build checks. Hosted Linux and Windows Core CI plus Security also pass for PR 3. These checks do not count as mouse tests. The production tray and hotkey have not passed their owner-present manual gate, and the synthetic egui editor does not yet save user drafts.

## Current priority

The next software work follows the public issue sequence:

1. [#12 — persist profile drafts offline](https://github.com/spencer-life/viperpilot/issues/12): complete draft-only user workflows and persistence, with no device planner or HID path.
2. [#4 — define per-field capability contracts](https://github.com/spencer-life/viperpilot/issues/4): make supported values, device scope, and evidence requirements explicit before any new writable field can be exposed.
3. [#13 — add guarded promotion to the device path](https://github.com/spencer-life/viperpilot/issues/13): connect only contract-approved intents to inspectable plans and the existing guarded worker, preserving independent readback and rollback checks.

Owner-present checkpoints remain pending: [#6 compact switcher accessibility](https://github.com/spencer-life/viperpilot/issues/6), [#7 production tray validation](https://github.com/spencer-life/viperpilot/issues/7), [#8 editor presentation](https://github.com/spencer-life/viperpilot/issues/8), [#9 production performance](https://github.com/spencer-life/viperpilot/issues/9), [#10 changed DPI values](https://github.com/spencer-life/viperpilot/issues/10), and [#11 additional button actions](https://github.com/spencer-life/viperpilot/issues/11). The field experiments for DPI and buttons remain isolated, one logical field at a time, with readback and baseline restoration. Software checks do not close these gates.

## Delivery sequence

### M0 — Native quick-switch foundation · software checked, manual gates open

- Preserve the native tray, global hotkey, compact switcher, and serialized hardware worker as the fast path.
- Keep saved entries that alias the built-in presets distinct from user-authored profile drafts and from onboard device storage.
- Keep unsupported identity, unknown state, field, and polling transitions fail-closed. Do not add input interception or injection as a fallback.
- Keep startup opt-in. Do not claim tray usability, accessibility, production performance, process-exit persistence, or power-cycle persistence until its documented owner-present checks are recorded.

**Exit gate:** Hardware-free checks remain green, then the owner-present tray checkpoints #6, #7 and #9 pass with recorded Windows conditions. Complete-profile and hotkey device regressions still require their own supervised evidence.

### M1 — Offline user-authored profile drafts · next

- Let a user create, name, edit, duplicate, and delete semantic profile drafts offline. Preserve Developer and Gaming as protected recovery profiles.
- Persist validated drafts separately from complete-preset aliases, device configuration, reports, and the immutable first baseline. Preserve previous valid data on failed writes and reject corrupt or future schemas without silently resetting them.
- Keep the synthetic egui preview isolated. Its Save and Apply actions remain disabled until the actual editor and safe boundaries are implemented.

**Exit gate:** Issue #12 passes hardware-free round-trip, invalid-data, rollback, and isolation checks. Saving intent does not invoke a planner or HID operation.

### M2 — Per-field capability contracts · planned

- Define typed field support, allowed semantic values, exact device/transport scope, validation, readback expectations, and the evidence required before a value can be applied.
- Treat DPI changes and additional button actions as separate research items. Existing wireless polling transitions and particular Mouse4/Mouse5 assignments apply only to their documented Viper V4 Pro identity and conditions.
- No source encoder, unchanged-value write, saved draft, or GET response by itself establishes support for a changed value.

**Exit gate:** Issue #4 documents and tests the fail-closed contract offline. Any new hardware capability remains unavailable until its own supervised single-field evidence is recorded.

### M3 — Guarded draft promotion · planned, hardware-gated

- Add a reviewed path from a draft to an inspectable change plan only for contract-approved fields and combinations.
- Keep identity checks, exact changed-field reporting, journaling, independent GET readback, partial-failure handling, and baseline restoration requirements.
- Validate one logical field at a time against a complete immutable baseline. Review full VID/PID and serials, exact plan, and the required serial-suffix command guard before any authorized setter experiment.

**Exit gate:** Issue #13 passes offline safety tests. The production tray and hotkey still require owner-present validation, and every writable field requires its own recorded write/readback/restore evidence before release.

### M4 — Detailed editor and measured product quality · planned

- Complete an accessible user-facing draft editor and align presentation with the approved design references. The editor must not present synthetic preview state as live device state.
- Measure production cold and warm launch, idle CPU and memory, and profile switching on a documented machine; compare equivalent conditions with the official Razer app before making performance claims.
- Keep editor, renderer, and any process boundary out of the tray-only path unless measured need and architecture gates justify them.

**Exit gate:** Owner-reviewed presentation, keyboard and screen-reader behavior, high-DPI and narrow-window behavior, and reproducible performance records. The preview's current renderer measurements are not production-app or Synapse comparisons.

## Independent hardware research

The supported scope remains the documented Razer Viper V4 Pro field set. Do not broaden to other mice, firmware, transports, or arbitrary Synapse settings by analogy. Each proposed field needs its own reviewed identity, immutable baseline, exact single-field plan, independent readback, and verified restoration. Record process-exit and mouse power-cycle observations in the project status documentation only after an owner-observed test. A GET-only result or successful software build is not hardware evidence.

## Later possibilities, without a scope commitment

The earlier roadmap's expansion ideas remain possible later work, not supported
features or requirements for the next slice:

- DPI stages, more button actions, battery/power settings and sensor options each
  need their own field contract and independent evidence before a setter.
- Wired transport and other models need separate identity, transport, baseline
  and capability evidence; no support is inherited from the wireless device.
- Portable profile import/export would need schema/provenance validation and
  reversible migrations. An imported file cannot carry a trusted baseline or
  automatically authorize cross-device application.
- Distribution work would need install/update/uninstall checks, configuration
  rollback and recovery. Startup remains opt-in; diagnostics must stay sanitized.

## Release checklist

1. Hardware-free formatting, lint, test, build, and security checks pass for the candidate being reviewed.
2. Owner-present desktop gates pass for the shipped production tray/compact switcher and any included editor, accessibility, or performance claims. The optional preview's separate editor gate does not establish tray support.
3. Every enabled setter has field-specific evidence for the documented Viper V4 Pro scope, including independent readback and baseline restoration.
4. Complete built-in profiles and the production hotkey pass their supervised regression; process-exit and power-cycle behavior are recorded before any persistence claim.
5. Public documentation distinguishes software checks, plans, measured device behavior, and open gates. Preserve private identifiers and raw diagnostic material in private evidence.
