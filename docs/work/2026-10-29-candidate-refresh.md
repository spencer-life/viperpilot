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
CLI help. The UI runtime source is unchanged from the independently reviewed native
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

## Delivered draft and artifact evidence

The [refreshed draft](https://github.com/spencer-life/viperpilot/releases/tag/untagged-4160b6d899496f6c5e90)
is pinned to source `2df219c66db0a9b67a42f99b549b69488b4f1711`, tagged
`pilot-2026-10-29-2df219c66db0`. It remains draft/prerelease. ZIP name:
`ViperPilot-pilot-2df219c66db0-windows-x64.zip`, 4,606,984 bytes. Its external
SHA-256 is
`e019e7d6b903b9c4b5b94e0652987a9d83a9ad92dc2c5b3f4f5755bd65142f30`;
GitHub's uploaded asset size and digest match. The original frozen ZIP remains
byte-identical and its draft assets were not replaced.

The fresh ZIP passed static verification in PowerShell 5.1 and 7: 14 entries,
13 manifest hashes, exact source/target and four x64 executables. Its actual
packaged native preview passed all three existing harness scenarios with
exit 0. Packaged CLI help and offline snapshot inspection also passed.

A separate Computer Use smoke verified the packaged native preview hash
`7354c59e2ec4364524c4de7ab3364b8e2ad25f88341cd640bc2b47118912a5b0`
and editor hash
`ae1c883094072a4a68e38cb2277c07018b0badd75a090bda2b1e0fbe3c853afe`.
Initial native compact containment/preview labeling and editor new-draft
rendering/UIA were observed. Input attempts failed with the tool error
`failed to activate captured window`, twice per app with refreshed state.
Thus the fresh packaged input/Save-reopen smoke was not completed; this is a
tool/session blockage, not evidence of an app defect. Only exact owned staged
processes were force-stopped after path checks (reported exit -1). Do not
describe those as clean exits or transfer older hash-specific desktop results.

The private real-device review packet binds the exact packaged CLI executable
hash and source. It proposes separately approved enumeration/GET capture only;
the original baseline's provenance, live identity comparison, isolated staging
and all actual write/recovery plans remain pending. No hardware command was run.
