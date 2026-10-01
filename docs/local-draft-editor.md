# Local draft editor

Updated 2026-09-29 for issue #12. The optional `egui-preview` executable now
edits persisted local intent. It remains outside the native tray, startup,
quick-switch worker and device paths. This change enables Save, not Apply.

## Workflow and stored fields

Open `viperpilot-egui-preview.exe` after building with
`mise run build-egui-preview-windows`. Create a draft and choose **Save draft**;
select it later to reopen its saved values. Edit the name to rename it, keeping
its stable ID. Duplicate creates another ID; Delete requires confirmation.
Unsaved changes require Save or Discard before navigation or closing.

Every enabled setting maps directly to the unchanged V1 intent schema:

| Editor field | Persisted intent |
| --- | --- |
| Name | `name` |
| Independent X and Y DPI | `dpi.x`, `dpi.y` |
| Polling | `polling_hz` |
| Mouse4 and Mouse5 | Pass-through, unassigned, shortcut modifiers/key, or unmodeled description |

DPI stages, other button mappings, lift-off and sleep settings are outside V1
and are explicitly unsavable. Loaded values are preserved, including valid
polling targets outside the suggested choices. Validation errors retain the
editing buffer and the previous saved library; values are never converted to
raw device assignments.

## Persistence and recovery

The editor uses `%LOCALAPPDATA%/ViperV4Utility/profile-drafts-v1.json`, separate
from `profiles-v1.json`, configuration, diagnostics, reports and immutable
baselines. It never reads or changes built-in recovery presets. A missing file
opens an empty library without writing. Corrupt, invalid or newer-schema files
block editing and are preserved. Preserve a copy before investigating a file;
do not reset it merely to open the editor.

Each mutation validates a copy, acquires a temporary `.profile-drafts-v1.lock`,
checks that the draft bytes still match those loaded by this session, and uses
the existing atomic save. The session changes only after the save succeeds.
Another session's changes cause an explicit conflict instead of replacement;
reopen to load the latest drafts. Unsaved edits must be saved elsewhere or
deliberately discarded before reloading.

An interrupted save can leave a lock. Close all draft editor processes before
removing only `.profile-drafts-v1.lock`; preserve the JSON and any temporary
save evidence. The app does not automatically delete an unfamiliar lock.
Atomic replacement is not a claim of universal power-loss durability.

For isolated checks, launch with `--draft-root <temporary-directory>`. The
Windows smoke and measurement scripts pass their own temporary roots. Their
updated scripts have not been run during this owner-away session.

## Validation boundary

At source checkpoint [`6792c6f`](https://github.com/spencer-life/viperpilot/commit/6792c6fe8bc3774ab133a0b830fac8796b421e9d), `mise run ci` passed 108 core tests and its lint/build/smoke checks.
`mise run ci-preview` passed strict Clippy, 114 library tests and seven editor
tests. Production and editor Windows cross-builds passed without launching.
Independent review found no remaining material findings.

Hardware-free tests cover persistence, validation, failed saves, external
changes, storage isolation and accessible editor actions. `mise run ci-preview`
includes both the feature-gated library session and binary UI tests. Hosted
Windows preview checks also include these headless tests. Default production
builds exclude the editor module and eframe.

Apply remains disabled for every draft. No connected device is assumed;
changed DPI, arbitrary polling/actions and full-profile combinations still
need device-specific evidence. Manual Windows presentation, keyboard,
screen-reader, scale, performance and mouse tests are deferred until the owner
can observe them. PR #3 remains draft; no installation or startup change is
part of this work.
