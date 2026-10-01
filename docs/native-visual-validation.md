# Native visual alignment

Updated 2026-09-30. This layer follows the editor styling in draft PR #27.

## Target and boundaries

The approved [compact](https://www.figma.com/design/Ujs2cpFXYNnSI2YquOG1jF?node-id=3-3)
and [detailed](https://www.figma.com/design/Ujs2cpFXYNnSI2YquOG1jF?node-id=3-2)
renders are concept references. The explicit palette is documented in
`figma-visual-validation.md`. Preserve profile names, truthful status and supported
controls; concept values and generated device images are not hardware evidence.

Actual isolated Windows captures before this layer showed a charcoal native shell
and existing mouse art, but white system buttons, blue profile emphasis and clipped
Unavailable power values. The guarded native UIA client invoked Details between
captures; only the owned synthetic preview process was opened and closed. These
captures establish the mismatch, not a visual-match pass.

## Supported drawing path

Keep standard `BUTTON` / `BS_PUSHBUTTON` controls. The historical `BS_OWNERDRAW`
path caused inaccessible Pane/no Invoke results and must not be restored.
Microsoft documents [button NM_CUSTOMDRAW](https://learn.microsoft.com/en-us/windows/win32/controls/nm-customdraw-button)
for parent `WM_NOTIFY` handling, with a Common Controls v6 manifest and only
`CDDS_PREPAINT` / `CDRF_SKIPDEFAULT` as the supported skip-default case. Custom
painting must preserve focus, disabled, pressed and hot feedback. High contrast
or a failed high-contrast query must defer to native rendering.

The v6 manifest is embedded only in `viperpilot-ui-preview`, using
[Cargo binary-specific link arguments](https://doc.rust-lang.org/cargo/reference/build-scripts.html#rustc-link-arg-bin)
and the pinned LLVM 22.1.8 [COFF manifest options](https://github.com/llvm/llvm-project/blob/llvmorg-22.1.8/lld/COFF/Options.td).
This does not change the production tray/CLI/editor manifests, install any OS
component, enable startup or touch the installed legacy app. Production promotion
requires its own manifest and complete-profile/hotkey validation decision.

## Verification requirements

Inspect the actual embedded preview resource and confirm linker arguments are
preview-specific. Run strict native UIA Button/Invoke/name/enabled assertions in
default, reversed and unavailable states. Capture compact/detailed views again;
check contained text, palette, control states and actual focus drawing. Keep raw
captures private and retain failed experiments. Source/build checks do not prove
screen-reader speech, physical keyboard navigation, other display scales or
high-contrast appearance. No device or tray release gate changes.

## Observed Windows results (2026-09-30)

- `mise run verify-linux` passed before the Windows-only drawing hardening; the
  final `mise run verify-windows-cross` passed formatting, strict target lint,
  default production build, native preview/client build and editor build.
- Read-only PE resource inspection confirmed a 604-byte v6 manifest in the native
  preview and no manifest in the production binary. Neither binary was executed
  during that inspection.
- `mise run test-ui-preview-windows` passed default, reversed and unavailable
  scenarios: standard Button/Invoke roles, semantic names, enabled states,
  synthetic profile actions, repeated view transitions and clean process exit.
- The Windows library test
  `button_renderer_falls_back_for_an_invalid_device_context` passed on Windows.
  It covers initial SaveDC failure; later failure paths received independent
  source review, not fault-injection runtime coverage. Final review found no
  material issues.
- Background PrintWindow captures of owned staged previews were inspected for
  compact, detailed and unavailable states. Rose profile emphasis and buttons,
  contained power labels, muted disabled controls and the focused Quick switch
  outline are visible. Existing mouse artwork and functional native layout are
  retained; this establishes closer alignment, not pixel-for-pixel Figma fidelity.

The tested native preview SHA-256 is
`83ca61d9af816a1c4a6ac4d4e077d77ac6dcdaf99f905ebe93282ca4c028a891`.
Raw captures and test logs stay private. The preview used synthetic in-memory
state; no production process, startup setting, installed app or mouse was touched.

## Failed experiments retained

The first checked GDI helper used `.as_bool()` on FillRect's integer return; the
Windows compiler rejected it. It now tests zero. Strict lint then rejected its
13 positional inputs; `ButtonPaintStyle` groups the drawing options. Early styled
captures exposed white rectangular corner remnants and the old blue Developer
accent. Filling the complete button rectangle with its parent background before
rounded drawing and using the rose status tone resolved both in fresh captures.

Physical keyboard activation, screen-reader speech, other display scales and
high-contrast appearance remain manual gates. The high-contrast fallback is
reviewed source behavior, not an observed OS-setting test. Keep issue #6 open,
Apply gated and all feature PRs draft.

## Owner keyboard and spacing report (2026-09-30)

The owner reports that tabbing works in both compact and detailed previews.
Their screenshots show focus outlines on a compact profile button and detailed
Quick switch. Enter/Space activation was not reported, so that gate stays open.
Read-only inspection of the owned detailed preview confirmed 96 DPI (100% scale)
and an 880 × 680 client area. The owner also reports minor overflow on both screens. The compact wrapped action
labels sit against the top button edge. The owner specifically marked both
compact actions, overlapping numbered-button borders, and all narrow detailed
status cards. They also rejected the old inspector layout as a match for the
approved Device dashboard render. Retain this failed visual checkpoint pending a corrected
Windows capture and owner comparison.

The compact spacing correction was validated at source `347544b`: strict Windows
checks passed, fresh captures show padded centered action labels, and the Windows
hotspot containment/separation test passed for computed 96/144/192-DPI geometry.
These geometry tests do not establish real monitor scaling. The Device dashboard
layout correction is a separate follow-up within this visual layer.

## Device dashboard correction (2026-09-30)

This focused follow-up is draft [#29](https://github.com/spencer-life/viperpilot/pull/29)
on `codex/device-dashboard-layout`, based on draft #28.

The detailed view now follows the structural reference: a left Device sidebar,
a large contained mouse image and a right device card with three horizontal
read-only status rows and the rose Customize buttons action. The numbered
read-only Buttons inspector is a separate page, with wider two-column status
cards. Quick switch and Device provide navigation back. Profiles/Settings/About
are informational sidebar text with no command or keyboard-focus path. The
existing mouse illustration remains in use; the flattened concept image is a
reference, never an interactive-screen asset. Exact illustration, icon and shadow
identity is not claimed.

Windows strict lint/build checks and both Windows geometry/view-routing tests
pass. The final native preview hash is `2c61abdbe039393dddf969de2652f1ff749557063770a4e3927656ca4b7b0d80`.
All three final runtime scenarios passed standard Button/Invoke semantics,
visible-control checks, direct Device-to-Compact restoration, inspector navigation,
hidden-control invocation refusal, ignored hidden recovery commands and clean exit.
Fresh compact/Device/Buttons captures were inspected for containment and the
reference structure at 96 DPI. No production, startup or hardware boundary changed.

Review and runtime retained these failures: Device-to-Compact did not initially
restore actions/status; a connection label retained an old position; assignment
raw/hint rectangles overlapped; the initial hero contain ratio did not scale
correctly. Those were fixed. A runtime visibility assertion then exposed the
older inverse ShowWindow branch that always showed inspector controls; one
requested visibility now governs both groups. Fresh captures also caught text
rectangles extending past row borders and mismatched static background brushes;
controls now stay inside padded row bounds and use their row surface brush.
Final source review found no material findings.

A later runtime pass reported an inspector hotspot still hidden at the first
view assertion. The original three-control transition fingerprint could match
without verifying all inspector controls. The harness now also waits for all six
hotspots to have the required visibility before asserting the view. This records
the observed failure and harness correction; it does not attribute an intermittent
provider result to a proven application defect. Inspector power values now use
three spaced horizontal label/value rows rather than six tightly stacked lines.

The corrected, hash-checked manual preview was staged separately and opened on
Device for owner comparison. The preceding owned manual process was closed and
confirmed no longer running; its exit code was not verified by the external
Get-Process wrapper, which reported an error on the exit-code check. The prior
session record and staged folder were retained. Automated staged sessions above
reported exit code 0. No other process was closed.
