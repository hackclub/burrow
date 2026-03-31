#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

# shellcheck source=Scripts/_burrow-flake.sh
source "${SCRIPT_DIR}/_burrow-flake.sh"

usage() {
  cat <<'EOF'
Usage: Scripts/forge-deploy.sh [--test|--switch] [--flake-attr <attr>] [--allow-dirty]

Standardized remote deploy path for the Burrow forge host.

Defaults:
  --switch
  --flake-attr burrow-forge

Environment:
  BURROW_FORGE_HOST        root@git.burrow.net
  BURROW_FORGE_SSH_KEY     intake/agent_at_burrow_net_ed25519
EOF
}

MODE="switch"
FLAKE_ATTR="burrow-forge"
ALLOW_DIRTY=0
BURROW_FLAKE_TMPDIRS=()

cleanup() {
  burrow_cleanup_flake_tmpdirs
}
trap cleanup EXIT

while [[ $# -gt 0 ]]; do
  case "$1" in
    --test)
      MODE="test"
      shift
      ;;
    --switch)
      MODE="switch"
      shift
      ;;
    --flake-attr)
      FLAKE_ATTR="${2:?missing value for --flake-attr}"
      shift 2
      ;;
    --allow-dirty)
      ALLOW_DIRTY=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "${REPO_ROOT}"

if [[ ${ALLOW_DIRTY} -ne 1 ]] && [[ -n "$(git status --short)" ]]; then
  echo "Refusing to deploy from a dirty checkout. Commit first, or pass --allow-dirty for incident-only work." >&2
  exit 1
fi

FORGE_HOST="${BURROW_FORGE_HOST:-root@git.burrow.net}"
FORGE_SSH_KEY="${BURROW_FORGE_SSH_KEY:-}"

if [[ -z "${FORGE_SSH_KEY}" ]]; then
  if [[ -f "${REPO_ROOT}/intake/agent_at_burrow_net_ed25519" ]]; then
    FORGE_SSH_KEY="${REPO_ROOT}/intake/agent_at_burrow_net_ed25519"
  else
    FORGE_SSH_KEY="${HOME}/.ssh/agent_at_burrow_net_ed25519"
  fi
fi

if [[ ! -f "${FORGE_SSH_KEY}" ]]; then
  echo "Forge SSH key not found at ${FORGE_SSH_KEY}." >&2
  echo "Set BURROW_FORGE_SSH_KEY or place the agent key in intake/." >&2
  exit 1
fi

FORGE_KNOWN_HOSTS_FILE="${BURROW_FORGE_KNOWN_HOSTS_FILE:-${HOME}/.cache/burrow/forge-known_hosts}"
mkdir -p "$(dirname "${FORGE_KNOWN_HOSTS_FILE}")"

export NIX_SSHOPTS="-i ${FORGE_SSH_KEY} -o IdentitiesOnly=yes -o UserKnownHostsFile=${FORGE_KNOWN_HOSTS_FILE} -o StrictHostKeyChecking=accept-new"
flake_ref="$(burrow_prepare_flake_ref "${REPO_ROOT}")"

nix --extra-experimental-features "nix-command flakes" shell nixpkgs#nixos-rebuild -c \
  nixos-rebuild "${MODE}" \
  --flake "${flake_ref}#${FLAKE_ATTR}" \
  --build-host "${FORGE_HOST}" \
  --target-host "${FORGE_HOST}"
