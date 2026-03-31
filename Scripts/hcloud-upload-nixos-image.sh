#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

# shellcheck source=Scripts/_burrow-flake.sh
source "${SCRIPT_DIR}/_burrow-flake.sh"

DEFAULT_CONFIG="burrow-forge"
DEFAULT_FLAKE="."
DEFAULT_LOCATION="hel1"
DEFAULT_ARCHITECTURE="x86"
DEFAULT_TOKEN_FILE="${REPO_ROOT}/intake/hetzner-api-token.txt"

CONFIG="${HCLOUD_IMAGE_CONFIG:-${DEFAULT_CONFIG}}"
FLAKE="${HCLOUD_IMAGE_FLAKE:-${DEFAULT_FLAKE}}"
LOCATION="${HCLOUD_IMAGE_LOCATION:-${DEFAULT_LOCATION}}"
ARCHITECTURE="${HCLOUD_IMAGE_ARCHITECTURE:-${DEFAULT_ARCHITECTURE}}"
TOKEN_FILE="${HCLOUD_TOKEN_FILE:-${DEFAULT_TOKEN_FILE}}"
DESCRIPTION="${HCLOUD_IMAGE_DESCRIPTION:-}"
UPLOAD_SERVER_TYPE="${HCLOUD_IMAGE_UPLOAD_SERVER_TYPE:-}"
UPLOAD_VERBOSE="${HCLOUD_IMAGE_UPLOAD_VERBOSE:-0}"
ARTIFACT_PATH_INPUT=""
OUTPUT_HASH=""
NO_UPDATE=0
BUILDER_SPEC="${HCLOUD_IMAGE_BUILDER_SPEC:-}"
EXTRA_LABELS=()
NIX_BUILD_FLAGS=()
BURROW_FLAKE_TMPDIRS=()
LOCAL_STORE_DIR=""

usage() {
  cat <<'EOF'
Usage: Scripts/hcloud-upload-nixos-image.sh [options]

Build a raw Burrow NixOS image and upload it into Hetzner Cloud as a snapshot.

Options:
  --config <name>           images.<name>-raw output to build (default: burrow-forge)
  --flake <path>            Flake path to build from (default: .)
  --location <code>         Hetzner location for the temporary upload server (default: hel1)
  --architecture <x86|arm>  CPU architecture of the image (default: x86)
  --server-type <name>      Hetzner server type for the temporary upload server
  --token-file <path>       Hetzner API token file (default: intake/hetzner-api-token.txt)
  --artifact-path <path>    Prebuilt raw image artifact to upload directly
  --output-hash <hash>      Stable hash label for --artifact-path uploads
  --builder-spec <string>   Complete builders string passed to nix build
  --description <text>      Description for the resulting snapshot
  --upload-verbose <n>      Pass -v N times to hcloud-upload-image
  --label key=value         Extra Hetzner image label (repeatable)
  --nix-flag <arg>          Extra argument passed to nix build (repeatable)
  --no-update               Reuse an existing snapshot with the same config/output hash
  -h, --help                Show this help text
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --config)
      CONFIG="${2:?missing value for --config}"
      shift 2
      ;;
    --flake)
      FLAKE="${2:?missing value for --flake}"
      shift 2
      ;;
    --location)
      LOCATION="${2:?missing value for --location}"
      shift 2
      ;;
    --architecture)
      ARCHITECTURE="${2:?missing value for --architecture}"
      shift 2
      ;;
    --server-type)
      UPLOAD_SERVER_TYPE="${2:?missing value for --server-type}"
      shift 2
      ;;
    --token-file)
      TOKEN_FILE="${2:?missing value for --token-file}"
      shift 2
      ;;
    --artifact-path)
      ARTIFACT_PATH_INPUT="${2:?missing value for --artifact-path}"
      shift 2
      ;;
    --output-hash)
      OUTPUT_HASH="${2:?missing value for --output-hash}"
      shift 2
      ;;
    --builder-spec)
      BUILDER_SPEC="${2:?missing value for --builder-spec}"
      shift 2
      ;;
    --description)
      DESCRIPTION="${2:?missing value for --description}"
      shift 2
      ;;
    --upload-verbose)
      UPLOAD_VERBOSE="${2:?missing value for --upload-verbose}"
      shift 2
      ;;
    --label)
      EXTRA_LABELS+=("${2:?missing value for --label}")
      shift 2
      ;;
    --nix-flag)
      NIX_BUILD_FLAGS+=("${2:?missing value for --nix-flag}")
      shift 2
      ;;
    --no-update)
      NO_UPDATE=1
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

