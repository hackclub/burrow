#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

# shellcheck source=Scripts/_burrow-flake.sh
source "${SCRIPT_DIR}/_burrow-flake.sh"

usage() {
  cat <<'EOF'
Usage: Scripts/provision-forgejo-nsc.sh [options]

Generate Burrow forgejo-nsc runtime inputs in intake/ and optionally refresh the
Namespace token from the currently logged-in namespace account.

Options:
  --host <user@host>       SSH target used to mint the Forgejo PAT.
                           Default: root@git.burrow.net
  --ssh-key <path>         SSH private key for the forge host.
                           Default: intake/agent_at_burrow_net_ed25519
  --nsc-bin <path>         Override the nsc binary.
  --no-refresh-token       Reuse intake/forgejo_nsc_token.txt if it already exists.
  --token-name <name>      Forgejo PAT name prefix (default: forgejo-nsc)
  --contact-user <name>    Forgejo username used for PAT creation (default: contact)
  --scope-owner <name>     Forgejo org/user owner for the default NSC scope (default: burrow)
  --scope-name <name>      Forgejo repository name for the default NSC scope (default: burrow)
  -h, --help               Show this help text.
EOF
}

HOST="${BURROW_FORGE_HOST:-root@git.burrow.net}"
SSH_KEY="${BURROW_FORGE_SSH_KEY:-${REPO_ROOT}/intake/agent_at_burrow_net_ed25519}"
NSC_BIN="${NSC_BIN:-}"
KNOWN_HOSTS_FILE="${BURROW_FORGE_KNOWN_HOSTS_FILE:-${HOME}/.cache/burrow/forge-known_hosts}"
REFRESH_TOKEN=1
TOKEN_NAME_PREFIX="${FORGEJO_PAT_NAME:-forgejo-nsc}"
CONTACT_USER="${FORGEJO_CONTACT_USER:-contact}"
SCOPE_OWNER="${FORGEJO_SCOPE_OWNER:-burrow}"
SCOPE_NAME="${FORGEJO_SCOPE_NAME:-burrow}"
BURROW_FLAKE_TMPDIRS=()

cleanup() {
  burrow_cleanup_flake_tmpdirs
}
trap cleanup EXIT

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
    --nsc-bin)
      NSC_BIN="${2:?missing value for --nsc-bin}"
      shift 2
      ;;
    --no-refresh-token)
      REFRESH_TOKEN=0
      shift
      ;;
    --token-name)
      TOKEN_NAME_PREFIX="${2:?missing value for --token-name}"
      shift 2
      ;;
    --contact-user)
      CONTACT_USER="${2:?missing value for --contact-user}"
      shift 2
      ;;
    --scope-owner)
      SCOPE_OWNER="${2:?missing value for --scope-owner}"
      shift 2
      ;;
    --scope-name)
      SCOPE_NAME="${2:?missing value for --scope-name}"
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

mkdir -p "$(dirname "${KNOWN_HOSTS_FILE}")"

burrow_require_cmd nix
burrow_require_cmd ssh
burrow_require_cmd python3

if [[ ! -f "${SSH_KEY}" ]]; then
  echo "forge SSH key not found: ${SSH_KEY}" >&2
  exit 1
fi

mkdir -p "${REPO_ROOT}/intake"
chmod 700 "${REPO_ROOT}/intake"

flake_ref="$(burrow_prepare_flake_ref "${REPO_ROOT}")"
if [[ -z "${NSC_BIN}" ]]; then
  if command -v nsc >/dev/null 2>&1; then
    NSC_BIN="$(command -v nsc)"
  else
    nsc_build_output="$(
      nix --extra-experimental-features "nix-command flakes" build \
        "${flake_ref}#nsc" \
        --no-link \
        --print-out-paths 2>&1
    )" || {
      printf '%s\n' "${nsc_build_output}" >&2
      exit 1
    }
    NSC_BIN="$(printf '%s\n' "${nsc_build_output}" | tail -n1)/bin/nsc"
  fi
fi

if [[ ! -x "${NSC_BIN}" ]]; then
  echo "unable to resolve an executable nsc binary; set NSC_BIN explicitly" >&2
  exit 1
fi

token_file="${REPO_ROOT}/intake/forgejo_nsc_token.txt"
dispatcher_out="${REPO_ROOT}/intake/forgejo_nsc_dispatcher.yaml"
autoscaler_out="${REPO_ROOT}/intake/forgejo_nsc_autoscaler.yaml"
dispatcher_src="${REPO_ROOT}/services/forgejo-nsc/deploy/dispatcher.yaml"
autoscaler_src="${REPO_ROOT}/services/forgejo-nsc/deploy/autoscaler.yaml"

if [[ "${REFRESH_TOKEN}" -eq 1 || ! -s "${token_file}" ]]; then
  "${NSC_BIN}" auth check-login --duration 20m >/dev/null
  "${NSC_BIN}" auth generate-dev-token --output_to "${token_file}" >/dev/null
  chmod 600 "${token_file}"
