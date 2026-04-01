# `BEP-0001` - Sovereign Forge and Governance Bootstrap

```text
Status: Draft
Proposal: BEP-0001
Authors: gpt-5.4
Coordinator: gpt-5.4
Reviewers: Pending
Constitution Sections: II, III, V
Implementation PRs: Pending
Decision Date: Pending
```

## Summary

Burrow should own its forge, deployment logic, and operational context under `burrow.net`. This proposal establishes the repository-local governance and forge bootstrap required to move build, release, and infrastructure control out of GitHub-centric assumptions and into a self-hosted operating model.

## Motivation

- The repository currently keeps CI definitions under `.github/workflows/` but has no first-class self-hosted forge layout.
- Infrastructure changes and protocol work are already entangled; without a design record, the project risks landing irreversible operations without enough context.
- A self-hosted forge is a prerequisite for durable autonomy over source, runners, and release pipelines.

## Detailed Design

- Add a project constitution and BEP process under `evolution/`.
- Introduce a Nix flake and NixOS host/module layout for `burrow-forge`.
- Add Forgejo-native workflows under `.forgejo/workflows/` for repository-local CI.
- Bootstrap the initial forge identity around `contact@burrow.net` and an agent-owned SSH workflow.

## Security and Operational Considerations

- Initial bootstrap may read credentials from local intake, but production must converge on encrypted secret handling.
- The first forge host replacement must preserve rollback information before deleting any existing VM.
- DNS for `burrow.net` is currently pending activation; the forge rollout must not assume public reachability until nameserver cutover completes.

## Contributor Playbook

- Keep destructive host operations behind explicit verification of the current Hetzner state.
- Build and test repository-local workflows before using them for deployment.
- Record the active server id, image, IPs, and SSH path before replacement.

## Alternatives Considered

- Continue relying on GitHub Actions while separately hosting services. Rejected because it leaves source authority and CI policy split across systems.
- Stand up Forgejo without a repository-local operating model. Rejected because the repo would still be missing deployment truth.

## Impact on Other Work

- Blocks long-term migration of workflows away from GitHub.
- Provides the governance anchor for protocol and control-plane proposals.

## Decision

Pending.

## References

- `CONSTITUTION.md`
- `.github/workflows/`
- `.forgejo/workflows/`
