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
The next code task is [editable, saved local drafts (issue #12)](https://github.com/spencer-life/viperpilot/issues/12).
The editor remains a preview with Save and Apply disabled; this is not finished
user-facing customization. Owner-observed manual tests remain pending.

## Validation and safety

Its GitHub Actions checks are hardware-free; they validate source and tests without connecting to or writing to a mouse. Historical raw device measurements and diagnostics are deliberately not published here; their absence does not prove hardware support. See [quick-switch status](docs/quick-switch-status.md) for software implementation and validation boundaries.

Hardware support and writes must follow the evidence gates in `AGENTS.md` and `ROADMAP.md`. Keep full device serials, personal paths, and raw hardware reports out of this public repository.

Source is licensed under GPL-2.0-only. See `LICENSE` and `NOTICE.md` for license and provenance details.

Product planning is tracked in the [public issue tracker](https://github.com/spencer-life/viperpilot/issues) and [roadmap](ROADMAP.md).
