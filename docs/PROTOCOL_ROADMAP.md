# Protocol Roadmap

Burrow currently has two tunnel paths in-tree:

- a WireGuard data plane
- a Tor-backed userspace TCP path

What it does not have yet is a transport-neutral control plane that can honestly claim full MASQUE `CONNECT-IP` or full Tailscale-style negotiation parity. This repository now contains the beginnings of that layer:

- control-plane data structures in `burrow/src/control/mod.rs`
- local auth bootstrap and persistent node/session storage in `burrow/src/auth/server/`
- governance documents under `evolution/` for the bigger protocol work

## `CONNECT-IP`

Full RFC 9484 support requires more than packet forwarding. It needs HTTP/3 session management, Capsule handling, HTTP Datagram context identifiers, address assignment, route advertisement, and request-scope enforcement. Burrow does not implement those end to end yet.

## Tailscale-Style Negotiation

Burrow now has register/map request and response types plus persistent node records, but it does not yet implement the full Tailscale capability surface, peer delta protocol, DERP coordination, or Noise-based control transport.

## Current Direction

The intended sequence is:

1. Stabilize the control-plane data model and bootstrap auth.
2. Introduce transport-neutral route and address abstractions.
3. Add MASQUE framing and HTTP/3 transport support.
4. Expand policy, relay, and interoperability testing.

This keeps Burrow honest about what is running today while creating a clean path for the rest.
