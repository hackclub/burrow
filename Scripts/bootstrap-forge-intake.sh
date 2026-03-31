#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

usage() {
  cat <<'EOF'
Usage: Scripts/bootstrap-forge-intake.sh [options]

Copy the minimum Burrow forge bootstrap secrets onto the target host under
/var/lib/burrow/intake with the ownership expected by the NixOS services.

Options:
  --host <user@host>          SSH target (default: root@git.burrow.net)
  --ssh-key <path>            SSH private key used to reach the host
                              (default: intake/agent_at_burrow_net_ed25519)
  --password-file <path>      Forgejo admin bootstrap password file
                              (default: intake/forgejo_pass_contact_at_burrow_net.txt)
  --agent-key-file <path>     Agent SSH private key copied for runner bootstrap
                              (default: intake/agent_at_burrow_net_ed25519)
  --no-verify                 Skip remote ls/stat verification after install
  -h, --help                  Show this help text
EOF
}

HOST="${BURROW_FORGE_HOST:-root@git.burrow.net}"
SSH_KEY="${BURROW_FORGE_SSH_KEY:-${REPO_ROOT}/intake/agent_at_burrow_net_ed25519}"
PASSWORD_FILE="${BURROW_FORGE_PASSWORD_FILE:-${REPO_ROOT}/intake/forgejo_pass_contact_at_burrow_net.txt}"
AGENT_KEY_FILE="${BURROW_FORGE_AGENT_KEY_FILE:-${REPO_ROOT}/intake/agent_at_burrow_net_ed25519}"
KNOWN_HOSTS_FILE="${BURROW_FORGE_KNOWN_HOSTS_FILE:-${HOME}/.cache/burrow/forge-known_hosts}"
VERIFY=1

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
    --password-file)
      PASSWORD_FILE="${2:?missing value for --password-file}"
      shift 2
      ;;
    --agent-key-file)
      AGENT_KEY_FILE="${2:?missing value for --agent-key-file}"
      shift 2
      ;;
    --no-verify)
      VERIFY=0
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

for path in "${SSH_KEY}" "${PASSWORD_FILE}" "${AGENT_KEY_FILE}"; do
  if [[ ! -s "${path}" ]]; then
    echo "required file missing or empty: ${path}" >&2
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
  "${PASSWORD_FILE}" \
  "${AGENT_KEY_FILE}" \
  "${HOST}:${remote_tmp}/"

ssh "${ssh_opts[@]}" "${HOST}" "
  set -euo pipefail
  install -d -m 0755 /var/lib/burrow/intake
  install -m 0400 -o forgejo -g forgejo '${remote_tmp}/$(basename "${PASSWORD_FILE}")' /var/lib/burrow/intake/forgejo_pass_contact_at_burrow_net.txt
  install -m 0400 -o root -g root '${remote_tmp}/$(basename "${AGENT_KEY_FILE}")' /var/lib/burrow/intake/agent_at_burrow_net_ed25519
"

if [[ "${VERIFY}" -eq 1 ]]; then
  ssh "${ssh_opts[@]}" "${HOST}" "
    set -euo pipefail
    ls -l \
      /var/lib/burrow/intake/forgejo_pass_contact_at_burrow_net.txt \
      /var/lib/burrow/intake/agent_at_burrow_net_ed25519
  "
fi

echo "Burrow forge bootstrap intake sync complete (host=${HOST})."