cleanup() {
  burrow_cleanup_flake_tmpdirs
  if [[ -n "${LOCAL_STORE_DIR}" && -d "${LOCAL_STORE_DIR}" ]]; then
    rm -rf "${LOCAL_STORE_DIR}" >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT

burrow_require_cmd nix
burrow_require_cmd curl
burrow_require_cmd python3
burrow_require_cmd rsync

if [[ ! -f "${TOKEN_FILE}" ]]; then
  echo "Hetzner API token file not found: ${TOKEN_FILE}" >&2
  exit 1
fi

HCLOUD_TOKEN="$(tr -d '\r\n' < "${TOKEN_FILE}")"
if [[ -z "${HCLOUD_TOKEN}" ]]; then
  echo "Hetzner API token file is empty: ${TOKEN_FILE}" >&2
  exit 1
fi

flake_ref="$(burrow_prepare_flake_ref "${FLAKE}")"

if [[ -z "${DESCRIPTION}" ]]; then
  DESCRIPTION="Burrow ${CONFIG} $(date -u +%Y-%m-%dT%H:%M:%SZ)"
fi

printf 'Building raw image for %s from %s\n' "${CONFIG}" "${flake_ref}" >&2

if [[ -z "${ARTIFACT_PATH_INPUT}" && -n "${BUILDER_SPEC}" && -z "${NIX_BUILD_STORE:-}" ]]; then
  mkdir -p "${HOME}/.cache/burrow"
  LOCAL_STORE_DIR="$(mktemp -d "${HOME}/.cache/burrow/local-store-XXXXXX")"
fi

artifact_path=""
compression=""
output_hash="${OUTPUT_HASH}"
if [[ -n "${ARTIFACT_PATH_INPUT}" ]]; then
  artifact_path="${ARTIFACT_PATH_INPUT}"
  if [[ ! -f "${artifact_path}" ]]; then
    echo "artifact path does not exist: ${artifact_path}" >&2
    exit 1
  fi
  compression="$(burrow_detect_compression "${artifact_path}")"
  if [[ -z "${output_hash}" ]]; then
    if command -v sha256sum >/dev/null 2>&1; then
      output_hash="$(sha256sum "${artifact_path}" | awk '{print $1}')"
    else
      output_hash="$(shasum -a 256 "${artifact_path}" | awk '{print $1}')"
    fi
  fi
else
  nix_build_cmd=(
    nix
    --extra-experimental-features
    "nix-command flakes"
    build
    "${flake_ref}#images.${CONFIG}-raw"
    --no-link
    --print-out-paths
  )

  if [[ -n "${BUILDER_SPEC}" ]]; then
    nix_build_cmd+=(--builders "${BUILDER_SPEC}")
  fi
  if [[ -n "${NIX_BUILD_STORE:-}" ]]; then
    nix_build_cmd+=(--store "${NIX_BUILD_STORE}")
  elif [[ -n "${LOCAL_STORE_DIR}" ]]; then
    nix_build_cmd+=(--store "${LOCAL_STORE_DIR}")
  fi

  if [[ "${#NIX_BUILD_FLAGS[@]}" -gt 0 ]]; then
    nix_build_cmd+=("${NIX_BUILD_FLAGS[@]}")
  fi

  build_output=""
  if ! build_output="$("${nix_build_cmd[@]}" 2>&1)"; then
    printf '%s\n' "${build_output}" >&2
    exit 1
  fi

  store_path="$(printf '%s\n' "${build_output}" | tail -n1)"
  if [[ -z "${store_path}" ]]; then
    echo "nix build did not return a store path" >&2
    printf '%s\n' "${build_output}" >&2
    exit 1
  fi

  artifact_path="$(burrow_resolve_image_artifact "${store_path}")"
  compression="$(burrow_detect_compression "${artifact_path}")"
  output_hash="$(basename "${store_path}")"
  output_hash="${output_hash%%-*}"
fi

label_args=(
  "burrow.nixos-config=${CONFIG}"
  "burrow.nixos-output-hash=${output_hash}"
)
if [[ "${#EXTRA_LABELS[@]}" -gt 0 ]]; then
  label_args+=("${EXTRA_LABELS[@]}")
fi
label_csv="$(IFS=,; printf '%s' "${label_args[*]}")"

find_existing_image() {
  HCLOUD_TOKEN="${HCLOUD_TOKEN}" \
  BURROW_LABEL_SELECTOR="burrow.nixos-config=${CONFIG},burrow.nixos-output-hash=${output_hash}" \
  python3 - <<'PY'
import json
import os
import sys
import urllib.parse
import urllib.request

selector = urllib.parse.quote(os.environ["BURROW_LABEL_SELECTOR"], safe=",=")
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

if [[ "${NO_UPDATE}" -eq 1 ]]; then
  existing_id="$(find_existing_image || true)"
  if [[ -n "${existing_id}" ]]; then
    printf 'Reusing existing Hetzner snapshot %s for %s\n' "${existing_id}" "${CONFIG}" >&2
    printf '%s\n' "${existing_id}"
    exit 0
  fi
fi

uploader_bin="${HCLOUD_UPLOAD_IMAGE_BIN:-}"
if [[ -z "${uploader_bin}" ]]; then
  uploader_build_output="$(
    nix --extra-experimental-features "nix-command flakes" build \
      "${flake_ref}#hcloud-upload-image" \
      --no-link \
      --print-out-paths 2>&1
  )" || {
    printf '%s\n' "${uploader_build_output}" >&2
    exit 1
  }
  uploader_bin="$(printf '%s\n' "${uploader_build_output}" | tail -n1)/bin/hcloud-upload-image"
