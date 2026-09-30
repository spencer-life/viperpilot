# Figma visual alignment

Updated 2026-09-30 after the owner requested close alignment with the approved renders.

## Reference and scope

The approved [Figma file](https://www.figma.com/design/Ujs2cpFXYNnSI2YquOG1jF)
contains raster concept references, not a component library or production assets.
Design context and rendered screenshots were inspected for the
[compact switcher](https://www.figma.com/design/Ujs2cpFXYNnSI2YquOG1jF?node-id=3-3),
[detailed dashboard](https://www.figma.com/design/Ujs2cpFXYNnSI2YquOG1jF?node-id=3-2)
and [palette](https://www.figma.com/design/Ujs2cpFXYNnSI2YquOG1jF?node-id=3-6).
The references select a compact-first layout, pink V and charcoal surfaces with
restrained rose accents. Generated device labels/statuses are not hardware evidence.
Do not embed the concept screenshot as an interactive screen or claim its values.

The first alignment slice styles only the on-demand draft editor. Existing native
compact/detailed views require a separate visual comparison; this result does not
claim that every view matches. The resident tray does not gain a renderer or new
dependency. Apply and production/device gates stay in force.

## Explicit reference tokens

| Role | Reference |
| --- | --- |
| Background | `#0F1419` |
| Surface | `#1A1F24` |
| Panel/input | `#232932` |
| Border | `#3A414B`, 1 px |
| Primary text | `#EAEFF4` |
| Secondary text | `#A3ACB7` |
| Rose accent | `#E87D91` |
| Active accent | `#A24B5C` |
| Success | `#3CCB8F`; use only for established successful state |
| Button/card radius | 8 / 12 px |
| Typography | Body 14, small 12, headings 24 / 20 px |

## Evidence and remaining checks

The actual owner-focused Windows capture before this slice was readable, but
showed a brighter pink, smaller text and default-looking compact controls. It
was not a visual-match pass. The owner separately completed a Save/reopen round
trip (`Manual Test`, DPI X/Y 1200/1200); storage validation is independent of styling. The owner also reported every
requested control reachable with visible focus using Tab/Shift+Tab on that
pre-styling build. An owner-supplied screenshot showed Mouse5 and the pinned
footer visible after scrolling. Recheck focus on the styled build.

Inspect the newly built editor at the same scale/window size before claiming
Windows visual alignment. Check full-width cards, control spacing, visible focus,
scroll access to Mouse5, pinned Save/disabled Apply, and scaling. Keep screenshots
private. Existing manual-session executable and data remain preserved; building a
new binary does not replace that staged session or the installed legacy app.

## Capture limitation

[ShareX CLI](https://getsharex.com/docs/command-line-arguments.html) supports
`-ActiveWindow`. Its current [capture implementation](https://github.com/ShareX/ShareX/blob/develop/ShareX.ScreenCaptureLib/Screenshot.cs)
uses the foreground window and desktop GDI pixels, not an isolated GPU surface.
It does not establish reliable occluded-window capture. This was a source review,
not a local ShareX runtime test. The existing preview harness tries PrintWindow
and falls back to a verified foreground capture; GPU content may remain
inconclusive. Do not treat an occluding window or blank capture as a rendering pass.

## First styling slice validation — 2026-09-30

`ci-preview` passed strict Clippy and 157 tests, including 11 editor tests.
`lint-windows-cross`, `build-egui-preview-windows`, formatting and whitespace
checks passed. Independent review identified selected-text contrast as 2.10:1
with rose on active rose; using primary text raises it to 4.91:1. Final review
found no remaining material issue.

Private Linux Xvfb/Mesa captures were inspected at 1080×790 and 850×640 with
20 synthetic drafts. Cards respected available width; the scrollable navigation
and bottom status stayed contained. The invalid-name state showed its validation
message and both disabled actions fully inside the minimum window. Actual bottom
panels replaced a fixed footer-height allowance. The first render exposed right
clipping after narrowing; computing the width before applying its 680px cap fixed
it, confirmed by the final capture.

An initial Xvfb display failed because the WSL Unix socket could not be created.
A process-local `-pn` display with TCP disabled worked through the abstract socket;
no persistent display configuration changed. An initial synthetic invalid-name
input did not reach the unfocused window; explicitly focusing only the private
offscreen window established the final invalid state. Only owned offscreen
processes were closed; fixture data and the owner's Windows session were preserved.

Final Windows editor SHA-256:
`810f93162f06978630671fd92fab1ef862e31adca111444a49ae5a0c62aeefcc`.
This executable has been built, not launched on Windows in this styling slice.
The owner-present focus/persistence observations above belong to the prior binary.
Windows visual/focus/scaling checks on this new binary remain open. This improves
the reference palette, type size and control layout; it does not claim pixel
parity with the raster concepts or completion of compact/detailed view alignment.
