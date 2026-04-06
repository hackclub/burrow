#!/usr/bin/env bash
set -euo pipefail

echo "Scripts/sync-forgejo-nsc-config.sh is obsolete." >&2
echo "Burrow forgejo-nsc now consumes agenix-backed secrets instead of host-local intake files." >&2
echo "Use Scripts/seal-forgejo-nsc-secrets.sh and deploy burrow-forge." >&2
exit 1
