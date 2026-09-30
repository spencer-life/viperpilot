---
title: Verify compact switcher accessibility and display behavior
labels:
    - accessibility
    - codex-created
    - release-gate
projects:
    - ViperPilot dashboard
state: open
state_reason: ""
synced_at: 2026-09-30T11:09:04.970287424Z
info:
    author: spencer-life
    created_at: 2026-09-29T18:22:41Z
    updated_at: 2026-09-30T07:21:41Z
---

## Context

The compact native switcher and tray flow need verification on an interactive Windows desktop. Synthetic actions and builds cannot establish keyboard, screen-reader, or real display behavior.

Implementation baseline: [public PR #3](https://github.com/spencer-life/viperpilot/pull/3) (draft).

## Done when

- Verify tray menu and compact/details labels, focus order, keyboard activation, accessible names/roles/patterns, and screen-reader announcements. Fix any control that cannot be used or understood without a mouse.
- Review legibility and hit targets at the display scales and narrow-window sizes used for the app, including reversed saved pairs, disconnected/unknown device state, and unavailable/corrupt library states. UI-only checks must not apply a profile.
- Record conditions, failures, and fixes in repository documentation. Publish no unredacted screenshots or device-identifying information. Keep the release gate open until the production UI passes.

## Checkpoint — 2026-09-29

Accessibility and real monitor/display behavior remain unverified. No Windows desktop or screen-reader validation was run for this checkpoint; keep the release gate open. Synthetic or Linux checks do not establish this issue's acceptance criteria.

## Software readiness — 2026-09-30

Generic worker/read errors now mark current state unavailable, clearing observed values and disabling switching/hotkeys until successful refresh. Pure GUI regression and Windows cross-build pass. Real native control roles/patterns, keyboard, screen-reader and scaling checks remain manual. See [manual handoff](https://github.com/spencer-life/viperpilot/blob/codex/ui-readiness/docs/manual-handoff.md). Installed app unchanged; no GUI/HID or startup operation ran. Issue remains open for its manual and delivery gates.

## Native accessibility remediation — 2026-09-30

The recorded owner-draw pane/no-Invoke result was a software defect. The
[native accessibility layer](https://github.com/spencer-life/viperpilot/tree/codex/native-button-accessibility)
now prepares standard Windows button styles and semantic hotspot names through
the system annotation service. The isolated preview script requires UIA Button
and InvokePattern, checks all six semantic hotspot names, and invokes Details via
UIA. Windows production/preview cross-builds, Windows-target lint and independent
review passed. The PowerShell script, GUI/UIA and screen reader were not run;
actual roles, names, invocation, keyboard, contrast and scaling remain manual
validation gates. The production tray/installer stay blocked and the installed
app remains unchanged.

## Owner-present preview result — 2026-09-30

Windows 11 Pro build 26200 / PowerShell 5.1 STA: all three isolated native scenarios now pass strict native UIA Button/Invoke, action names and enabled states, all six semantic hotspot names, actual UIA Details invocation, synthetic profile/view transitions and clean exit. The earlier managed-client Pane result alone was insufficient to attribute an app defect: the same current HWND reports Button/Invoke through native CUIAutomation. A separate feature-gated, PID/class/child/control-guarded native test client corrects the harness; process/output waits are bounded. Windows lint and independent review pass.

See [tested harness and private-evidence summary](https://github.com/spencer-life/viperpilot/tree/codex/native-uia-test-client). Keyboard, screen-reader speech, contrast/scaling and production tray/device behavior remain separate manual gates. Installed app untouched; no production, startup or mouse action ran. Issue stays open.

## Native visual alignment — 2026-09-30

The [native visual layer](https://github.com/spencer-life/viperpilot/tree/codex/native-figma-polish)
adds a preview-only Common Controls v6 manifest and supported custom drawing of
standard buttons. Final strict Windows checks and all three isolated native UIA
scenarios pass. Owned background captures show rose/charcoal controls, readable
power labels, muted disabled states and visible focus. The invalid-HDC fallback
test passes on Windows; independent review found no material issues. Earlier
white corners and old blue emphasis were caught in captures and fixed.

[Validation record](https://github.com/spencer-life/viperpilot/blob/codex/native-figma-polish/docs/native-visual-validation.md)
retains failed experiments and limits. Keyboard activation, screen-reader speech,
other DPI scales, high contrast and production tray remain separate gates. No
installed app, startup or hardware operation occurred. Keep this issue open.

## Owner spacing feedback and dashboard correction — 2026-09-30

The owner reports working Tab navigation on both native views and marked compact
action spacing, overlapping mouse-number borders and narrow detailed cards. They
also rejected the old inspector layout as a match for the approved Figma render.
[Draft #29](https://github.com/spencer-life/viperpilot/pull/29) separates the
Device dashboard from the numbered read-only Buttons inspector. Compact action
labels are padded/centered; hotspot targets are separate; inspector status cards
are wider and power rows have spacing. The Device page follows the reference
sidebar/large mouse/right summary structure with read-only rows and Customize
buttons navigation. Existing illustration retained; pixel identity is not claimed.

Final strict Windows checks and all three native UIA scenarios pass, including
view visibility, direct Device-to-Compact restoration, hidden-control refusal and
clean exit. Windows hotspot-geometry and view-routing tests pass. Captures and
independent source review were checked; failed experiments remain in the
[validation record](https://github.com/spencer-life/viperpilot/blob/codex/device-dashboard-layout/docs/native-visual-validation.md).
Enter/Space activation of the revised views, screen-reader speech, other scales,
high contrast and production/hardware gates remain open. Installed app untouched.
