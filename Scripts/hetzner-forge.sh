#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

usage() {
  cat <<'EOF'
Usage: Scripts/hetzner-forge.sh [show|create|delete|recreate|build-image|create-from-image|recreate-from-image] [options]

Manage the Burrow forge server and its Hetzner snapshot lifecycle.

Defaults:
  action: show
  server-name: burrow-forge
  server-type: ccx23
  location: hel1
  image: ubuntu-24.04
  ssh keys: contact@burrow.net,agent@burrow.net

Options:
  --server-name <name>     Server name to manage.
  --server-type <type>     Hetzner server type.
  --location <code>        Hetzner location.
  --image <name|id>        Image used at create time.
  --config <name>          Burrow image config name for snapshot lookup/build (default: burrow-forge).
  --ssh-key <name>         SSH key name to attach. Repeatable.
  --token-file <path>      Hetzner API token file.
  --flake <path>           Flake path used by image-build actions (default: .)
  --upload-location <code> Hetzner location used for image upload (default: same as --location)
  --yes                    Required for delete and recreate.
  -h, --help               Show this help text.

Environment:
  HCLOUD_TOKEN_FILE        Defaults to intake/hetzner-api-token.txt
EOF
}

ACTION="show"
SERVER_NAME="burrow-forge"
SERVER_TYPE="ccx23"
LOCATION="hel1"
IMAGE="ubuntu-24.04"
CONFIG="burrow-forge"
FLAKE="."
UPLOAD_LOCATION=""
TOKEN_FILE="${HCLOUD_TOKEN_FILE:-intake/hetzner-api-token.txt}"
YES=0
SSH_KEYS=("contact@burrow.net" "agent@burrow.net")

if [[ $# -gt 0 ]]; then
  case "$1" in
    show|create|delete|recreate|build-image|create-from-image|recreate-from-image)
      ACTION="$1"
      shift
      ;;
  esac
fi

while [[ $# -gt 0 ]]; do
  case "$1" in
    --server-name)
      SERVER_NAME="${2:?missing value for --server-name}"
      shift 2
      ;;
    --server-type)
      SERVER_TYPE="${2:?missing value for --server-type}"
      shift 2
      ;;
    --location)
      LOCATION="${2:?missing value for --location}"
      shift 2
      ;;
    --image)
      IMAGE="${2:?missing value for --image}"
      shift 2
      ;;
    --config)
      CONFIG="${2:?missing value for --config}"
      shift 2
      ;;
    --ssh-key)
      SSH_KEYS+=("${2:?missing value for --ssh-key}")
      shift 2
      ;;
    --token-file)
      TOKEN_FILE="${2:?missing value for --token-file}"
      shift 2
      ;;
    --flake)
      FLAKE="${2:?missing value for --flake}"
      shift 2
      ;;
    --upload-location)
      UPLOAD_LOCATION="${2:?missing value for --upload-location}"
      shift 2
      ;;
    --yes)
      YES=1
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

if [[ ! -f "${TOKEN_FILE}" ]]; then
  echo "Hetzner API token file not found: ${TOKEN_FILE}" >&2
  exit 1
fi

if [[ -z "${UPLOAD_LOCATION}" ]]; then
  UPLOAD_LOCATION="${LOCATION}"
fi

if [[ "${ACTION}" == "delete" || "${ACTION}" == "recreate" || "${ACTION}" == "recreate-from-image" ]] && [[ ${YES} -ne 1 ]]; then
  echo "--yes is required for ${ACTION}" >&2
  exit 1
fi

latest_snapshot_id() {
  HCLOUD_TOKEN="$(tr -d '\r\n' < "${TOKEN_FILE}")" \
  BURROW_CONFIG="${CONFIG}" \
  python3 - <<'PY'
import json
import os
import urllib.parse
import urllib.request

selector = urllib.parse.quote(f"burrow.nixos-config={os.environ['BURROW_CONFIG']}", safe=",=")
req = urllib.request.Request(
    f"https://api.hetzner.cloud/v1/images?type=snapshot&label_selector={selector}",
    headers={"Authorization": f"Bearer {os.environ['HCLOUD_TOKEN']}"},
)
with urllib.request.urlopen(req, timeout=30) as resp:
    data = json.load(resp)
images = sorted(data.get("images", []), key=lambda item: item.get("created") or "")
if images:
    print(images[-1]["id"])
PY
}

