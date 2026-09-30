# egui evaluation for ViperPilot

Updated 2026-09-29. This is an architecture decision record for a hardware-free UI experiment, not evidence of new device support or completed product UI. The immediate priority is a fast native tray/hotkey path with low idle cost. User-authored profile drafts are also a product goal. Developer and Gaming remain protected recovery examples; local aliases of them are not custom profiles.

## Decision for the current slice

Use egui 0.36.2 through a feature-gated, isolated eframe preview executable. Select Glow, AccessKit, and bundled default fonts explicitly, with eframe default features off. The editor now reads/writes only the local draft store. It never opens HID, alias/config storage, startup, the production tray, or the production mutex. Keep the existing Win32 tray, hotkey, compact switcher, and serialized hardware worker as the resident path.

The optional editor runs as a separate on-demand process, keeping its renderer out of the default tray build. Tray launch/lifecycle, IPC and production integration have not been implemented or validated. Keep the existing native tray/hotkey path as the resident product priority; validate the preferred direction against the gates below.

## Options considered

| Option | Fit for this app | Cost or uncertainty | Decision |
| --- | --- | --- | --- |
| Keep all visible UI in owner-drawn Win32 | Small existing binary and one message loop; current tray and hotkey already work | Detailed editors require more bespoke layout, controls, and accessibility work; current owner-drawn buttons expose UIA Pane without InvokePattern | Retain for quick switching |
| Make eframe the entire resident app | One UI framework for all screens; standard widgets and AccessKit support | Reworks the Win32 message-loop/tray/hotkey ownership and loads the renderer during tray-only use | Do not migrate the resident shell in this slice |
| On-demand eframe settings process | Rich editor while resident shell stays small; failure and resource cost may be isolated | Requires authenticated/local IPC, lifecycle rules, and measurement | Preferred production direction after preview gates |
| Hand-integrate egui into the current Win32 pump | Could keep one process/window owner | Must translate input, execute frames, handle output, and provide a renderer; upstream shows custom `egui_glow`/winit integration, but no drop-in adapter for this raw Win32 pump | Not selected |
| Browser/WebView editor | Familiar layout and Figma translation | Adds a browser runtime to a task whose priority is low overhead and native access | Not selected |

## Feature choices and facts

- egui is the immediate-mode widget and painting library; eframe supplies a native window, event handling, and renderer. The current eframe version is 0.36.2. Its defaults include WGPU and several cross-platform features. Glow is an officially supported renderer, and upstream says it can materially reduce binary size relative to WGPU. This is a reason to test Glow for a 2D editor, not a measured ViperPilot result.
- AccessKit is available on native Windows. Common egui widgets publish roles and names; a custom-painted mouse diagram must provide WidgetInfo and keyboard behavior deliberately. egui_kittest can test the semantic tree without opening the production app. Windows UI Automation and screen-reader behavior still need direct review.
- The current egui workspace declares Rust 1.95 minimum. ViperPilot now tracks the latest stable Rust channel and records 1.98.1 as its tested minimum. No new renderer is included in a normal production build until the preview experiment passes.
- Immediate-mode layout runs when a frame is requested. Upstream says egui avoids idle repaints, but that does not establish this app's tray idle CPU, memory, startup time, or GPU cost. These must be measured in the actual process arrangement. `NativeOptions::run_and_return` defaults to true, which supports a closing on-demand editor loop; it does not supply tray lifecycle or IPC behavior.
- egui is MIT OR Apache-2.0. ViperPilot is GPL-2.0-only; the MIT option is the relevant permissive license path for a distributed build. Dependency/license review remains part of release work.

## Relevant feature audit

