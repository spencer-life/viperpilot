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
After the owner closed the prior preview, this hash-checked executable launched
from a new temporary staging folder using the preserved temporary draft library.
Actual Windows UIA confirmed `Manual Test`, DPI X/Y `1200 DPI`/`1200 DPI`,
enabled Save draft and disabled Apply. The first read occurred before the main
window was ready; a fresh owned-process read established these values.
The first screen capture was occluded by Codex and does not prove appearance.
The owner-present focus observations above belong to the prior binary. After the owner brought the new preview to the foreground, a private actual
Windows capture at 2576×1408 showed the palette, consistent card widths, all
profile/action controls and bottom Save/disabled Apply rendering legibly without
overlap. The bounded form leaves substantial empty space when maximized; this is
not pixel parity with the compact raster concept. The owner then repeated the
Tab/Shift+Tab check on this styled build and confirmed focus remained visible.
This is an owner-observed focus/navigation result, not screen-reader speech or
IME validation. Other display scales remain unobserved. This improves
the reference palette, type size and control layout; it does not claim pixel
parity with the raster concepts or completion of compact/detailed view alignment.

### Styled Windows paste/Unicode round trip — 2026-09-30

The owner pasted `Test – 日本語 – café`, saved and closed the styled isolated
editor. Reopening the same hash-checked executable and temporary library
preserved that exact Unicode sequence in both the saved JSON and actual UIA
Profile name/Edit value. DPI X/Y remained 1200/1200 and Apply remained disabled.
This validates the requested paste/name persistence case, not IME composition or
screen-reader speech. The owner also confirmed visible keyboard focus on this
styled build. Narrow-window/other-scale and reader/IME checks remain open.

### Owner minimum-window screenshots — 2026-09-30

Owner-provided Windows screenshots at 850×640 show Save and disabled Apply
contained at the bottom in both initial and scrolled states. Mouse5 is reachable
by scrolling. These establish the observed minimum-window layout at this
display scale, not other DPI scales.

The same screenshots expose missing Japanese glyphs: `日本語` renders as boxes
in the name input, navigation and status. The exact Unicode storage/UIA round
trip passed, but visual non-Latin name support failed. Do not describe the
persistence result as a complete input/rendering pass. Windows lists installed
MS Gothic and Yu Gothic font collections; an editor-only font fallback fix is
being investigated. IME composition and screen-reader speech remain unobserved.

### Missing-glyph correction — 2026-09-30

The Windows editor appends one locally installed, glyph-validated Japanese font
to the bundled proportional/monospace fallback chains. Latin fonts and type sizes
are preserved. Font reads are bounded to 64 MiB; missing, unreadable or malformed
fonts retain defaults. Validation occurs in an isolated context; real eframe
initialization only receives the validated definitions, preserving texture uploads.
A rendering warning appears only when the selected name has missing glyphs and
the fallback was unavailable. No font was installed, redistributed or added to
the resident tray. This is not a claim of universal Unicode glyph coverage.

Strict preview Clippy and 159 tests (13 editor tests), Windows cross-lint, release
build, formatting/whitespace and independent review passed. Initial checks caught
unused mutability and excessive boolean state; these were corrected. Review also
removed an extra real-context startup pass before launch to avoid discarding atlas
updates. Corrupt-font rejection and preserved default family order are tested.

The hash-checked build launched with the same preserved temporary library. Actual
Windows UIA still read exactly `Test – 日本語 – café`, DPI X/Y 1200/1200 and
disabled Apply. Normal and minimum-size owned-window PrintWindow captures showed
the Japanese glyphs correctly in the input, navigation and status; Save/Apply
were contained. This background capture succeeded without foreground manipulation.
An initial redirected readback used the Windows legacy output encoding; selecting
UTF-8 output corrected the capture of values, with no app/data mutation. Raw
captures stay private; capture margins do not establish pixel parity or other
DPI-scale behavior. IME composition and screen-reader speech remain unobserved.

Font-fixed Windows binary SHA-256:
`e15529a64537c2dafacf1941241a75eccc996135f02b6b05a5d77fa84a70f870`.
