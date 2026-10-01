# ViperPilot

This repository contains a native Windows utility for the Razer Viper V4 Pro.
Treat hardware behavior as measured only when the repository records the test
conditions and evidence. Label protocol or product assumptions as unproven.

## Working rules

- Keep changes scoped to this utility. Use the repository's `mise.toml` tasks
  for package checks and builds; it also pins the Windows cross-build tools.
- Date and justify changes to configuration, protocol behavior, and supported
  hardware inline. Record failed experiments and ruled-out explanations along
  with successful results.
- Preserve rollback paths for install or configuration changes. Do not discard
  reports or diagnostic evidence that explains a hardware result.
- Change and validate one logical device field at a time. Independently read it
  back, then restore the recorded baseline before proceeding to another field.
  Treat every setter as a possible flash write.

## Hardware access and safety gates

- `viperctl enumerate` reports operating-system enumeration only.
- `snapshot`, `probe-polling`, `plan`, `plan-polling`, and `verify` are GET-only.
  A plan or a GET result is not proof that a hardware write succeeded.
- Do not run `apply` or `restore` unless a complete immutable baseline exists,
  the connected VID/PID and full serials have been read and reviewed, and the
  exact write plan has been reviewed. Confirm the device using the utility's
  required serial suffix as a final command guard; that suffix does not replace
  full identity review.
- Keep the first baseline immutable. Never overwrite it, guess an unknown raw
  assignment, or proceed through ambiguous device identity or unverified state.
- Test only one logical field per hardware experiment. Independently reread the
  field and restore the baseline before starting another experiment.
- Never kill Synapse. Do not add runtime button interception or input
  injection as a fallback.
- Preserve the fail-closed behavior for unsupported devices, fields, polling
  transitions, or protocol responses. Do not broaden hardware writes based on
  code inspection alone.
- Record measured process-exit and mouse power-cycle behavior in `README.md`.
  Tray source may be compiled and smoke-tested without enabling startup. Do not
  release the tray until both complete profiles and the production hotkey pass
  their documented validation gates.

## Project direction

For a new session, read `docs/continue-here.md` before making changes. Continue
the public stack documented in `docs/continue-here.md`; GitHub is the
implementation source. `codex/sanitized-quick-switch` owns the catalog layer;
use the handoff table for the current top and any safety layers above it. Make each feature change on its owning layer; create a small new PR for
a new concern instead of accumulating unrelated work on the top branch. Do not reapply the historical ZIP patch or restart the
saved-profile foundation. Keep PR #3 and the new split PRs in draft while their validation gates are
open. The owner is away from the PC as of 2026-09-29; defer manual desktop and
physical-device tests until an observed session is available.

The owner's 2026-09-28 priority is quick access and fast profile switching with
minimal background overhead, not merely replacing the Synapse settings editor.
Read `docs/quick-switch-status.md` for this decision, the current source-only
boundary, and recorded validation blockers before continuing the profile work.
Do not mistake local named configurations for independently stored onboard slots.

The local draft editor (#12) and read-only capability contract (#4) are
implemented in focused draft PRs #15 and #16 respectively. Read `docs/capability-contract.md` before promotion
work (#13), and `docs/profile-catalog.md` for the opt-in offline projection.
Catalog export never replaces V1 production files. No custom combination is approved; Apply stays disabled until the
documented device, field, recovery and complete-profile gates pass. See `docs/tooling-decision.md` for the Cargo/mise decision; Aube was not
adopted, and its embedded Rust library was not compiled or benchmarked.

`ROADMAP.md` describes possible expansion. It is planning material, not evidence
that a capability is supported. Update the roadmap and its validation gates as
measured support changes.

## Preserve the installed app

2026-09-29: the owner uses the legacy app daily. Development must preserve its
installed binaries, `%LOCALAPPDATA%/ViperV4Utility` data, shortcuts, startup,
process and device state. Build and test only in repository output or temporary
folders. Default development data is `%LOCALAPPDATA%/ViperPilotDevelopment`;
never import or overwrite the legacy config or immutable baseline automatically.
`install-windows`, `scripts/install.ps1` and the production tray entry point
deliberately fail before installer/tray effects.
Do not bypass them or run the production tray while the owner is away. Explicit
editor/catalog roots must also point at separate development or fixture data.
Read `docs/development-isolation.md` before any later manual test. Separate
folders do not isolate the same physical mouse or global hotkeys/startup.

## Small PRs and stacked work

2026-09-29: the owner requested focused PRs so each feature can be reviewed and
fixed independently. Use GitHub's official `gh-stack` skill and `github/gh-stack`
CLI extension when creating or changing dependent PR layers. Read the skill before
stack operations; if it is unavailable locally, consult the
[official skill](https://github.com/github/gh-stack/blob/v0.1.1/skills/gh-stack/SKILL.md).

- Keep one coherent concern per PR. Independent work gets a separate branch/PR;
  dependent work gets a focused layer based on the preceding branch.
- Reconcile current GitHub heads and stack membership before editing. Use the
  ownership table in `docs/continue-here.md`, and make fixes on the layer that
  owns them. Preserve upper-layer work when reconciling dependencies.
- Use explicit non-interactive flags and the intended remote. For existing PRs
  across worktrees, `gh stack link` with full PR URLs registers the chain without
  rewriting commits; local navigation needs local stack tracking first.
- Preserve draft states and existing write/merge authorization. Do not use
  `--open` to mark drafts ready, force-push, or merge as an incidental stack step.
  A stack merge can include every unmerged layer below its target.
- Verify current integration checks after bases or heads change. Keep issue
  ownership and the existing Project dashboard current. Small PRs do not relax
  Apply, immutable-baseline, manual Windows/mouse or tray-release gates.

## Public issue tracking

Track development work in the public issue tracker at
https://github.com/spencer-life/viperpilot/issues using `gh-issue-sync` and the
Markdown files under `.issues/open/`. Pull selected issue state with `--full`
before editing or pushing so Project-only changes are included. Preserve Project
membership in frontmatter and verify it after pushing; an incremental pull can
miss membership changes that do not update the issue timestamp. Use credentials
that can read the Project, not a repository-only integration. Keep full device identifiers, raw reports, local paths, private-repository
links, and unredacted screenshots in private local records; public issues may
contain only redacted summaries. An issue does not authorize hardware writes;
follow the safety gates above.
