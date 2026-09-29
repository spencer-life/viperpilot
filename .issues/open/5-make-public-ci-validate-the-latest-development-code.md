---
title: Make public CI validate the latest development code
labels:
    - codex-created
    - enhancement
state: open
state_reason: null
synced_at: 2026-09-29T18:28:48.181853044Z
---

## Context

The public repository is becoming the main development repository. Its checks need to validate the current development branch and pull requests, including the latest ported implementation. This is separate from historical private-repository Actions quota or runner incidents.

Implementation baseline: [public PR #3](https://github.com/spencer-life/viperpilot/pull/3) (draft).

## Done when

- After the current public PR checks pass, define and enforce the required status checks for public `main` without creating a circular dependency in the stacked PR merge order. Record the required check names and verify that the stacked PRs can merge in order. Until then, branch protection is an open gate.

- Document which workflows run on pushes, pull requests, and the default branch, and which host platforms each job covers.
- Ensure pull request checks target the checked-out PR commit and include the required formatting, tests, and supported Windows build checks.
- Provide a reproducible local command for each required job, and keep hardware-dependent experiments out of unattended CI.
- Run the checks against the current development code and record the resulting workflow URL and commit in the public repository.
- Keep any credentials, device identifiers, local paths, and full hardware evidence out of workflow logs and artifacts.
