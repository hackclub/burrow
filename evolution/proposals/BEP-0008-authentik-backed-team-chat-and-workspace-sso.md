# `BEP-0008` - Authentik-Backed Team Chat and Workspace Identity

```text
Status: Draft
Proposal: BEP-0008
Authors: gpt-5.4
Coordinator: gpt-5.4
Reviewers: Pending
Constitution Sections: II, III, V
Implementation PRs: Pending
Decision Date: Pending
```

## Summary

Burrow should add a self-hosted team chat surface at `chat.burrow.net` and
continue the project-wide move toward Authentik as the identity authority for
external work systems. The immediate targets are a self-hosted Zulip
deployment rooted in Authentik SAML, a Linear SAML configuration when the
workspace plan supports it, and a 1Password Unlock-with-SSO deployment rooted
in the same Authentik-backed OIDC authority.

This keeps Burrow's day-to-day coordination surfaces aligned with the same
admin groups, canonical users, and secret-handling model already used for
Forgejo, Headscale, and Tailscale. It also avoids fragmenting login state
across vendor-native Google auth flows when Burrow already operates an IdP.

## Motivation

- Forge, Tailnet, operator identity, and Tailscale custom OIDC are already
  rooted in Authentik. Team chat, work tracking, and password-manager access
  should not become separate authority islands.
- Zulip provides a self-hosted chat system under Burrow's control, which fits
  the constitution better than adding another hosted chat dependency.
- Linear remains a SaaS dependency, but its workspace access should still be
  derived from Burrow-managed identities and domains when the vendor plan
  exposes SAML configuration.
- 1Password Business is another external work surface where Burrow-controlled
  identities are preferable to vendor-native Google-only auth. Its current
  vendor flow is OIDC-based Unlock with SSO rather than SAML, so the proposal
  needs to preserve protocol accuracy instead of flattening everything into
  one SAML bucket.
- Burrow already has a canonical public identity registry and a secret-backed
  external-email alias map. Reusing that structure is lower-risk than
  inventing per-app user bootstrap logic.

## Detailed Design

- Add a Burrow-managed Zulip workload on the forge host at `chat.burrow.net`.
  The deployment should be repo-owned and rebuildable from Nix, even if the
  runtime uses vendor-supported container images internally.
- Zulip should authenticate through Authentik SAML rather than local passwords
  as the primary path. Initial bootstrap may still keep an operational escape
  hatch while the deployment is being validated.
- Add Authentik-managed SAML applications for:
  - Zulip at `chat.burrow.net`
  - Linear using Burrow's claimed domains and Authentik metadata
- Add an Authentik-managed SCIM backchannel for Linear so Burrow can push
  role groups declaratively instead of hand-maintaining workspace roles.
- Add an Authentik-managed OIDC application for 1Password Business under the
  Burrow team sign-in address.
- Treat Zulip and Linear as downstream applications of the same identity
  authority, and treat 1Password as part of that same authority even though
  its vendor protocol is OIDC rather than SAML. The source of truth remains:
  - public identities and admin intent in `contributors.nix`
  - private alias mappings and external accounts in agenix-encrypted secrets
- Keep app-specific configuration in dedicated reconciliation code or module
  options instead of hand-edited UI state.
- Prefer service-specific reconciliation over ad hoc manual setup so rebuilds
  and host replacement converge automatically.
- Derive Linear SCIM role groups from Burrow's canonical identity metadata.
  If Burrow-wide admin intent says a user is an operator/admin, the repo-owned
  configuration should map that intent onto the Linear push group without a
  second manual roster.
- Model 1Password according to the vendor's actual integration contract:
  - OIDC Authorization Code Flow with PKCE
  - public client rather than a confidential client
  - no Burrow-side dependence on a stored client secret unless the vendor flow
    changes

## Security and Operational Considerations

- Do not store external personal email mappings in public registry files.
  Public tree data may include Burrow usernames and canonical `@burrow.net`
  addresses, but external aliases must stay in encrypted secrets.
- Zulip internal service credentials, Django secret material, and any mail
  credentials must have explicit storage and rotation paths.
- Linear SAML must not become Burrow's only admin recovery path. At least one
  owner login path outside the enforced SAML flow should remain available until
  rollout is proven.
- Linear SCIM group push should be role-scoped and explicit. Burrow should
  avoid blanket ownership mapping unless that intent is recorded in the repo.
- 1Password Owners cannot be forced onto Unlock with SSO during initial setup.
  Burrow should preserve the owner recovery path and treat OIDC rollout as a
  scoped migration for non-owner users first.
- If Zulip is deployed without production-grade outbound email at first, that
  limitation must be documented and treated as an operational constraint, not a
  hidden assumption.
- Rollback should be straightforward:
  - disable or stop the Zulip module
  - remove the Authentik SAML apps
  - remove the Authentik OIDC app used for 1Password if necessary
  - leave the underlying Burrow identities unchanged

## Contributor Playbook

- Define the app and identity intent in the repository before modifying the
  forge host.
- Add or update Nix modules so `burrow-forge` can rebuild Zulip and the
  corresponding Authentik SAML configuration from the tree.
- Verify:
  - `chat.burrow.net` serves a working Zulip login surface
  - Authentik exposes working metadata for Zulip and Linear
  - Authentik exposes a working OIDC issuer for 1Password
  - users in Burrow admin groups receive the expected access on first login
- Record concrete evidence for:
  - host deployment generation
  - Authentik reconciliation success
  - Zulip login success
  - Linear SAML configuration state
  - 1Password Unlock with SSO configuration state

## Alternatives Considered

- Use Zulip Cloud instead of self-hosting. Rejected because the ask is to host
  chat under `chat.burrow.net`, and Burrow already operates a forge host with a
  self-managed identity plane.
- Keep Linear on Google-native login. Rejected because it leaves Burrow work
  access outside the project's operator and group model.
- Treat 1Password as a SAML app for consistency. Rejected because the live
  vendor flow is OIDC and Burrow should not pretend otherwise in repo-owned
  infrastructure.
- Add per-app manual Authentik configuration without repository automation.
  Rejected because it violates Burrow's infrastructure-in-repo commitment.

## Impact on Other Work

- Extends Burrow's Authentik role from control-plane identity into team-work
  surfaces.
- Introduces a persistent chat workload on the forge host, with resource and
  monitoring implications.
- Creates a likely follow-up for SCIM or richer group synchronization if Linear
  or Zulip role mapping needs to become fully declarative later.
- Adds a second OIDC relying party beyond Forgejo, Headscale, and Tailscale,
  which raises the importance of keeping Burrow's Authentik scope mappings and
  redirect handling consistent across applications.

## Decision

Pending.

## References

- `CONSTITUTION.md`
- `contributors.nix`
- `evolution/proposals/BEP-0004-hosted-mail-and-saas-identity.md`
- Authentik docs: SAML provider and metadata endpoints
- Zulip docs: SAML authentication and docker deployment
- Linear docs: SAML and access control
- 1Password docs: Unlock with SSO using OpenID Connect