fi

webhook_secret="$(python3 - <<'PY'
import secrets
print(secrets.token_hex(32))
PY
)"

token_name="${TOKEN_NAME_PREFIX}-$(date -u +%Y%m%dT%H%M%SZ)"
forgejo_pat="$(
  ssh \
    -i "${SSH_KEY}" \
    -o IdentitiesOnly=yes \
    -o UserKnownHostsFile="${KNOWN_HOSTS_FILE}" \
    -o StrictHostKeyChecking=accept-new \
    "${HOST}" \
    "set -euo pipefail; forgejo_bin=\$(systemctl show -p ExecStart forgejo.service --value | sed -E 's/^\\{ path=([^ ;]+).*/\\1/'); sudo -u forgejo \"\${forgejo_bin}\" --config /var/lib/forgejo/custom/conf/app.ini --custom-path /var/lib/forgejo/custom --work-path /var/lib/forgejo admin user generate-access-token --username '${CONTACT_USER}' --scopes all --raw --token-name '${token_name}'" \
    | tr -d '\r\n'
)"

if [[ -z "${forgejo_pat}" ]]; then
  echo "failed to mint Forgejo PAT on ${HOST}" >&2
  exit 1
fi

ssh \
  -i "${SSH_KEY}" \
  -o IdentitiesOnly=yes \
  -o UserKnownHostsFile="${KNOWN_HOSTS_FILE}" \
  -o StrictHostKeyChecking=accept-new \
  "${HOST}" \
  'bash -s' <<EOF
set -euo pipefail

base_url='http://127.0.0.1:3000'
token='${forgejo_pat}'
scope_owner='${SCOPE_OWNER}'
scope_name='${SCOPE_NAME}'

api() {
  curl -sS -o /tmp/forgejo-provision-response.json -w '%{http_code}' \
    -H "Authorization: token \${token}" \
    -H 'Content-Type: application/json' \
    "\$@"
}

org_code="\$(api "\${base_url}/api/v1/orgs/\${scope_owner}")"
if [[ "\${org_code}" == "404" ]]; then
  cat >/tmp/forgejo-provision-org.json <<JSON
{"username":"${SCOPE_OWNER}","full_name":"${SCOPE_OWNER}","visibility":"public"}
JSON
  org_code="\$(api -X POST --data @/tmp/forgejo-provision-org.json "\${base_url}/api/v1/orgs")"
  if [[ "\${org_code}" != "201" ]]; then
    echo "failed to create Forgejo org ${SCOPE_OWNER} (HTTP \${org_code})" >&2
    cat /tmp/forgejo-provision-response.json >&2
    exit 1
  fi
fi

repo_code="\$(api "\${base_url}/api/v1/repos/\${scope_owner}/\${scope_name}")"
if [[ "\${repo_code}" == "404" ]]; then
  cat >/tmp/forgejo-provision-repo.json <<JSON
{"name":"${SCOPE_NAME}","description":"Burrow forge bootstrap repository","private":false,"default_branch":"main","auto_init":false}
JSON
  repo_code="\$(api -X POST --data @/tmp/forgejo-provision-repo.json "\${base_url}/api/v1/orgs/\${scope_owner}/repos")"
  if [[ "\${repo_code}" != "201" ]]; then
    echo "failed to create Forgejo repo ${SCOPE_OWNER}/${SCOPE_NAME} (HTTP \${repo_code})" >&2
    cat /tmp/forgejo-provision-response.json >&2
    exit 1
  fi
fi
EOF

FORGEJO_PAT="${forgejo_pat}" \
WEBHOOK_SECRET="${webhook_secret}" \
DISPATCHER_SRC="${dispatcher_src}" \
AUTOSCALER_SRC="${autoscaler_src}" \
DISPATCHER_OUT="${dispatcher_out}" \
AUTOSCALER_OUT="${autoscaler_out}" \
python3 - <<'PY'
import os
from pathlib import Path

def render(src: str, dst: str) -> None:
    text = Path(src).read_text(encoding="utf-8")
    text = text.replace("PENDING-FORGEJO-PAT", os.environ["FORGEJO_PAT"])
    text = text.replace("PENDING-WEBHOOK-SECRET", os.environ["WEBHOOK_SECRET"])
    Path(dst).write_text(text, encoding="utf-8")

render(os.environ["DISPATCHER_SRC"], os.environ["DISPATCHER_OUT"])
render(os.environ["AUTOSCALER_SRC"], os.environ["AUTOSCALER_OUT"])
PY

chmod 600 "${dispatcher_out}" "${autoscaler_out}"

echo "Rendered intake/forgejo_nsc_token.txt, intake/forgejo_nsc_dispatcher.yaml, and intake/forgejo_nsc_autoscaler.yaml."
echo "Minted Forgejo PAT ${token_name} for ${CONTACT_USER} on ${HOST}."
