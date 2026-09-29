# ViperPilot

ViperPilot is a lightweight native Windows utility for the Razer Viper V4 Pro. Current support remains limited to documented device and field gates; other mouse models are not implied.

Its GitHub Actions checks are hardware-free; they validate source and tests without connecting to or writing to a mouse. Historical raw device measurements and diagnostics are deliberately not published here; their absence does not prove hardware support. See [quick-switch status](docs/quick-switch-status.md) for software implementation and validation boundaries.

Hardware support and writes must follow the evidence gates in `AGENTS.md` and `ROADMAP.md`. Keep full device serials, personal paths, and raw hardware reports out of this public repository.

Source is licensed under GPL-2.0-only. See `LICENSE` and `NOTICE.md` for license and provenance details.

Product planning is tracked in the [public issue tracker](https://github.com/spencer-life/viperpilot/issues) and [roadmap](ROADMAP.md).
