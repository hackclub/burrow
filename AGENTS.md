# instructions for agents

1. Spell the project name as `Burrow` in user-facing copy and `burrow` in code, package, and protocol identifiers unless an existing integration requires a different literal.
2. Read [CONSTITUTION.md](CONSTITUTION.md) before changing Apple clients, the daemon, the control plane, forge infrastructure, identity, or security-sensitive code.
3. Anchor non-trivial changes in a Burrow Evolution Proposal (BEP) under [evolution/](evolution/README.md) so future contributors can inherit the rationale, safeguards, and rollout shape.
4. Before touching the Apple app, daemon IPC, or Tailnet flows, review:
   - [evolution/proposals/BEP-0002-control-plane-bootstrap-and-local-auth.md](evolution/proposals/BEP-0002-control-plane-bootstrap-and-local-auth.md)
   - [evolution/proposals/BEP-0003-connect-ip-and-negotiation-roadmap.md](evolution/proposals/BEP-0003-connect-ip-and-negotiation-roadmap.md)
   - [evolution/proposals/BEP-0005-daemon-ipc-and-apple-boundary.md](evolution/proposals/BEP-0005-daemon-ipc-and-apple-boundary.md)
   - [evolution/proposals/BEP-0006-tailnet-authority-first-control-plane.md](evolution/proposals/BEP-0006-tailnet-authority-first-control-plane.md)
5. Apple clients must talk only to the daemon over gRPC. Do not add direct HTTP, control-plane, or helper-process calls from Swift UI code.
6. Treat Tailnet as one protocol family. Tailscale-managed and self-hosted Headscale-style deployments differ by authority, policy, and auth details, not by a separate user-facing protocol surface.
7. Maintain canonical identity and operator metadata in [contributors.nix](contributors.nix). If Burrow forge, Authentik, Headscale, or admin/group mappings need to change, edit that registry first and derive runtime configuration from it.
8. When process or architecture is unclear, stop and draft or update a BEP instead of improvising durable behavior in code.
