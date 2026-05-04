# `BEP-0006` - Tailnet Authority-First Control Plane

```text
Status: Draft
Proposal: BEP-0006
Authors: gpt-5.4
Coordinator: gpt-5.4
Reviewers: Pending
Constitution Sections: I, II, IV, V
Implementation PRs: Pending
Decision Date: Pending
```

## Summary

Burrow should treat Tailnet as one protocol family. Tailscale-managed and self-hosted Headscale-style deployments differ by authority, policy, and auth details, not by a distinct user-facing protocol. Burrow’s config and UI should therefore be authority-first rather than provider-first.

## Motivation

- Splitting Tailscale and Headscale into separate user-facing providers causes fake architectural divergence.
- Discovery already naturally returns an authority and optional issuer; that is the stable contract users actually need.
- Future managed or enterprise deployments should fit the same model without requiring another protocol picker.

## Detailed Design

- Tailnet configuration is centered on:
  - account
  - identity
  - authority/login server URL
  - optional tailnet name
  - optional hostname
  - auth method/material
- User-facing surfaces should not force a protocol choice between Tailscale and Headscale.
- Provider inference may remain internal metadata for compatibility and diagnostics:
  - default managed Tailscale authority
  - custom self-hosted authority
  - Burrow-owned authority when explicitly applicable
- Discovery returns authority and related metadata; editing the authority is the mechanism that moves a configuration from managed default to custom control server.
- The daemon and control layer own provider inference; the UI should primarily present “Tailnet” plus the selected authority.
- Platform clients consume the same daemon gRPC surface for Tailnet discovery, authority probing, browser sign-in, and saved network payloads. macOS/iOS SwiftUI and Linux GTK may differ in presentation and local credential stores, but neither should introduce a second control-plane path.

## Security and Operational Considerations

- Authority-first config reduces UI complexity and makes misconfiguration easier to reason about.
- Provider-specific assumptions must not leak into packet or control-plane semantics unless the authority actually requires them.
- Auth material must remain authority-scoped and identity-scoped in daemon storage.

## Contributor Playbook

- Remove provider pickers from Tailnet UI unless a concrete protocol difference requires one.
- Store the authority explicitly in payloads and infer provider internally only when needed.
- Keep Linux GTK and Apple clients at functional parity by routing Tailnet add/discover/probe/login through `TailnetControl` and `Networks` RPCs instead of platform-local HTTP or legacy JSON daemon commands.
- Prefer tests that validate authority normalization and discovery behavior over UI-provider branching.

## Alternatives Considered

- Keep separate user-facing providers for Tailscale and Headscale. Rejected because it models deployment shape as protocol shape.
- Collapse all control planes into one opaque Burrow provider. Rejected because the authority still matters operationally and diagnostically.

## Impact on Other Work

- Refines BEP-0002’s Tailscale-shaped control-plane work.
- Constrains the Tailnet Apple and Linux GTK refactors plus future daemon control-plane storage.

## Decision

Pending.

## References

- `burrow/src/control/`
- `Apple/UI/Networks/`
- `burrow-gtk/src/`
- `proto/burrow.proto`
