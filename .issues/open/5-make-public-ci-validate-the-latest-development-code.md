---
title: Make public CI validate the latest development code
labels:
    - codex-created
    - enhancement
state: open
state_reason: ""
synced_at: 2026-09-30T03:20:21.107116521Z
info:
    author: spencer-life
    created_at: 2026-09-29T18:22:39Z
    updated_at: 2026-09-29T21:45:06Z
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

## Checkpoint — 2026-09-29

Public hosted Core Linux and Windows CI passed at sanitized PR #3 head `7cb0440` ([run](https://github.com/spencer-life/viperpilot/actions/runs/36612610509)); Security also passed ([run](https://github.com/spencer-life/viperpilot/actions/runs/36612610273)). `main` currently has no required checks or rulesets, so defining checks and validating the stacked PR merge order remains open.


## Enforcement preparation — 2026-09-29

Hosted Linux/Windows Core checks passed at `1db3cc7` ([run](https://github.com/spencer-life/viperpilot/actions/runs/36660537944)); Security also passed ([run](https://github.com/spencer-life/viperpilot/actions/runs/36660538070)). These results predate the catalog slice.

Core job names are aligned with foundation PR #1: `ci (ubuntu-24.04)` and `ci (windows-2025)`, so a main-only requirement can serve both stacked PRs. Workflow triggers, native host/local tasks, checkout behavior, stable contexts and staged merge order are recorded in [CI and merge gates](https://github.com/spencer-life/viperpilot/blob/codex/sanitized-quick-switch/docs/ci-and-merge-gates.md). Security must be added as a requirement after #1 merges and before #3 is merged; #1's historical head does not emit it. No merge queue or merge is enabled.

A minimal two-Core-check ruleset was prepared and independently reviewed, with requirements bound to the observed GitHub Actions integration, no bypass actors, and a captured empty-rule rollback baseline. Automatic approval review rejected activating it because explicit authorization is required for the persistent settings change and its ability to block main updates. No repository settings changed; authorization is pending. This issue remains open.
