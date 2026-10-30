# Explicit startup changes — 2026-10-29

Status refresh previously called the startup registry writer before reading the
device. A false preference could delete the development Run value; a true one
could recreate or repair it. Remove that implicit mutation. The explicit
SetStartWithWindows command remains the only caller of the registry writer.

Automatic crash/moved-executable repair is deliberately removed. An authorized
owner must explicitly reapply the preference if repair is needed. The schema,
stored preference, registry identity, installer and ordinary launch gate remain
unchanged. No registry operation, production launch or mouse operation was run.

Revert this source commit to restore the old behavior; no machine setting needs
rollback. Native Windows core and preview CI passed, including the subsequent
pilot guard tests. WSL execution timed out, so a native local source clone was
used without restarting WSL or disturbing the original checkouts.
