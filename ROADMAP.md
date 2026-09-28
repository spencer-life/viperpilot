# Viper Mouse Utility Roadmap

> The project and product name are provisional. This document is planning guidance, not evidence of device support.

Historical device measurements and diagnostic reports remain private and are not included in this public repository. Their absence does not establish or disprove support for any device or setting.

## Planned work

1. **Research capabilities.** Identify candidate mouse models, connection modes, and controls. Separate primary-source documentation from measured behavior and record unknowns.
2. **Build software foundations.** Define validated configuration and profile handling, safe planning, independent readback, and recovery paths. Unsupported devices, fields, and responses remain unavailable.
3. **Validate hardware changes.** For each candidate field, use an immutable baseline, review full device identity and the exact plan, test one logical field, read it back independently, and restore the baseline. Record conditions and results.
4. **Design the desktop experience.** Shape the app and tray around capabilities that have passed their evidence gates; keep unknown or unsupported controls clear to the user.

## Evidence gate

Treat existing write paths as unverified by this public evidence set. Do not exercise or expand them until new evidence is recorded in this repository and the safety gates in `AGENTS.md` are satisfied. Similarity to another mouse, source inspection, and missing historical evidence are not proof of support.
