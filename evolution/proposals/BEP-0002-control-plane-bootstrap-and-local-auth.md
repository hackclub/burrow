# `BEP-0002` - Control-Plane Bootstrap and Local Auth

```text
Status: Draft
Proposal: BEP-0002
Authors: gpt-5.4
Coordinator: gpt-5.4
Reviewers: Pending
Constitution Sections: I, II, III, V
Implementation PRs: Pending
Decision Date: Pending
```

## Summary

Burrow needs a repository-owned control-plane model instead of ad hoc network payload storage plus third-party-only auth. This proposal introduces a local username/password bootstrap for `contact@burrow.net`, plus a register/map data model shaped to support a Tailscale-style control server without claiming full parity yet.

## Motivation

- Current auth support is limited and does not provide a plain local bootstrap path for the project's own operator identity.
- The existing database stores network payloads, but not a durable model for users, nodes, sessions, or control-plane negotiation state.
- Future work on route policy, device coordination, and richer negotiation needs a real data model now.

## Detailed Design

- Add control-plane types for users, nodes, register requests, and map responses.
- Extend the auth server schema with local credentials, sessions, provider logins, and control nodes.
- Expose JSON endpoints for local login, node registration, and map retrieval.
- Seed the initial operator account from intake-backed bootstrap credentials.

## Security and Operational Considerations

- Passwords are stored with Argon2id hashes only.
- Session tokens are bearer credentials and must be treated as sensitive.
- The bootstrap credential path is a short-term path; follow-up work should move it into encrypted secret management before public deployment.

## Contributor Playbook

- Verify bootstrap account creation in an isolated test database.
- Exercise login, register, and map end to end with integration tests.
- Do not advertise protocol parity beyond the implemented request/response contract.

## Alternatives Considered

- Wait for full external identity-provider integration first. Rejected because the forge needs an operator account now.
- Keep control-plane state implicit in daemon-local configuration. Rejected because it cannot express multi-device coordination.

## Impact on Other Work

- Unblocks forge bootstrap and future device control-plane work.
- Creates the storage model needed for richer policy and transport proposals.

## Decision

Pending.

## References

- `burrow/src/auth/server/`
- `burrow/src/control/`
