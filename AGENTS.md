# Viper V4 Pro Utility

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

`ROADMAP.md` describes possible expansion. It is planning material, not evidence
that a capability is supported. Update the roadmap and its validation gates as
measured support changes.
