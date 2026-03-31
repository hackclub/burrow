#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage: Scripts/sync-forgejo-nsc-config.sh [options]

Copy Burrow forgejo-nsc runtime inputs from intake/ onto the forge host and
restart the dispatcher/autoscaler units.

Options:
  --host <user@host>       SSH target (default: root@git.burrow.net)
  --ssh-key <path>         SSH private key (default: intake/agent_at_burrow_net_ed25519)
  --rotate-pat             Re-render the intake files before syncing.
  --no-restart             Copy files only.
  -h, --help               Show this help text.
EOF
}

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

HOST="${BURROW_FORGE_HOST:-root@git.burrow.net}"
SSH_KEY="${BURROW_FORGE_SSH_KEY:-${REPO_ROOT}/intake/agent_at_burrow_net_ed25519}"
KNOWN_HOSTS_FILE="${BURROW_FORGE_KNOWN_HOSTS_FILE:-${HOME}/.cache/burrow/forge-known_hosts}"
ROTATE_PAT=0
NO_RESTART=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --host)
      HOST="${2:?missing value for --host}"
      shift 2
      ;;
    --ssh-key)
      SSH_KEY="${2:?missing value for --ssh-key}"
      shift 2
      ;;
    --rotate-pat)
      ROTATE_PAT=1
      shift
      ;;
    --no-restart)
      NO_RESTART=1
      shift
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

mkdir -p "$(dirname "${KNOWN_HOSTS_FILE}")"

burrow_require_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "missing required command: $1" >&2
    exit 1
  fi
}

burrow_require_cmd ssh
burrow_require_cmd scp

if [[ ! -f "${SSH_KEY}" ]]; then
  echo "forge SSH key not found: ${SSH_KEY}" >&2
  exit 1
fi

if [[ "${ROTATE_PAT}" -eq 1 ]]; then
  "${SCRIPT_DIR}/provision-forgejo-nsc.sh" --host "${HOST}" --ssh-key "${SSH_KEY}"
fi

token_file="${REPO_ROOT}/intake/forgejo_nsc_token.txt"
dispatcher_file="${REPO_ROOT}/intake/forgejo_nsc_dispatcher.yaml"
autoscaler_file="${REPO_ROOT}/intake/forgejo_nsc_autoscaler.yaml"

for path in "${token_file}" "${dispatcher_file}" "${autoscaler_file}"; do
  if [[ ! -s "${path}" ]]; then
    echo "required runtime input missing or empty: ${path}" >&2
    exit 1
  fi
done

ssh_opts=(
  -i "${SSH_KEY}"
  -o IdentitiesOnly=yes
  -o UserKnownHostsFile="${KNOWN_HOSTS_FILE}"
  -o StrictHostKeyChecking=accept-new
)

remote_tmp="$(ssh "${ssh_opts[@]}" "${HOST}" "mktemp -d")"
cleanup() {
  if [[ -n "${remote_tmp:-}" ]]; then
    ssh "${ssh_opts[@]}" "${HOST}" "rm -rf '${remote_tmp}'" >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT

scp "${ssh_opts[@]}" \
  "${token_file}" \
  "${dispatcher_file}" \
  "${autoscaler_file}" \
  "${HOST}:${remote_tmp}/"

ssh "${ssh_opts[@]}" "${HOST}" "
  set -euo pipefail
  install -d -m 0755 /var/lib/burrow/intake
  install -m 0400 -o forgejo-nsc -g forgejo-nsc '${remote_tmp}/$(basename "${token_file}")' /var/lib/burrow/intake/forgejo_nsc_token.txt
  install -m 0400 -o forgejo-nsc -g forgejo-nsc '${remote_tmp}/$(basename "${dispatcher_file}")' /var/lib/burrow/intake/forgejo_nsc_dispatcher.yaml
  install -m 0400 -o forgejo-nsc -g forgejo-nsc '${remote_tmp}/$(basename "${autoscaler_file}")' /var/lib/burrow/intake/forgejo_nsc_autoscaler.yaml
"

if [[ "${NO_RESTART}" -eq 0 ]]; then
  ssh "${ssh_opts[@]}" "${HOST}" "
    set -euo pipefail
    systemctl restart forgejo-nsc-dispatcher.service forgejo-nsc-autoscaler.service
    systemctl is-active forgejo-nsc-dispatcher.service forgejo-nsc-autoscaler.service
    ls -l \
      /var/lib/burrow/intake/forgejo_nsc_token.txt \
      /var/lib/burrow/intake/forgejo_nsc_dispatcher.yaml \
      /var/lib/burrow/intake/forgejo_nsc_autoscaler.yaml
  "
fi

echo "forgejo-nsc runtime sync complete (host=${HOST}, restarted=$((1 - NO_RESTART)))."
