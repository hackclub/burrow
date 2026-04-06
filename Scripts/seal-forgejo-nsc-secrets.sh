#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

usage() {
  cat <<'EOF'
Usage: Scripts/seal-forgejo-nsc-secrets.sh [options]

Encrypt Burrow forgejo-nsc runtime inputs from intake/ into the agenix secrets
consumed by burrow-forge.

Options:
  --provision            Re-render the local intake files before sealing.
  --host <user@host>     SSH target forwarded to provision-forgejo-nsc.sh.
  --ssh-key <path>       SSH private key forwarded to provision-forgejo-nsc.sh.
  --nsc-bin <path>       Override the nsc binary for provisioning.
  -h, --help             Show this help text.
EOF
}

PROVISION=0
HOST="${BURROW_FORGE_HOST:-root@git.burrow.net}"
SSH_KEY="${BURROW_FORGE_SSH_KEY:-${REPO_ROOT}/intake/agent_at_burrow_net_ed25519}"
NSC_BIN="${NSC_BIN:-}"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --provision)
      PROVISION=1
      shift
      ;;
    --host)
      HOST="${2:?missing value for --host}"
      shift 2
      ;;
    --ssh-key)
      SSH_KEY="${2:?missing value for --ssh-key}"
      shift 2
      ;;
    --nsc-bin)
      NSC_BIN="${2:?missing value for --nsc-bin}"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "unknown option: $1" >&2
      usage >&2
      exit 64
      ;;
  esac
done

require_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "missing required command: $1" >&2
    exit 1
  fi
}

require_cmd age
require_cmd nix
require_cmd python3

if [[ "${PROVISION}" -eq 1 ]]; then
  provision_args=(--host "${HOST}" --ssh-key "${SSH_KEY}")
  if [[ -n "${NSC_BIN}" ]]; then
    provision_args+=(--nsc-bin "${NSC_BIN}")
  fi
  "${SCRIPT_DIR}/provision-forgejo-nsc.sh" "${provision_args[@]}"
fi

tmpdir="$(mktemp -d)"
cleanup() {
  rm -rf "${tmpdir}"
}
trap cleanup EXIT

seal_secret() {
  local target="$1"
  local source_path="$2"
  recipients_file="${tmpdir}/$(basename "${target}").recipients"
  if [[ ! -s "${source_path}" ]]; then
    echo "required runtime input missing or empty: ${source_path}" >&2
    exit 1
  fi
  nix eval --impure --json --expr "let s = import ${REPO_ROOT}/secrets.nix; in s.\"${target}\".publicKeys" \
    | python3 -c 'import json, sys; [print(item) for item in json.load(sys.stdin)]' \
    > "${recipients_file}"

  age -R "${recipients_file}" -o "${REPO_ROOT}/${target}" "${source_path}"
}

seal_secret "secrets/infra/forgejo-nsc-token.age" "${REPO_ROOT}/intake/forgejo_nsc_token.txt"
seal_secret "secrets/infra/forgejo-nsc-dispatcher-config.age" "${REPO_ROOT}/intake/forgejo_nsc_dispatcher.yaml"
seal_secret "secrets/infra/forgejo-nsc-autoscaler-config.age" "${REPO_ROOT}/intake/forgejo_nsc_autoscaler.yaml"

chmod 600 \
  "${REPO_ROOT}/secrets/infra/forgejo-nsc-token.age" \
  "${REPO_ROOT}/secrets/infra/forgejo-nsc-dispatcher-config.age" \
  "${REPO_ROOT}/secrets/infra/forgejo-nsc-autoscaler-config.age"

echo "Sealed forgejo-nsc runtime inputs into:"
printf '  %s\n' \
  "${REPO_ROOT}/secrets/infra/forgejo-nsc-token.age" \
  "${REPO_ROOT}/secrets/infra/forgejo-nsc-dispatcher-config.age" \
  "${REPO_ROOT}/secrets/infra/forgejo-nsc-autoscaler-config.age"
echo "Deploy burrow-forge to apply the new CI credentials."
