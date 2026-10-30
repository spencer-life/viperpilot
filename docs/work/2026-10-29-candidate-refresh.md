# Candidate source refresh — 2026-10-29

Branch `feat/pilot-candidate-refresh` assembles reviewed follow-ups without
changing main or replacing the original `33adccf7dafa` draft assets:

- UI coverage and atomic pair reversal: `6a5aae4`.
- Read-only review CLI and phased preparation: original `da4b600`, applied
  as `3c9be30`.
- Independent static ZIP verifier: original `5fb1a0d`, applied as `258cd99`.

Final integration CI passed locally: core 150 library + 5 CLI + 3 capability +
6 catalog tests; combined preview 168 library + 5 CLI + 17 editor + 3 capability
and 6 catalog tests, formatting, native Windows-policy Clippy, release builds and
CLI help. The UI runtime is unchanged from the independently reviewed native
preview that passed the three harness scenarios and visible pair-swap retest.
The CLI addition and verifier do not alter the editor runtime. No new hosted
workflow or PR was requested.

The portable build uses a clean exact source commit, an explicitly pinned
Windows x64 target/output directory, external ZIP checksum and per-file manifest.
Its `build-info.json` identifies that assembled commit. START-HERE now explains
the offline baseline inspector, the GET-only command's separate approval gate,
and the optional synthetic icon's observed limits. The package is a draft
supervised candidate; ordinary production launch remains blocked.

An existing owner baseline candidate passed private offline typed validation;
original bytes remain unchanged, and its recorded current/active-stage DPI
matches the fixed-preset 1600 × 1600 prerequisite. None of that establishes
baseline provenance, current live identity or safe writes. Enumeration,
observation, baseline staging/import, plan generation and hardware writes remain
unexecuted. The owner requested preparation for review only.

Desktop gaps and pending disposable-data confirmations are recorded in
[additional isolated UI coverage](2026-10-29-remaining-ui.md). Actual Windows
notification-area appearance, monitor scaling/high contrast, screen-reader
speech, physical input, Windows IME integration and all live-device acceptance
gates remain open. Preserve the installed legacy app, its process/data/startup,
the original draft archive and private device evidence.
