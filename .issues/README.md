# Public issue tracker

Issues in this directory are the Markdown source for GitHub issues in
[`spencer-life/viperpilot`](https://github.com/spencer-life/viperpilot/issues).
Run `gh-issue-sync init --owner spencer-life --repo viperpilot` once in each
checkout (local sync metadata is ignored by Git). Then use `gh-issue-sync pull`,
`gh-issue-sync status`, and `gh-issue-sync push` to keep the local files and
GitHub issues aligned.

Only sanitized planning and validation summaries belong here. Keep complete
serials, raw hardware reports, private repository links, local machine paths,
and unredacted screenshots in private local records. Physical-device issues
require the safety gates in `AGENTS.md`; creating or tracking an issue does not
authorize a hardware write.

These Markdown files mirror GitHub issues; they do not automatically synchronize with the private repository's backlog. Continuation notes for this sanitized mirror are in [the public branch record](https://github.com/spencer-life/viperpilot/blob/codex/sanitized-quick-switch/docs/continue-here.md). The private backlog keeps its own issue files.

## Preserve Project membership — 2026-09-30

Use `gh-issue-sync pull ISSUE --full` before editing an existing mirror. During
UI readiness tracking, an incremental pull skipped unchanged issues whose Project
membership had changed; pushing mirrors without that metadata removed two board
memberships. Both were restored and their mirrors refreshed with a full pull.
Preserve `projects` frontmatter and use a credential that can read the private
Project. Verify membership after a push, and stop on a conflict rather than
forcing stale metadata over the remote issue. Push only selected issues and use
`--no-comments` for body/metadata updates that must not post messages.
