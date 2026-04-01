# `BEP-0003` - CONNECT-IP and Negotiation Roadmap

```text
Status: Draft
Proposal: BEP-0003
Authors: gpt-5.4
Coordinator: gpt-5.4
Reviewers: Pending
Constitution Sections: I, II, V
Implementation PRs: Pending
Decision Date: Pending
```

## Summary

Burrow should grow from a WireGuard-first tunnel runner into a transport stack that can support HTTP/3 MASQUE `CONNECT-IP` and a richer node negotiation model. This proposal stages that work so Burrow can adopt the right abstractions instead of stapling QUIC-era semantics onto a WireGuard-only daemon.

## Motivation

- `CONNECT-IP` introduces HTTP/3 sessions, context identifiers, address assignment, and route advertisements that do not fit the current daemon model.
- A Tailscale-style control plane requires explicit node, endpoint, and session state rather than raw network blobs.
- The project needs a roadmap that distinguishes data-model work, control-plane work, and actual transport implementation.

## Detailed Design

- Stage 1: land control-plane types and persistent auth/session/node storage.
- Stage 2: add transport-agnostic route, address-assignment, and policy abstractions in Burrow.
- Stage 3: implement MASQUE `CONNECT-IP` framing and HTTP Datagram handling.
- Stage 4: connect the transport layer to real relay, policy, and observability paths.

## Security and Operational Considerations

- `CONNECT-IP` changes the trust boundary from WireGuard peers to HTTP/3 peers and relays; authentication, replay handling, and scope restriction must be explicit.
- Route advertisements and delegated prefixes must be validated before touching the data plane.
- Control-plane capability claims must not imply support that the transport layer does not yet implement.

## Contributor Playbook

- Keep protocol codecs independently testable before integrating them into live transports.
- Add interoperability tests for every new capsule or datagram type.
- Separate request parsing, policy validation, and packet forwarding so regressions stay localized.

## Alternatives Considered

- Implement MASQUE directly in the daemon without control-plane refactoring. Rejected because the current daemon has no transport-neutral contract for routes or prefixes.
- Treat Tailscale negotiation as a one-off compatibility shim. Rejected because Burrow needs first-class control-plane concepts either way.

## Impact on Other Work

- Depends on BEP-0002.
- Informs future relay, policy, and node coordination work.

## Decision

Pending.

## References

- RFC 9484
- `burrow/src/daemon/`
- `burrow/src/control/`
