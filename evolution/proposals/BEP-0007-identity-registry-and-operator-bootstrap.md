# `BEP-0007` - Identity Registry and Operator Bootstrap

```text
Status: Draft
Proposal: BEP-0007
Authors: gpt-5.4
Coordinator: gpt-5.4
Reviewers: Pending
Constitution Sections: II, III, IV, V
Implementation PRs: Pending
Decision Date: Pending
```

## Summary

Burrow should maintain one canonical registry for project identities, aliases, bootstrap users, SSH keys, and admin-group mappings. Forgejo, Authentik, and related bootstrap configuration should derive from that registry instead of hardcoding overlapping identity facts in multiple modules.

## Motivation

- Burrow currently hardcodes operator and admin/bootstrap user facts directly in host configuration.
- Multi-account and self-hosted identity are becoming core architecture, not incidental infra details.
- A single registry reduces drift across Forgejo, Authentik, Headscale, SSH authorization, and future control-plane bootstrap.

## Detailed Design

- Add a root-level identity registry (`contributors.nix`) as the canonical source of truth for:
  - usernames
  - display names
  - canonical emails
  - external source emails or aliases
  - admin scope
  - bootstrap eligibility
  - forge authorized SSH keys
  - named roles
- Consume that registry from host configuration for:
  - Forgejo authorized keys
  - Forgejo bootstrap admin defaults
  - Authentik bootstrap users
  - Burrow user/admin group names
- Future work may derive contributor docs, OIDC bootstrap, and additional runtime configuration from the same registry.

## Security and Operational Considerations

- Identity drift is a security bug when it affects admin groups, bootstrap accounts, or SSH authorization.
- The registry stores metadata only; secrets remain in agenix or other declared secret paths.
- Changes to the registry should receive explicit review because they affect access and governance.

## Contributor Playbook

- Edit `contributors.nix` first when changing operator, admin, alias, or bootstrap identity state.
- Derive runtime configuration from the registry instead of duplicating the same facts elsewhere.
- Keep secret references separate from identity metadata.

## Alternatives Considered

- Continue hardcoding users in module options. Rejected because drift is inevitable once Forgejo, Authentik, and Headscale all depend on the same identities.
- Create separate per-service user lists. Rejected because it duplicates governance facts and weakens review.

## Impact on Other Work

- Supports forge auth, Authentik group sync, and future multi-account Burrow control-plane work.
- Creates the basis for stronger contributor and operator provenance later.

## Decision

Pending.

## References

- `contributors.nix`
- `nixos/hosts/burrow-forge/default.nix`
- `nixos/modules/burrow-authentik.nix`
- `nixos/modules/burrow-forge.nix`
