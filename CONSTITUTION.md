# Burrow Constitution

1. Mission

Burrow exists to build a proper VPN: fast, inspectable, deployable on infrastructure the project controls, and legible enough that future contributors can extend it without guesswork.

2. Commitments

- Protocol work must favor correctness over novelty. Burrow does not claim support for a transport or control-plane feature until the wire format, state handling, and recovery behavior are implemented and tested.
- Security is a design constraint, not a cleanup phase. Key material, bootstrap credentials, control-plane tokens, and routing policy must have explicit storage and rotation paths.
- Performance matters. Burrow should avoid needless copies, hidden blocking, and ad hoc process graphs that make packet forwarding or control-plane convergence harder to reason about.
- Source, infrastructure, and release logic live in the repository. If the forge cannot be rebuilt from the tree, the work is incomplete.
- Non-trivial changes require a Burrow Evolution Proposal. Durable rationale belongs in the repository, not only in chat.

3. Infrastructure

Burrow controls its own forge, runners, deployment automation, and edge configuration for `burrow.net` and `burrow.rs`.

- Dedicated compute is preferred over SaaS dependencies when the dependency would hold release, source, or identity authority.
- Secrets may be bootstrapped from local intake for initial bring-up, but long-lived operation must converge on encrypted, versioned secret handling.
- Production access must be attributable. Automation identities, SSH keys, and service accounts must be named and documented.

4. Contributors

- Read this constitution before drafting product, protocol, or infrastructure changes.
- Capture intent, testing expectations, and rollback procedures in proposals.
- Prefer reversible migrations. If a change is destructive, document the preconditions and teardown plan first.
- Security-sensitive work requires explicit reviewer attention, even when the implementation is performed by an agent.

5. Governance

- Burrow Evolution Proposals (BEPs) are the primary design record for architectural, protocol, forge, and deployment changes.
- Accepted proposals are authoritative until superseded.
- Constitutional changes require a dedicated proposal that quotes the affected text and records the decision.

6. Origin

Burrow started as a firewall-burrowing client and now carries its own transport, daemon, mesh, and control-plane work. This constitution exists so the project can finish that evolution coherently.
