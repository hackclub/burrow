# WireGuard Rust Lineage

Burrow's in-tree WireGuard engine is not a greenfield implementation. It was lifted from the Rust WireGuard lineage around Cloudflare's BoringTun, then cut down and reshaped to fit Burrow's own daemon and tunnel abstractions.

## What Was Lifted

- The repository history includes `1b39eca` (`boringtun wip`) and `28af9003` (`merge boringtun into burrow`).
- The current `burrow/src/wireguard/noise/*` files still carry the original Cloudflare copyright and SPDX headers.
- Core protocol machinery such as the Noise handshake, session state, rate limiter, and timer logic came from that imported body of work.

## What Changed in Burrow

Burrow does not embed BoringTun unchanged.

- The original device layer was replaced with Burrow-specific interface and peer control blocks in `burrow/src/wireguard/iface.rs` and `burrow/src/wireguard/pcb.rs`.
- Configuration handling was rewritten around Burrow's own INI parser and config model in `burrow/src/wireguard/config.rs`.
- The daemon now resolves the active runtime from the database-backed network list rather than from a single static WireGuard payload.
- Burrow added its own runtime switching path so WireGuard can share one daemon lifecycle with the rest of the managed runtime system.

## What Was Improved

The lifted code has been tightened further in-repo.

- Deprecated constant-time comparisons were replaced with `subtle`.
- Network ordering and runtime selection are now deterministic and test-covered.
- The Burrow runtime can swap between WireGuard configurations without restarting the daemon process itself.

## Why This Matters

This project should be explicit about lineage. Burrow benefits from proven Rust WireGuard work, but it owns the integration surface, runtime behavior, and future maintenance burden. That is why the code should be documented as lifted, modified, and improved rather than described as wholly original.
