# Verify a portable pilot download

From a source checkout on Windows, select the native task environment and supply
the ZIP path, reviewed release checksum and exact source commit:

```powershell
$env:MISE_ENV = 'windows'
mise run verify-pilot-package -- -Archive 'PATH_TO_DOWNLOADED_ZIP' -ExpectedSha256 'ebb89a8234d2c5efeb7c8cc2432730441763f3093fd06d08c73bc90db41cd37a' -ExpectedCommit '33adccf7dafa435773c018f54a1b8fc40bcd4b0f'
```

Those values identify the 2026-10-29 draft candidate. For another candidate,
use its separately reviewed release hash and commit. Do not derive the expected
hash from the archive's own manifest. The script can also be invoked directly
as `.\scripts\verify-pilot-package.ps1` with the same named arguments.

The verifier reads one open file stream for both hashing and ZIP inspection.
It checks the external SHA-256, exact versioned root and 14-file inventory,
duplicate/traversal paths, all 13 manifest hashes, source/target metadata and the
four x64 PE headers. It neither extracts files nor launches packaged programs.
It writes no application data and needs no device, startup, registry or network
access. A failure exits nonzero.

This establishes static package integrity against the caller's reviewed values.
It does not establish executable behavior, signing or mouse compatibility.

## Validation — 2026-10-29

The frozen draft ZIP passed in Windows PowerShell 5.1 and PowerShell 7. Both
versions rejected a bad expected hash, a wrong commit, a modified manifest and
a traversal fixture, each with exit code 1. Fixtures were temporary; the original
ZIP and GitHub draft assets were preserved. Independent review identified the
earlier hash/reopen race; the corrected script holds the same stream throughout.
