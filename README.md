# ViperPilot

ViperPilot is a lightweight native Windows utility for the Razer Viper V4 Pro. Current support remains limited to documented device and field gates; other mouse models are not implied.

The main goal is fast access and direct profile switching through the native
Windows tray or hotkey, with minimal background overhead. User-configurable
settings are an expansion priority. Full Synapse parity and support for every
recent mouse are not established requirements.

## Continue development

Start with [the continuation brief](docs/continue-here.md), then read
[implementation and validation status](docs/quick-switch-status.md) and
`AGENTS.md`. Current development is on `codex/sanitized-quick-switch` in
[draft PR #3](https://github.com/spencer-life/viperpilot/pull/3), stacked on
[foundation PR #1](https://github.com/spencer-life/viperpilot/pull/1).
The optional editor now supports [editable, saved local drafts (issue #12)](https://github.com/spencer-life/viperpilot/issues/12).
Save persists local intent; Apply remains disabled. Owner-observed Windows UI
and mouse tests remain pending; PR #3 stays draft.

## Local draft editor

Build the separate Windows editor with `mise run build-egui-preview-windows`,
then open `viperpilot-egui-preview.exe` on demand. It is excluded from the default
tray build and is not started or installed by this change. Create a draft, edit
its name, X/Y DPI, polling or Mouse4/Mouse5 actions, and choose **Save draft**.
Select a saved draft to reopen it; **Duplicate draft** and **Delete draft** affect
only the local draft store. Renaming uses the name field and Save. Unsaved edits
require Save or Discard before switching or closing.

Drafts use the existing versioned `profile-drafts-v1.json` under
`%LOCALAPPDATA%/ViperPilotDevelopment`. Corrupt, invalid or future-format files show an
error and are preserved. Drafts are separate from recovery presets, aliases,
configuration, diagnostics and immutable baselines. No draft action plans or
sends a device write. Unsupported fields are explicitly unsavable. For isolated
editor checks, pass `--draft-root <temporary-directory>`; this uses a separate
store. See [draft persistence details](docs/local-draft-editor.md).

The collapsed **Capability evidence** section distinguishes offline intent from
recorded device evidence. The [read-only capability contract](docs/capability-contract.md)
uses an exact model, firmware, transport and HID collection scope. It never
authorizes a custom profile or creates a device write plan; other mice inherit
no support.

## Offline catalog preview

`viperctl catalog-preview --root DIRECTORY` previews existing preset aliases and
frozen local draft copies without writing. `catalog-export` creates a separate
`profile-catalog-v1.json` and refuses to replace any existing export. Production
files and the quick-switch pair stay unchanged; drafts remain unverified. See
[catalog and rollback details](docs/profile-catalog.md).

## Validation and safety

Its GitHub Actions checks are hardware-free; they validate source and tests without connecting to or writing to a mouse. Historical raw device measurements and diagnostics are deliberately not published here; their absence does not prove hardware support. See [quick-switch status](docs/quick-switch-status.md) for software implementation and validation boundaries.

Hardware support and writes must follow the evidence gates in `AGENTS.md` and `ROADMAP.md`. Keep full device serials, personal paths, and raw hardware reports out of this public repository.

Source is licensed under GPL-2.0-only. See `LICENSE` and `NOTICE.md` for license and provenance details.

Product planning is tracked in the [public issue tracker](https://github.com/spencer-life/viperpilot/issues) and [roadmap](ROADMAP.md).
