#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

usage() {
  cat <<'EOF'
Usage: Scripts/check-forge-host.sh [options]

Run a post-boot verification pass against the Burrow forge host.

Options:
  --host <user@host>    SSH target (default: root@git.burrow.net)
  --ssh-key <path>      SSH private key (default: intake/agent_at_burrow_net_ed25519)
  --expect-nsc          Fail if forgejo-nsc services are not active
  --expect-tailnet      Fail if Authentik and Headscale services are not active
  -h, --help            Show this help text
EOF
}

HOST="${BURROW_FORGE_HOST:-root@git.burrow.net}"
SSH_KEY="${BURROW_FORGE_SSH_KEY:-${REPO_ROOT}/intake/agent_at_burrow_net_ed25519}"
KNOWN_HOSTS_FILE="${BURROW_FORGE_KNOWN_HOSTS_FILE:-${HOME}/.cache/burrow/forge-known_hosts}"
EXPECT_NSC=0
EXPECT_TAILNET=0

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
    --expect-nsc)
      EXPECT_NSC=1
      shift
      ;;
    --expect-tailnet)
      EXPECT_TAILNET=1
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

if [[ ! -f "${SSH_KEY}" ]]; then
  echo "forge SSH key not found: ${SSH_KEY}" >&2
  exit 1
fi

ssh \
  -i "${SSH_KEY}" \
  -o IdentitiesOnly=yes \
  -o UserKnownHostsFile="${KNOWN_HOSTS_FILE}" \
  -o StrictHostKeyChecking=accept-new \
  "${HOST}" \
  EXPECT_NSC="${EXPECT_NSC}" \
  EXPECT_TAILNET="${EXPECT_TAILNET}" \
  'bash -s' <<'EOF'
set -euo pipefail

base_services=(
  forgejo.service
  caddy.service
  burrow-forgejo-bootstrap.service
  burrow-forgejo-runner-bootstrap.service
  burrow-forgejo-runner.service
)

nsc_services=(
  forgejo-nsc-dispatcher.service
  forgejo-nsc-autoscaler.service
)

tailnet_services=(
  burrow-authentik-runtime.service
  burrow-authentik-ready.service
  headscale.service
  headscale-bootstrap.service
)

show_service() {
  local service="$1"
  systemctl show \
    --no-pager \
    --property Id \
    --property LoadState \
    --property UnitFileState \
    --property ActiveState \
    --property SubState \
    --property Result \
    "${service}"
}

service_is_healthy() {
  local service="$1"
  local active_state
  local result
  local unit_type

  active_state="$(systemctl show --property ActiveState --value "${service}")"
  result="$(systemctl show --property Result --value "${service}")"
  unit_type="$(systemctl show --property Type --value "${service}")"

  if [[ "${active_state}" == "active" ]]; then
    return 0
  fi

  if [[ "${unit_type}" == "oneshot" && "${active_state}" == "inactive" && "${result}" == "success" ]]; then
    return 0
  fi

  return 1
}

for service in "${base_services[@]}"; do
  echo "== ${service} =="
  show_service "${service}"
  if ! service_is_healthy "${service}"; then
    echo "required service is not active: ${service}" >&2
    exit 1
  fi
done

for service in "${nsc_services[@]}"; do
  echo "== ${service} =="
  show_service "${service}" || true
  if [[ "${EXPECT_NSC}" == "1" && "$(systemctl is-active "${service}" 2>/dev/null || true)" != "active" ]]; then
    echo "required NSC service is not active: ${service}" >&2
    exit 1
  fi
done

for service in "${tailnet_services[@]}"; do
  echo "== ${service} =="
  show_service "${service}" || true
  if [[ "${EXPECT_TAILNET}" == "1" ]] && ! service_is_healthy "${service}"; then
    echo "required tailnet service is not active: ${service}" >&2
    exit 1
  fi
done

echo "== intake =="
ls -l /var/lib/burrow/intake || true

if [[ "${EXPECT_TAILNET}" == "1" ]]; then
  echo "== agenix =="
  ls -l /run/agenix || true
  test -s /run/agenix/burrowAuthentikEnv
  test -s /run/agenix/burrowHeadscaleOidcClientSecret
fi

if [[ "${EXPECT_NSC}" == "1" ]]; then
  echo "== agenix-nsc =="
  ls -l /run/agenix || true
  test -s /run/agenix/burrowForgejoNscToken
  test -s /run/agenix/burrowForgejoNscDispatcherConfig
  test -s /run/agenix/burrowForgejoNscAutoscalerConfig
fi

if command -v curl >/dev/null 2>&1; then
  echo "== http-local =="
  curl -fsS -o /dev/null -w 'forgejo_login %{http_code}\n' http://127.0.0.1:3000/user/login
  curl -fsS -o /dev/null -H 'Host: burrow.net' -w 'burrow_root %{http_code}\n' http://127.0.0.1/
  curl -fsS -o /dev/null -H 'Host: git.burrow.net' -w 'git_login %{http_code}\n' http://127.0.0.1/user/login
  if [[ "${EXPECT_TAILNET}" == "1" ]]; then
    curl -fsS -o /dev/null -H 'Host: auth.burrow.net' -w 'authentik_ready %{http_code}\n' http://127.0.0.1/-/health/ready/
    curl -sS -o /dev/null -H 'Host: ts.burrow.net' -w 'headscale_root %{http_code}\n' http://127.0.0.1/ || true
  fi
fi
EOF
