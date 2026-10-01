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
