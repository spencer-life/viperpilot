# 2026-09-29: the owner keeps the legacy installation in daily use.
# This development branch must not replace its binaries, shortcuts or startup.
# Keep the original installer in Git history; no replacement installation ran.
$ErrorActionPreference = 'Stop'
throw 'Development installation is disabled to preserve your current app. Build only; see docs/development-isolation.md. No files or processes were changed.'