if [[ "${ACTION}" == "build-image" ]]; then
  exec "${SCRIPT_DIR}/nsc-build-and-upload-image.sh" \
    --config "${CONFIG}" \
    --flake "${FLAKE}" \
    --location "${UPLOAD_LOCATION}" \
    --upload-server-type "${SERVER_TYPE}" \
    --token-file "${TOKEN_FILE}"
fi

if [[ "${ACTION}" == "create-from-image" || "${ACTION}" == "recreate-from-image" ]]; then
  if [[ "${IMAGE}" == "ubuntu-24.04" ]]; then
    IMAGE="$(latest_snapshot_id)"
  fi
  if [[ -z "${IMAGE}" ]]; then
    echo "No Burrow snapshot found for config ${CONFIG}. Run build-image first." >&2
    exit 1
  fi
  if [[ "${ACTION}" == "create-from-image" ]]; then
    ACTION="create"
  else
    ACTION="recreate"
  fi
fi

ssh_keys_csv=""
for key in "${SSH_KEYS[@]}"; do
  if [[ -n "${ssh_keys_csv}" ]]; then
    ssh_keys_csv+=","
  fi
  ssh_keys_csv+="${key}"
done

export BURROW_HCLOUD_ACTION="${ACTION}"
export BURROW_HCLOUD_SERVER_NAME="${SERVER_NAME}"
export BURROW_HCLOUD_SERVER_TYPE="${SERVER_TYPE}"
export BURROW_HCLOUD_LOCATION="${LOCATION}"
export BURROW_HCLOUD_IMAGE="${IMAGE}"
export BURROW_HCLOUD_TOKEN_FILE="${TOKEN_FILE}"
export BURROW_HCLOUD_SSH_KEYS="${ssh_keys_csv}"

python3 - <<'PY'
import json
import os
import sys
from pathlib import Path

import requests

base = "https://api.hetzner.cloud/v1"
action = os.environ["BURROW_HCLOUD_ACTION"]
server_name = os.environ["BURROW_HCLOUD_SERVER_NAME"]
server_type = os.environ["BURROW_HCLOUD_SERVER_TYPE"]
location = os.environ["BURROW_HCLOUD_LOCATION"]
image = os.environ["BURROW_HCLOUD_IMAGE"]
token = Path(os.environ["BURROW_HCLOUD_TOKEN_FILE"]).read_text(encoding="utf-8").strip()
ssh_keys = [key for key in os.environ["BURROW_HCLOUD_SSH_KEYS"].split(",") if key]

session = requests.Session()
session.headers.update({"Authorization": f"Bearer {token}", "Content-Type": "application/json"})


def request(method: str, path: str, **kwargs) -> requests.Response:
    response = session.request(method, f"{base}{path}", timeout=30, **kwargs)
    response.raise_for_status()
    return response


def find_server():
    response = request("GET", "/servers", params={"name": server_name})
    data = response.json()
    for server in data.get("servers", []):
        if server.get("name") == server_name:
            return server
    return None


def summarize(server):
    ipv4 = (((server.get("public_net") or {}).get("ipv4")) or {}).get("ip")
    image_name = ((server.get("image") or {}).get("name")) or ""
    summary = {
        "id": server.get("id"),
        "name": server.get("name"),
        "status": server.get("status"),
        "server_type": ((server.get("server_type") or {}).get("name")),
        "location": ((server.get("location") or {}).get("name")),
        "image": image_name,
        "ipv4": ipv4,
        "created": server.get("created"),
    }
    print(json.dumps(summary, indent=2))


server = find_server()

if action == "show":
    if server is None:
        print(json.dumps({"name": server_name, "present": False}, indent=2))
    else:
        summarize(server)
    sys.exit(0)

if action == "delete":
    if server is None:
        print(json.dumps({"name": server_name, "deleted": False, "reason": "not found"}, indent=2))
        sys.exit(0)
    request("DELETE", f"/servers/{server['id']}")
    print(json.dumps({"name": server_name, "deleted": True, "id": server["id"]}, indent=2))
    sys.exit(0)

if action == "recreate" and server is not None:
    request("DELETE", f"/servers/{server['id']}")
    server = None

if action in {"create", "recreate"}:
    if server is not None:
      summarize(server)
      sys.exit(0)

    payload = {
        "name": server_name,
        "server_type": server_type,
        "location": location,
        "image": image,
        "ssh_keys": ssh_keys,
        "labels": {
            "project": "burrow",
            "role": "forge",
        },
    }
    response = request("POST", "/servers", json=payload)
    created = response.json()["server"]
    summarize(created)
    sys.exit(0)

raise SystemExit(f"unsupported action: {action}")
PY
