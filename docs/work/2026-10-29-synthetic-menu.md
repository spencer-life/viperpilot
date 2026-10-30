# Synthetic native menu validation — 2026-10-29

The owner's unattended follow-up targets software checks that require no mouse
access. The isolated native preview now opens the real popup-menu builder via
right-click or Shift+F10. This creates no notification-area icon and starts no
worker, storage, startup, hotkey, instance mutex or HID integration.

All selections route through an explicit in-memory handler. It never calls the
production dispatcher or notification helper. Fixed recovery actions keep their
preset semantics even with a reversed pair; quick actions follow the synthetic
pair. Startup/diagnostics checkmarks are simulations only, reload performs no
I/O, and Exit closes only the preview. `--pilot-read-only` projects disabled
profile actions and omits startup. Its window title retains PREVIEW.

The modal popup is deferred and shown after releasing the application borrow.
Its selection is handled against the displayed snapshot before queued worker
events. The preview ignores device, worker and global-hotkey messages. Known
posted menu IDs cannot bypass its disabled apply or pilot-startup restrictions.

## Software checks

- Native core CI passed: 150 library + 2 CLI + 3 capability + 6 catalog tests,
  formatting, Windows Clippy, release build and hardware-free CLI help.
- Preview CI passed: 165 library + 2 CLI + 14 editor + 3 capability + 6 catalog
  tests, combined-feature Clippy and release build.
- Nine new tests inspect actual native HMENU objects: default/reversed labels
  and markers, unavailable library, busy/disconnected/read-only action states,
  fixed recovery, Unicode names, local pair changes and posted-ID guards.
  A live synthetic receiver proves selections queue no worker command. Geometry
  and sentinel tests cover keyboard placement, zero/sign extension on x64,
  negative desktop coordinates and 0/96/144/192 DPI inputs. Informational rows
  are disabled and their ID 0 has no command route.
- Existing default/reversed/unavailable preview harness scenarios passed:
  semantic buttons, transitions, hidden/disabled guards and queued Enter.
  Each test-owned process exited 0 and its fixture staging was cleaned up.
- The initial marker test expected MF_CHECKED; the existing renderer uses a
  textual checkmark for pair selection. The assertion was corrected to inspect
  that actual label. No production behavior was changed to satisfy the test.
- The Windows preview task previously linted both preview features but tested
  only the editor feature. Its test command now enables both so these native
  menu regressions actually execute in CI.

Computer Use checked default, reversed, unavailable and read-only modes on the
initial menu build. Right-click, Shift+F10, arrows, Enter and Escape worked;
reversed quick targets and fixed recovery stayed distinct. Restricted actions
were gray and pilot startup absent. Three modes had recorded exit code 0; the
default closed but its launch handle was not retained for that observation.

The visible check found keyboard invocation using the existing cursor location,
which could place the popup far from its window. The fix anchors keyboard menus
near the window's upper-left using its DPI, while pointer/tray invocation keeps
its previous placement. Review caught the x64 coordinate-extension edge case;
both forms are covered. Final core/preview CI and all three scripted keyboard
harnesses passed the corrected build.

Targeted Computer Use retesting passed on corrected preview `ab1c532e2bf1` in
default and read-only modes. With the pointer near the lower-right, Shift+F10
opened the popup near the window's upper-left in both. Right-click still used
the pointer. Arrows, Enter, submenu behavior and Escape worked; Diagnostics
toggled only the synthetic checkbox. Both retained process handles recorded
exit code 0. The initial four-mode report and corrected-build retest remain
private; no editor scrolling or Save/reopen was repeated for this menu-only pass.

Native arrow navigation may focus a disabled informational row. It remains
gray/non-actionable; the regression verifies ID 0 cannot dispatch a command.
The saved-profile parent in read-only mode remains enabled so its disabled
children can be inspected. Independent source review has no remaining material
findings. This does not establish real notification-area, physical-input,
screen-reader speech or hardware behavior. The notification-area icon/anchor,
live profiles/hotkey, exit and power-cycle acceptance gates remain open.

## Separate delivery work

A second independent branch,
[test/portable-package-verification](https://github.com/spencer-life/viperpilot/tree/test/portable-package-verification),
adds a read-only ZIP verifier and native mise task. It validates the external
reviewed hash, exact source, manifest, safe inventory and four x64 headers
without extraction or execution. Both PowerShell versions accept the frozen
candidate and reject bad hash/commit, manifest and traversal fixtures. Review
fixed a hash/reopen race by inspecting the same open stream throughout.

The uploaded draft candidate stays pinned to `33adccf7dafa` and its assets are
unchanged. These source follow-ups do not merge, publish a release, trigger new
hosted PR checks, install, change startup, interrupt the legacy app or access
the physical mouse.
