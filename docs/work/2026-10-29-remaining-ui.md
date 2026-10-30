# Additional isolated UI coverage — 2026-10-29

This continuation addresses concrete gaps found in the earlier automated QA.
It preserves the installed legacy application and does not authorize production
launch, installation, startup changes, global hotkeys or device access.

## Changes and software evidence

The default two-entry library had no way to reverse its quick-switch pair:
each individual target menu correctly disabled the other occupied entry. A new
**Swap first and second targets** action exchanges both targets atomically.
The production handler saves only the local library; the preview changes only
memory. It does not apply a profile. Regression tests cover synthetic read-only
behavior and a persisted read-only worker swap with unchanged baseline bytes.

Native preview now accepts `--tray-icon`, optionally together with
`--pilot-read-only`. It creates a Shell notification-area icon with the distinct
tooltip **ViperPilot PREVIEW — no device access**. It retains disconnected
worker channels and has no storage, device worker, startup change, global
hotkey or instance mutex. Alt+F4 hides this opt-in preview; its menu Exit removes
the icon before destroying the window and terminates the process. Ordinary
preview invocation continues to create no icon. Version-4 activation accepts
the Shell keyboard/select events, and standard tooltips request `NIF_SHOWTIP`.

Independent source review found no remaining material issue in either change.
Local native CI passed formatting, Windows-policy Clippy, builds and CLI help:

- Core: 150 library, 2 CLI, 3 capability and 6 catalog tests.
- Combined preview: 168 library, 2 CLI, 17 editor, 3 capability and 6 catalog tests.
- Existing default/reversed/unavailable native harnesses all passed against
  the new executable, including queued Enter, hidden-control refusal and exit 0.

New editor tests send actual egui preedit/update/commit and cancellation events,
then move focus and Save/reopen. A separate test checks blank duplicate-name
refusal and Cancel byte preservation. These are headless integration events;
they do not establish the Windows IME bridge or physical composition behavior.

## Additional desktop evidence

The separate Computer Use chat tested the frozen editor in fresh QA roots:

- Mouse4 Ctrl+Shift+F10 and Mouse5 Alt+Windows+A survived Save/close/reopen,
  with exact modifiers, keys and stable draft identity confirmed in QA JSON.
- Empty duplicate names disabled Create copy; Cancel preserved the library.
- Dirty-close Keep editing retained the buffer; Save and close persisted it.
- Concurrent editors exposed a stale-save conflict, retained the losing
  buffer and preserved the other editor's saved bytes. Discard-and-reload is
  pending its action-time confirmation.
- A separate 20-entry fixture scrolled to and selected the final sidebar
  entry; details scrolled independently while the footer stayed visible.
- Delete Cancel preserved both disposable entries. Final Delete is pending
  its action-time confirmation.
- Native left/right/middle/DPI inspector hotspots selected the expected
  synthetic slots; their assignments remained explicitly unread/unavailable.

Private reports retain executable hashes, paths and session details. The
new native preview build is SHA-256
`f70c728d9c0376720d6da2bac82013c94de9435555aa0fb70cc29e39dca5448f`;
its read-only desktop retest swapped the pair and restored it, updated both
button/submenu labels, and kept the displayed profile Developer. Exit from the
preview-window menu terminated that process with code 0. Earlier desktop
evidence is scoped to its own executable hash.

The Computer Use inventory did not expose a notification-area target, so the
tester could not uniquely inspect the preview icon. Icon appearance, tooltip,
Shell context/activation, hide/reopen and disappearance remain unobserved.
No blind Shell interaction or legacy-icon selection was attempted.

## Remaining boundaries

Actual Windows monitor scaling/high contrast, screen-reader speech, physical
input and Windows IME composition remain unverified. No display/input setting
was changed, language component installed or screen reader launched. A proposed
display test still requires recording the actual initial state and exact
rollback before a separately authorized temporary host change.

The uploaded draft ZIP remains frozen at `33adccf7dafa`; these source changes
are not in that archive. Real-device preparation is a separate review. An
existing baseline candidate passed offline typed validation privately with
unchanged original bytes; its provenance and live identity remain unconfirmed.
No enumeration, GET capture, baseline import or mouse write has occurred.
