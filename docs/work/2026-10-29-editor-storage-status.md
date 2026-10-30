# Truthful editor storage status — 2026-10-29

## Observed issue and correction

Automated isolated Windows tests on the earlier c05b478 editor demonstrated
clear corrupt/schema99 storage errors, disabled editing/Apply and unchanged
fixture bytes after Retry. Both screens nevertheless retained NEW DRAFT in
the header. That label could suggest an editable draft exists when loading
failed; it is retained as the original failed usability checkpoint.

The editor now prioritizes STORAGE UNAVAILABLE when there is no editor session.
It retains the existing theme and normal unsaved/saved/new labels. No storage,
validation, protocol, production tray or device behavior is changed.

## Regression and software checks

The new headless test uses malformed JSON and schema99 fixtures at 850 x 640:
it verifies the unavailable badge, absence of NEW DRAFT/profile editing,
disabled Save/Apply and original bytes after repeated Retry. A separate
external fixture repair then verifies Retry recovers a normal NEW DRAFT and
enabled Save. It does not simulate an editor reset or overwrite a failed file.

Local verify-linux passes the core/feature suites and now 14 editor tests.
Strict default/combined-preview Windows cross-lint and the editor release
cross-build pass. Independent source review found no material findings; actual
theme/long-label containment remains a separate Windows check.

New editor SHA-256:
b379f09517d80853ed097daf74594d6806e146fd62f02b9571956bccc95d83dd.
A separate Windows Computer Use recheck passed with this hash at 1080 x 790
and 850 x 640 client sizes. Corrupt and future stores show the new badge,
fully contained without observed overlap/clipping, actionable error/Retry,
disabled editing/Save/Apply and no Profile name field. Retry preserved the
blocked state and both original fixture hashes. A fresh empty root instead
showed normal NEW DRAFT, editable fields, enabled Save and disabled Apply;
it remained empty because nothing was saved. All owned processes closed.

An extra resize drag reported outside-bounds after minimum size had already
been reached; a fresh capture confirmed the minimum client dimensions.
This is retained as a test-operation failure, not an established app defect.
Earlier clipboard/CRUD evidence belongs to the prior hash and is not silently
reassigned to this binary.

## Additional isolated evidence

The earlier editor's extended Windows QA passed user-confirmed scratch Discard,
independent app-field Ctrl+C/Ctrl+V/reopen, stable duplicate/rename identities,
normal/minimum-size validation, and corrupt/future error/Retry preservation.
These are automated observations, not owner physical keyboard, IME composition,
screen-reader speech, actual monitor-scale or high-contrast results. All those
owned windows closed and previous fixtures/evidence were preserved.

No production tray, installation, startup, Synapse or mouse/device action was
performed. Invalid/future fixture bytes and raw reports remain private. Custom
Apply and production/hardware gates remain unchanged.
