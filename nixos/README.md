# Burrow Forge Runbook

This directory contains the Burrow forge host definition and the Hetzner bootstrap shape for `burrow-forge`.

Mail hosting is intentionally not part of this NixOS host in the current plan. Burrow's first mail path is Forward Email with Burrow-owned custom S3 backups; see [`docs/FORWARDEMAIL.md`](../docs/FORWARDEMAIL.md).

## Files

- `hosts/burrow-forge/default.nix`: host entrypoint
- `modules/burrow-forge.nix`: Forgejo, Caddy, PostgreSQL, and admin bootstrap module
- `modules/burrow-forge-runner.nix`: Forgejo Actions runner and agent identity bootstrap
- upstream `compatible.systems/conrad/nsc-autoscaler`: Namespace-backed ephemeral Forgejo runner module consumed via the Burrow flake input
- `modules/burrow-authentik.nix`: minimal Authentik IdP for Burrow control planes
- `modules/burrow-headscale.nix`: Headscale control plane rooted in Authentik OIDC
- `modules/burrow-namespace-portal.nix`: small admin portal for forge-owned Namespace authentication and NSC token refresh
- `../secrets.nix`: agenix recipient map for tracked Burrow forge secrets
- `hetzner-cloud-config.yaml`: desired Hetzner host shape
- `keys/contact_at_burrow_net.pub`: initial operator SSH public key
- `keys/agent_at_burrow_net.pub`: automation SSH public key
- `../Scripts/hetzner-forge.sh`: Hetzner inventory and replace workflow
- `../Scripts/nsc-build-and-upload-image.sh`: temporary Namespace builder -> raw image -> Hetzner snapshot
- `../Scripts/bootstrap-forge-intake.sh`: copy the Forgejo bootstrap password and agent SSH key into `/var/lib/burrow/intake/`
- `../Scripts/check-forge-host.sh`: verify Forgejo, Caddy, the local runner, optional NSC services, and optional Tailnet services after boot
- `../Scripts/cloudflare-upsert-a-record.sh`: upsert DNS-only Cloudflare `A` records for Burrow host cutovers
- `../Scripts/forge-deploy.sh`: remote `nixos-rebuild` entrypoint for the forge host
- `../Scripts/provision-forgejo-nsc.sh`: render Burrow Namespace dispatcher/autoscaler runtime inputs and ensure the default Forgejo scope exists
- `../Scripts/sync-forgejo-nsc-config.sh`: copy intake-backed dispatcher/autoscaler inputs to the host
- `../Scripts/authentik-sync-namespace-portal-oidc.sh`: reconcile the Authentik OIDC app used by `nsc.burrow.net`

## Intended Flow

1. Build and upload the raw NixOS image with `Scripts/hetzner-forge.sh build-image` or `Scripts/nsc-build-and-upload-image.sh`.
2. Recreate `burrow-forge` from the latest labeled snapshot with `Scripts/hetzner-forge.sh recreate-from-image --yes`.
3. Run `Scripts/bootstrap-forge-intake.sh` to place the Forgejo bootstrap password file and automation SSH key under `/var/lib/burrow/intake/`.
4. Let `burrow-forgejo-bootstrap.service` create or rotate the initial Forgejo admin account.
5. Let `burrow-forgejo-runner-bootstrap.service` register the self-hosted Forgejo runner and seed Git identity as `agent <agent@burrow.net>`.
6. Run `Scripts/provision-forgejo-nsc.sh` locally, then `Scripts/sync-forgejo-nsc-config.sh` to place the raw Namespace dispatcher/autoscaler runtime inputs under `/var/lib/burrow/intake/` for the upstream `services.forgejo-nsc` module.
7. Visit `https://nsc.burrow.net/` as a Burrow admin to link the forge-owned Namespace session and rotate `/var/lib/burrow/intake/forgejo_nsc_token.txt` without relying on a personal local `nsc` login.
8. Ensure `/var/lib/agenix/agenix.key` exists on the host, encrypt `secrets/infra/authentik.env.age`, `secrets/infra/authentik-google-client-id.age`, `secrets/infra/authentik-google-client-secret.age`, `secrets/infra/forgejo-oidc-client-secret.age`, and `secrets/infra/headscale-oidc-client-secret.age`, and let agenix materialize them under `/run/agenix/`.
9. Use `Scripts/cloudflare-upsert-a-record.sh` to point `git.burrow.net`, `burrow.net`, `auth.burrow.net`, `ts.burrow.net`, `nsc.burrow.net`, and `nsc-autoscaler.burrow.net` at the host with Cloudflare proxying disabled for ACME.
10. Use `Scripts/forge-deploy.sh --allow-dirty` for subsequent remote `nixos-rebuild` runs from the live workspace.
11. Configure Forward Email custom S3 backups for `burrow.net` and `burrow.rs` out-of-band with `Tools/forwardemail-custom-s3.sh`.

## Current Constraints

- `burrow-forge` is live on NixOS in `hel1` at `89.167.47.21`, and `Scripts/check-forge-host.sh --expect-nsc` passes locally against that host.
- Authentik and Headscale secrets now live in tracked agenix blobs under `secrets/infra/` and decrypt to `/run/agenix/` on the forge host.
- Public Burrow forge cutover completed on March 15, 2026:
  - `burrow.net`, `git.burrow.net`, and `nsc-autoscaler.burrow.net` now publish public `A` records to `89.167.47.21`
  - HTTP redirects to HTTPS on all three names
  - `https://burrow.net` returns the root forge landing response
  - `https://git.burrow.net` returns the live Forgejo front door
  - `https://nsc-autoscaler.burrow.net` terminates TLS on Caddy and returns the expected application-level `404` for `/`
- The Cloudflare token currently in `intake/cloudflare-token.txt` is an account-scoped token: `POST /accounts/<account>/tokens/verify` succeeds, while `POST /user/tokens/verify` returns `Invalid API Token`.
- `burrow.rs` still resolves publicly to a Vercel `DEPLOYMENT_NOT_FOUND` response.
- Both domains publish Forward Email MX/TXT records.
- Forward Email custom S3 is live on both domains against the Hetzner `burrow` bucket and the public regional endpoint `https://hel1.your-objectstorage.com`.
- The current Hetzner account contains both:
  - the older Ubuntu bootstrap server in `hil`
  - the live `burrow-forge` NixOS server in `hel1`
- The remaining forge work is follow-on product/integration work, not host bring-up, mail backup wiring, or public DNS cutover.
