# Cloud Project access handoff

Prepared 2026-09-30. Local Project access works; the failing Cloud task has not
been inspected. Empty OAuth scopes and an integration error suggest credential
selection/substitution as a hypothesis, not proof of a missing PAT scope.

Identify the actual environment/task first. Current Cloud network secrets use
HTTPS placeholders substituted for allowed destinations during setup and tasks.
Legacy setup secrets are removed before the agent phase. Use the existing
credential; do not widen permissions or copy its value into scripts/logs. For the
current experience, verify the requested `GH_TOKEN` network secret targets
`api.github.com`, has no same-key direct variable, and is tested in a new task
after publishing the setup. GitHub GraphQL also needs POST access.

Read-only task checks (do not print tokens or raw HTTP traces):

```sh
gh api user --jq .login
gh api graphql -f query='query { user(login:"spencer-life") { projectV2(number:5) { id } } }' --jq '.data.user.projectV2 != null'
```

Project read success does not prove write permission. Separate any later
write test into an explicitly authorized temporary item with cleanup. Do not
disable hosted CI or change required checks based on Cloud read success.
Cloud Linux checks cannot execute native Windows UI or mouse tests.

Official OpenAI documentation: [current Cloud configuration](https://learn.chatgpt.com/docs/environments/cloud-environments#configure-environment-variables-and-network-secrets)
and [legacy environment secrets](https://learn.chatgpt.com/docs/environments/cloud-environment#environment-variables-and-secrets).
