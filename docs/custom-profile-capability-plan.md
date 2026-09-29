# Custom profiles: capability and validation contract

Updated 2026-09-29. The product goal is user-authored profiles with editable DPI, button mappings, polling, and later supported device settings. Developer and Gaming are protected recovery examples, not the only acceptable configurations. A local draft library already stores semantic intent, but the user-facing editor is incomplete and its synthetic preview cannot save. Unverified settings may be stored as clearly marked local drafts; Apply stays disabled until each requested write is independently supported. Full Synapse parity and support for every recent mouse are not established requirements.

## Current boundaries

| Field | Local draft | Current device evidence | Write implication |
| --- | --- | --- | --- |
| Profile name | User editable | Local metadata only | Never a device setter |
| Wireless polling | Desired value can be stored | 1000↔4000 Hz measured on the recorded wireless Viper V4 Pro identity | Only the existing exact transition gate is proven; a whole custom profile still needs combination validation |
| DPI | Desired X/Y values can be stored | 1600→1600 proof and GET readback only | Changed values, stage editing, and ranges remain research; no general DPI Apply |
| Mouse4/Mouse5 | Semantic action intent can be stored | Native pass-through and exact Ctrl+Alt+F10/F11 assignments measured on the recorded identity | Other shortcuts/actions require per-action evidence; no raw-byte editor |
| Left/right/middle/DPI button | Future intent may be shown in UI exploration | Current profile path preserves these baseline assignments | No new setter permitted |
| Battery, idle sleep, low-power threshold | Read-only observations or future drafts | GET-only | No setter permitted |
| Other mouse models, firmware, wired transport | Intent can be described | No equivalent measured capability contract | No inherited support or write permission |

Current hardware evidence is limited to one documented wireless Viper V4 Pro device/firmware/transport combination and the specific fields listed above. A local draft, GET result, or encoder in source is never proof that a changed setter works. The unchanged 1600→1600 DPI exercise does not establish support for changed DPI values. Full serial identity is reviewed only during a supervised hardware session and kept in private evidence.

## Offline acceptance checks

1. Store drafts separately from profiles-v1.json, config-v1.json, reports, and the first immutable baseline. Missing draft data starts empty in memory without a write; corrupt or future schemas fail without reset.
2. Validate version, IDs, names, structure, and bounds before saving. Create/edit/duplicate/delete operations are transactional in memory and preserve the previous file on failure. No draft load or save calls the planner or HID layer.
3. The current data model stores one X/Y DPI target and Mouse4/Mouse5 actions. The egui preview explores additional DPI stages, buttons, and other settings with synthetic state only; its Save and Apply buttons are disabled. Persisting complete editor state and enabling user-facing Save are still required (tracked by [issue #12](https://github.com/spencer-life/viperpilot/issues/12)). Apply remains disabled until a complete device-specific write plan is permitted.
4. Tests cover independent draft round-trips, malformed and future files, unsupported actions/identities, immutable baseline isolation, and the current production profile/hotkey regressions. Windows UIA checks cover the preview controls; keyboard and screen-reader operation of a connected editor remain open. Compact-switcher accessibility, production tray, editor presentation, and production performance checks remain owner-present gates ([issues #6](https://github.com/spencer-life/viperpilot/issues/6) through [#9](https://github.com/spencer-life/viperpilot/issues/9)).

## Later supervised hardware gates

For each candidate value or action, record the full connected identity and immutable baseline; review the exact single-field plan; perform one setter experiment; independently GET the field; restore and verify the baseline before starting another. Record failed attempts and ambiguous responses. Each new DPI value, side-button action, polling transition, stage behavior, or model needs its own evidence. Changed DPI values and additional button actions remain pending owner-present experiments ([issue #10](https://github.com/spencer-life/viperpilot/issues/10) and [issue #11](https://github.com/spencer-life/viperpilot/issues/11)). After isolated fields pass, test the actual combined custom profile, rollback, process exit, and mouse power cycle. Keep startup and release gates in AGENTS.md and the README.

## Implementation relationship

- ProfileIntentV1 is a semantic, local-only model. Its field assessments describe evidence and do not produce WritePlan or HID packets. ProfileDraftLibraryV1 persists validated, unverified local intents in profile-drafts-v1.json; that library is separate from quick-switch aliases.
- The existing ProfileLibraryV1 still holds aliases of the two fixed presets. A later migration must make user-authored profiles first-class while preserving the built-ins and allowing a clean rollback.
- The egui settings preview tests layout and interaction with synthetic data. The preferred production direction remains an on-demand editor; its production process boundary and integration are not implemented or validated. Draft-only editing can use the local store without tray IPC if writes are serialized; any future Apply action needs a narrow, validated connection to the resident worker. Neither production connection exists yet.
