---
title: Verify compact switcher accessibility and display behavior
labels:
    - accessibility
    - codex-created
    - release-gate
state: open
state_reason: null
synced_at: 2026-09-30T07:21:43.066629204Z
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
