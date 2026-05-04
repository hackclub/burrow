# `BEP-0005` - Daemon IPC and Apple Boundary

```text
Status: Draft
Proposal: BEP-0005
Authors: gpt-5.4
Coordinator: gpt-5.4
Reviewers: Pending
Constitution Sections: II, III, IV, V
Implementation PRs: Pending
Decision Date: Pending
```

## Summary

Burrow should formalize one Apple/runtime boundary: Apple clients speak only to the daemon over gRPC on the app-group Unix socket, and the daemon owns all external control-plane, helper-process, and runtime coordination work. This prevents UI code from accreting side HTTP paths or ad hoc control-plane integrations that bypass the system Burrow is supposed to own.

## Motivation

- The current Tailnet work already showed the failure mode: Swift UI code started reaching around the daemon boundary to talk to helper HTTP endpoints directly.
- Apple-specific process ownership is easy to blur between the app, the network extension, and helper daemons unless the contract is explicit.
- If Burrow wants a durable multi-runtime architecture, the daemon must remain the only orchestration boundary between clients and control/data-plane behavior.

## Detailed Design

- Apple UI and Apple support libraries may call only daemon gRPC methods over the declared Burrow Unix socket.
- Direct Swift calls to external control-plane HTTP APIs, localhost helper HTTP servers, or runtime-specific subprocesses are forbidden.
- The daemon is responsible for:
  - discovery of Tailnet authorities and related metadata
  - control-plane session setup and tracking
  - login/session lifecycle brokering
  - runtime start/stop/reconcile
  - translating helper or bridge processes into stable daemon RPCs
- `burrow/src/control/` owns transport-neutral control-plane semantics such as discovery, authority normalization, and request/response shaping.
- Apple UI owns presentation only:
  - forms
  - local state
  - presenting returned auth URLs or statuses
  - surfacing daemon availability and errors
- Any new Apple-facing runtime capability requires a daemon RPC first.

## Security and Operational Considerations

- Keeping control-plane I/O out of Swift UI reduces accidental secret, token, and callback sprawl across app code.
- The daemon boundary makes testing and kill-switch behavior tractable because runtime integration is localized.
- Apple daemon lifecycle ownership must be explicit: either the app ensures the daemon is running before RPC or the extension owns it and the UI surfaces daemon-unavailable state clearly.
- Non-Apple presentation clients should follow the same daemon-first lifecycle pattern: connect to a managed daemon when present, or start a user-scoped embedded daemon before issuing RPCs, without adding platform-local control-plane paths.

## Contributor Playbook

- Before adding a new Apple-side workflow, identify the daemon RPC that should own it.
- If the RPC does not exist, add the protocol shape in `proto/burrow.proto`, implement it in the daemon, and only then wire Swift UI.
- Verify that no Swift UI or support code calls external control-plane HTTP endpoints directly.
- For Tailnet and similar flows, test:
  - daemon unavailable behavior
  - successful RPC path
  - error propagation through the UI
- Keep Linux GTK and Apple clients visually and functionally aligned around the same daemon-backed home surface: Networks, Accounts, Tunnel, and add flows should remain corresponding views over the daemon API.

## Alternatives Considered

- Let Apple UI call control-plane endpoints directly for convenience. Rejected because it creates parallel orchestration paths and breaks the daemon contract.
- Allow one-off exceptions for login helpers. Rejected because those exceptions become the architecture.

## Impact on Other Work

- Governs the Tailnet refactor and future Apple runtime work.
- Governs Linux GTK daemon startup parity where the same daemon API is reused from a user-scoped presentation process.
- Interacts with BEP-0002 control-plane bootstrap and BEP-0003 transport refactoring.

## Decision

Pending.

## References

- `Apple/UI/`
- `Apple/Core/`
- `Apple/NetworkExtension/`
- `burrow/src/daemon/`
- `burrow/src/control/`