fi

if [[ ! -x "${uploader_bin}" ]]; then
  echo "unable to resolve an executable hcloud-upload-image binary; set HCLOUD_UPLOAD_IMAGE_BIN explicitly" >&2
  exit 1
fi

upload_cmd=(
  "${uploader_bin}"
)
if [[ "${UPLOAD_VERBOSE}" =~ ^[0-9]+$ ]] && [[ "${UPLOAD_VERBOSE}" -gt 0 ]]; then
  for _ in $(seq 1 "${UPLOAD_VERBOSE}"); do
    upload_cmd+=(-v)
  done
fi
upload_cmd+=(
  upload
  --image-path "${artifact_path}"
  --location "${LOCATION}"
  --description "${DESCRIPTION}"
  --labels "${label_csv}"
)
if [[ -n "${UPLOAD_SERVER_TYPE}" ]]; then
  upload_cmd+=(--server-type "${UPLOAD_SERVER_TYPE}")
else
  upload_cmd+=(--architecture "${ARCHITECTURE}")
fi
if [[ -n "${compression}" ]]; then
  upload_cmd+=(--compression "${compression}")
fi

printf 'Uploading %s to Hetzner Cloud via %s\n' "${artifact_path}" "${uploader_bin}" >&2
HCLOUD_TOKEN="${HCLOUD_TOKEN}" "${upload_cmd[@]}" >&2

image_id=""
for _ in $(seq 1 24); do
  image_id="$(find_existing_image || true)"
  if [[ -n "${image_id}" ]]; then
    break
  fi
  sleep 5
done

if [[ -z "${image_id}" ]]; then
  echo "failed to locate uploaded Hetzner snapshot after upload completed" >&2
  exit 1
fi

printf '%s\n' "${image_id}"