| Capability | Preview choice | Reason or check |
| --- | --- | --- |
| Buttons, text edits, numeric inputs, selectors, scrolling | Use standard egui widgets | Gives the custom editor ordinary focus and semantic behavior instead of another owner-drawn control set. |
| Dark styling, icons, mouse diagram | Use egui style and painter sparingly | Match the dark rose/charcoal planning direction in the [Figma design](https://www.figma.com/design/Ujs2cpFXYNnSI2YquOG1jF). The linked [development workflow](https://www.figma.com/board/7ZCW50LM1czKUmPISbxukM/Figma---Development-Workflow) and [project brain](https://www.figma.com/board/1DhV61xcYdFD3SbtWbGBHN/ViperPilot-%25E2%2580%2594-Project-Brain---Roadmap) are planning references; this documentation refresh did not update or validate those files. Custom interactive art must add WidgetInfo, keyboard access, and focus feedback. |
| Native accessibility | Enable AccessKit | Check roles, names, patterns, and screen-reader use on Windows; a feature flag alone is not proof. |
| Renderer | Enable Glow; leave WGPU off | A 2D editor does not need WGPU features now; compare actual size and driver behavior. |
| Fonts | Start with bundled defaults | `system_fonts` is separate and disabled. Test long and non-Latin profile names and all decorative glyphs; add system fallback only if coverage requires it. |
| State persistence | Leave eframe persistence off | The app needs one explicit, versioned profile schema and rollback policy rather than a second implicit UI store. |
| Multiple viewports, web build, inspection server | Leave off | The requested product is a native Windows editor with one detailed window; add only for a demonstrated need. |
| egui_kittest | Use semantic/behavior tests | Query accessible roles/names and exercise controls headlessly. Its image snapshots require extra WGPU/snapshot features, which are omitted from this preview. |
| Tray, global hotkey, HID, installer | Keep outside egui | These are platform and device services already owned by the native shell and guarded worker. |

## Required gates before production integration

1. The feature-gated preview cross-builds with the repository's mise task and runs without hardware access. Existing default Linux CI, native Windows CI, and Windows production cross-build remain green.
2. Behavior tests edit synthetic profile names, DPI intent, polling intent, and button mapping intent; no preview action can request a device write or claim unverified settings are applied.
3. On Windows, inspect the persisted editor's UIA roles/names, keyboard focus and activation, IME composition, paste and non-Latin profile names, screen-reader output, high-DPI scaling, and the approved Figma dark compact/detailed states. Record failures as well as passes. These owner-present editor and compact-switcher checks remain pending.
4. Measure binary size, cold open to interactive, close/reopen, and idle CPU/working set for tray-only, editor-open, and editor-closed states. Compare with the current native build on the same machine. Do not assert a lighter or faster app from library choice alone.
5. Before connecting a real editor to the worker, define a versioned semantic profile-intent schema, migration/rollback, per-device capability contract, inspectable write plan, independent readback, and fail-closed behavior. The first immutable baseline is never a profile or draft.

## Observed preview results (2026-09-29)

The implementation candidate's hardware-free foundation checks report 108 core tests and 3 egui preview tests passing, with Linux and Windows CI checks green. These are software-only results; they do not establish production tray behavior or physical-device behavior. PR 3 remains a draft. Current work is tracked in [issue #12](https://github.com/spencer-life/viperpilot/issues/12) for offline draft persistence, [issue #4](https://github.com/spencer-life/viperpilot/issues/4) for capability contracts, and [issue #13](https://github.com/spencer-life/viperpilot/issues/13) for guarded promotion. Manual compact accessibility, production tray, editor presentation, and production performance checks (#6–#9) remain open.

- The isolated Windows preview cross-build and runtime smoke passed. Its executable is 7,461,888 bytes; the current production tray executable is 906,752 bytes in the same cross-build target. This compares binary size only, not runtime overhead. The default production dependency tree contains no eframe package.
- Headless egui/AccessKit tests passed (3): synthetic values stay editable, the `Profile name` input has a semantic text-input role and name, and Apply remains disabled. Windows UI Automation found that input as an enabled Edit and both Apply and Save as disabled Buttons. The preview opened and closed with exit code 0. The test did not exercise a screen reader or keyboard-only navigation.
- Headless Linux Xvfb/Mesa renders exercised the synthetic quick-edit and all-settings views after replacing unsupported decorative glyphs and pinning the disabled action footer. Screenshot files are omitted from this public repository; these renders do not establish Windows display behavior.
- Windows desktop capture remains inconclusive: a normal desktop capture showed a white client area while a separate renderer-frame capture showed rendered controls. It is not established whether the issue is presentation or capture. Physical display appearance, high-DPI behavior, time to interactive, resident tray resource comparison, and production process separation remain open gates. The capture images are not included in this public repository.
- `mise run measure-egui-preview-windows` stages only the preview EXE and repeats three launches. Window-handle detection took 301, 69, and 75 ms; working set after two seconds was 83.3, 82.2, and 82.4 MiB; process CPU time added 0 ms in each following five-second idle interval at the available counter resolution. Each process closed with exit code 0. These are one-machine preview measurements, not time to interactive, tray-only overhead, or a Synapse comparison.
- As of the issue #12 implementation, the optional editor uses the separate local draft library for create/edit/rename/duplicate/delete/save/reopen. Enabled fields match V1 intent; unsupported settings are unsavable. Save has no planner or HID path, and Apply remains disabled. Historical Windows measurements above apply to the earlier synthetic preview; the persistent editor has not been manually tested on Windows.

## Customization boundary

Saved quick-switch entries currently alias complete Developer/Gaming presets; a separate local draft library already stores semantic user intent. The optional user-facing editor now persists that local intent; production tray launch integration remains deferred. Drafts are not onboard slots or permission to send a setter. The documented hardware scope is one Viper V4 Pro field set: wireless 1000/4000 Hz transitions and particular Mouse4/Mouse5 actions have recorded evidence under their specific conditions. A 1600-to-1600 DPI proof does not validate changed DPI values. Other actions, fields, transports, and devices require their own isolated evidence before Apply can be enabled. See [ROADMAP.md](../ROADMAP.md), [quick-switch status](quick-switch-status.md), and [AGENTS.md](../AGENTS.md) for current gates. Full serials and raw reports remain private local evidence.

## Primary sources

- egui overview and integration responsibilities: https://github.com/emilk/egui
- eframe feature manifest and renderer choices: https://github.com/emilk/egui/blob/0.36.2/crates/eframe/Cargo.toml
- egui architecture: https://github.com/emilk/egui/blob/0.36.2/ARCHITECTURE.md
- Accessibility and semantic widget tests: https://github.com/emilk/egui/blob/0.36.2/docs/accessibility.md
- egui_kittest guidance: https://github.com/emilk/egui/blob/0.36.2/crates/egui_kittest/README.md
- Current egui workspace version, MSRV, and license: https://github.com/emilk/egui/blob/0.36.2/Cargo.toml
- eframe native options and close behavior: https://github.com/emilk/egui/blob/0.36.2/crates/eframe/src/epi.rs
- egui-winit Windows input handling: https://github.com/emilk/egui/blob/0.36.2/crates/egui-winit/src/lib.rs
- Rust 1.98.1 release: https://blog.rust-lang.org/2026/09/03/Rust-1.98.1/
