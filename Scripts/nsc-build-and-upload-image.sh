#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

# shellcheck source=Scripts/_burrow-flake.sh
source "${SCRIPT_DIR}/_burrow-flake.sh"

CONFIG="${HCLOUD_IMAGE_CONFIG:-burrow-forge}"
FLAKE="${HCLOUD_IMAGE_FLAKE:-.}"
LOCATION="${HCLOUD_IMAGE_LOCATION:-hel1}"
TOKEN_FILE="${HCLOUD_TOKEN_FILE:-${REPO_ROOT}/intake/hetzner-api-token.txt}"
NSC_SSH_HOST="${NSC_SSH_HOST:-ssh.ord2.namespace.so}"
NSC_MACHINE_TYPE="${NSC_MACHINE_TYPE:-linux/amd64:32x64}"
NSC_BUILDER_DURATION="${NSC_BUILDER_DURATION:-4h}"
NSC_BUILDER_JOBS="${NSC_BUILDER_JOBS:-32}"
NSC_BUILDER_FEATURES="${NSC_BUILDER_FEATURES:-kvm,big-parallel}"
NSC_BIN="${NSC_BIN:-}"
REMOTE_COMPRESSION="${HCLOUD_IMAGE_REMOTE_COMPRESSION:-auto}"
UPLOAD_SERVER_TYPE="${HCLOUD_IMAGE_UPLOAD_SERVER_TYPE:-}"
KEEP_TMPDIR="${HCLOUD_IMAGE_KEEP_TMPDIR:-0}"
NO_UPDATE=0
NIX_BUILD_FLAGS=()
EXTRA_LABELS=()
BURROW_FLAKE_TMPDIRS=()
BUILDER_ID=""

usage() {
  cat <<'EOF'
Usage: Scripts/nsc-build-and-upload-image.sh [options]

Create a temporary Namespace Linux builder, build the Burrow raw image on it,
and upload the resulting artifact to Hetzner Cloud.

Options:
  --config <name>         images.<name>-raw output to build (default: burrow-forge)
  --flake <path>          Flake path to build from (default: .)
  --location <code>       Hetzner upload location (default: hel1)
  --token-file <path>     Hetzner API token file (default: intake/hetzner-api-token.txt)
  --machine-type <type>   Namespace machine type (default: linux/amd64:32x64)
  --ssh-host <host>       Namespace SSH endpoint (default: ssh.ord2.namespace.so)
  --duration <ttl>        Namespace builder lifetime (default: 4h)
  --builder-jobs <n>      Nix builder job count advertised to the local client
  --builder-features <s>  Comma-separated Nix system features (default: "kvm,big-parallel")
  --remote-compression <mode>
                          Compress raw/image artifacts on the Namespace builder
                          before copy-back. Modes: auto, none, xz, zstd
                          (default: auto)
  --upload-server-type <name>
                          Hetzner server type for the temporary upload host
  --label key=value       Extra Hetzner snapshot label (repeatable)
  --nix-flag <arg>        Extra argument passed to nix build (repeatable)
  --no-update             Reuse an existing snapshot with the same config/output hash
  -h, --help              Show this help text
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
    --token-file)
      TOKEN_FILE="${2:?missing value for --token-file}"
      shift 2
      ;;
    --machine-type)
      NSC_MACHINE_TYPE="${2:?missing value for --machine-type}"
      shift 2
      ;;
    --ssh-host)
      NSC_SSH_HOST="${2:?missing value for --ssh-host}"
      shift 2
      ;;
    --duration)
      NSC_BUILDER_DURATION="${2:?missing value for --duration}"
      shift 2
      ;;
    --builder-jobs)
      NSC_BUILDER_JOBS="${2:?missing value for --builder-jobs}"
      shift 2
      ;;
    --builder-features)
      NSC_BUILDER_FEATURES="${2:?missing value for --builder-features}"
      shift 2
      ;;
    --remote-compression)
      REMOTE_COMPRESSION="${2:?missing value for --remote-compression}"
      shift 2
      ;;
    --upload-server-type)
      UPLOAD_SERVER_TYPE="${2:?missing value for --upload-server-type}"
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
  if [[ -n "${BUILDER_ID}" && -n "${NSC_BIN}" ]]; then
    "${NSC_BIN}" destroy "${BUILDER_ID}" --force >/dev/null 2>&1 || true
  fi
  burrow_cleanup_flake_tmpdirs
  if [[ "${KEEP_TMPDIR}" != "1" && -n "${TMPDIR_BURROW_NSC:-}" && -d "${TMPDIR_BURROW_NSC}" ]]; then
    rm -rf "${TMPDIR_BURROW_NSC}"
  fi
}
trap cleanup EXIT

burrow_require_cmd nix
burrow_require_cmd curl
burrow_require_cmd python3
burrow_require_cmd ssh
burrow_require_cmd ssh-keygen
burrow_require_cmd ssh-keyscan
burrow_require_cmd tar

flake_ref="$(burrow_prepare_flake_ref "${FLAKE}")"

if [[ -z "${NSC_BIN}" ]]; then
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

if [[ ! -x "${NSC_BIN}" ]]; then
  echo "unable to resolve an executable nsc binary; set NSC_BIN explicitly" >&2
  exit 1
fi

if [[ -n "${NSC_SESSION:-}" && ! -f "${HOME}/.ns/session" ]]; then
  mkdir -p "${HOME}/.ns"
  printf '%s\n' "${NSC_SESSION}" > "${HOME}/.ns/session"
  chmod 600 "${HOME}/.ns/session"
fi

"${NSC_BIN}" auth check-login --duration 20m >/dev/null
"${NSC_BIN}" version >/dev/null || true

TMPDIR_BURROW_NSC="$(mktemp -d "${HOME}/.cache/burrow/nsc-XXXXXX")"
ssh_key="${TMPDIR_BURROW_NSC}/builder"
known_hosts="${TMPDIR_BURROW_NSC}/known_hosts"
id_file="${TMPDIR_BURROW_NSC}/builder.id"

ssh-keygen -q -t ed25519 -N "" -f "${ssh_key}"
ssh-keyscan -H "${NSC_SSH_HOST}" > "${known_hosts}"

ssh_base=(
  ssh
  -i "${ssh_key}"
  -o UserKnownHostsFile="${known_hosts}"
  -o StrictHostKeyChecking=yes
)

wait_for_ssh() {
  local instance_id="$1"
  for _ in $(seq 1 30); do
    if "${ssh_base[@]}" -q "${instance_id}@${NSC_SSH_HOST}" true >/dev/null 2>&1; then
      return 0
    fi
    sleep 5
  done
  return 1
}

configure_builder() {
  local instance_id="$1"
  "${ssh_base[@]}" "${instance_id}@${NSC_SSH_HOST}" <<'EOF'
set -euo pipefail

if ! command -v nix >/dev/null 2>&1; then
  curl -fsSL https://install.determinate.systems/nix | sh -s -- install linux --determinate --init none --no-confirm
fi

if [ -e /nix/var/nix/profiles/default/etc/profile.d/nix-daemon.sh ]; then
  . /nix/var/nix/profiles/default/etc/profile.d/nix-daemon.sh
fi

mkdir -p /etc/nix
cat <<CFG >/etc/nix/nix.conf
build-users-group =
trusted-users = root $USER
auto-optimise-store = true
substituters = https://cache.nixos.org
builders-use-substitutes = true
CFG

mkdir -p /nix/var/nix/daemon-socket

if ! pgrep -x nix-daemon >/dev/null 2>&1; then
  nohup nix-daemon >/dev/null 2>&1 </dev/null &
fi

for _ in $(seq 1 120); do
  if [ -S /nix/var/nix/daemon-socket/socket ]; then
    exit 0
  fi
  if ! pgrep -x nix-daemon >/dev/null 2>&1; then
    nohup nix-daemon >/dev/null 2>&1 </dev/null &
  fi
  sleep 1
done

echo "nix-daemon socket never appeared" >&2
exit 1
EOF
}

printf 'Creating temporary Namespace builder (%s)\n' "${NSC_MACHINE_TYPE}" >&2
"${NSC_BIN}" create \
  --bare \
  --machine_type "${NSC_MACHINE_TYPE}" \
  --ssh_key "${ssh_key}.pub" \
  --duration "${NSC_BUILDER_DURATION}" \
  --label "burrow=true" \
  --label "purpose=hetzner-image-build" \
  --output_to "${id_file}" \
  >/dev/null

BUILDER_ID="$(tr -d '\r\n' < "${id_file}")"
if [[ -z "${BUILDER_ID}" ]]; then
  echo "nsc create did not return a builder id" >&2
  exit 1
fi

printf 'Waiting for Namespace builder %s\n' "${BUILDER_ID}" >&2
wait_for_ssh "${BUILDER_ID}"
configure_builder "${BUILDER_ID}" >&2

remote_root="burrow-image-build-${BUILDER_ID}"
remote_flake_path="./${remote_root}"
local_flake_dir="${flake_ref#path:}"
remote_build_stdout="/tmp/burrow-image-build-${BUILDER_ID}.stdout"
remote_build_stderr="/tmp/burrow-image-build-${BUILDER_ID}.stderr"

printf 'Syncing flake to Namespace builder %s\n' "${BUILDER_ID}" >&2
tar -C "${local_flake_dir}" -cf - . \
  | "${ssh_base[@]}" "${BUILDER_ID}@${NSC_SSH_HOST}" "rm -rf '${remote_root}' && mkdir -p '${remote_root}' && tar -C '${remote_root}' -xf -"

run_remote_build() {
  local remote_cmd=(
    env
    "CONFIG=${CONFIG}"
    "REMOTE_FLAKE_PATH=${remote_flake_path}"
    "REMOTE_BUILD_STDOUT=${remote_build_stdout}"
    "REMOTE_BUILD_STDERR=${remote_build_stderr}"
    bash
    -s
    --
  )
  if [[ "${#NIX_BUILD_FLAGS[@]}" -gt 0 ]]; then
    remote_cmd+=("${NIX_BUILD_FLAGS[@]}")
  fi

  "${ssh_base[@]}" "${BUILDER_ID}@${NSC_SSH_HOST}" "${remote_cmd[@]}" <<'EOF'
set -euo pipefail

config="${CONFIG}"
remote_flake_path="${REMOTE_FLAKE_PATH}"
remote_build_stdout="${REMOTE_BUILD_STDOUT}"
remote_build_stderr="${REMOTE_BUILD_STDERR}"
nix_build_cmd=(
  nix
  --extra-experimental-features
  "nix-command flakes"
  build
  "path:${remote_flake_path}#images.${config}-raw"
  --no-link
  --print-out-paths
)
if [[ "$#" -gt 0 ]]; then
  nix_build_cmd+=("$@")
fi

rm -f "${remote_build_stdout}" "${remote_build_stderr}"
if ! "${nix_build_cmd[@]}" >"${remote_build_stdout}" 2>"${remote_build_stderr}"; then
  cat "${remote_build_stderr}" >&2
  exit 1
fi
EOF
}

resolve_remote_store_path() {
  "${ssh_base[@]}" "${BUILDER_ID}@${NSC_SSH_HOST}" \
    env "REMOTE_BUILD_STDOUT=${remote_build_stdout}" "REMOTE_BUILD_STDERR=${remote_build_stderr}" bash -s <<'EOF'
set -euo pipefail

remote_build_stdout="${REMOTE_BUILD_STDOUT}"
remote_build_stderr="${REMOTE_BUILD_STDERR}"

if [[ ! -s "${remote_build_stdout}" ]]; then
  echo "remote build stdout file is missing or empty: ${remote_build_stdout}" >&2
  if [[ -s "${remote_build_stderr}" ]]; then
    cat "${remote_build_stderr}" >&2
  fi
  exit 1
fi

tail -n1 "${remote_build_stdout}"
EOF
}

resolve_remote_artifact_path() {
  local store_path="$1"
  "${ssh_base[@]}" "${BUILDER_ID}@${NSC_SSH_HOST}" \
    env "REMOTE_STORE_PATH=${store_path}" bash -s <<'EOF'
set -euo pipefail

store_path="${REMOTE_STORE_PATH}"
artifact_path="${store_path}"
if [[ -d "${artifact_path}" ]]; then
  artifact_path="$(find "${artifact_path}" -type f \( -name '*.raw' -o -name '*.raw.*' -o -name '*.img' -o -name '*.img.*' \) | sort | head -n1)"
fi
if [[ -z "${artifact_path}" || ! -f "${artifact_path}" ]]; then
  echo "unable to locate image artifact under ${store_path}" >&2
  exit 1
fi

printf '%s\n' "${artifact_path}"
EOF
}

plan_remote_artifact_transfer() {
  local artifact_path="$1"
  local compression_mode="$2"

  "${ssh_base[@]}" "${BUILDER_ID}@${NSC_SSH_HOST}" \
    env "REMOTE_ARTIFACT_PATH=${artifact_path}" "REMOTE_COMPRESSION=${compression_mode}" bash -s <<'EOF'
set -euo pipefail

artifact_path="${REMOTE_ARTIFACT_PATH}"
compression_mode="${REMOTE_COMPRESSION}"

case "${artifact_path}" in
  *.bz2)
    printf '%s\tbz2\n' "$(basename "${artifact_path}")"
    exit 0
    ;;
  *.xz)
    printf '%s\txz\n' "$(basename "${artifact_path}")"
    exit 0
    ;;
  *.zst|*.zstd)
    printf '%s\tzstd\n' "$(basename "${artifact_path}")"
    exit 0
    ;;
esac

select_compression() {
  case "${compression_mode}" in
    auto)
      if command -v zstd >/dev/null 2>&1; then
        printf 'zstd\n'
        return 0
      fi
      if command -v xz >/dev/null 2>&1; then
        printf 'xz\n'
        return 0
      fi
      printf 'none\n'
      ;;
    none|xz|zstd)
      printf '%s\n' "${compression_mode}"
      ;;
    *)
      echo "unsupported remote compression mode: ${compression_mode}" >&2
      exit 1
      ;;
  esac
}

mode="$(select_compression)"
case "${mode}" in
  none)
    printf '%s\tnone\n' "$(basename "${artifact_path}")"
    ;;
  zstd)
    printf '%s.zst\tzstd\n' "$(basename "${artifact_path}")"
    ;;
  xz)
    printf '%s.xz\txz\n' "$(basename "${artifact_path}")"
    ;;
esac
EOF
}

stream_remote_artifact() {
  local artifact_path="$1"
  local compression_mode="$2"
  local destination="$3"

  "${ssh_base[@]}" "${BUILDER_ID}@${NSC_SSH_HOST}" \
    env "REMOTE_ARTIFACT_PATH=${artifact_path}" "REMOTE_COMPRESSION=${compression_mode}" bash -s <<'EOF' > "${destination}"
set -euo pipefail

artifact_path="${REMOTE_ARTIFACT_PATH}"
compression_mode="${REMOTE_COMPRESSION}"

case "${artifact_path}" in
  *.bz2|*.xz|*.zst|*.zstd)
    cat "${artifact_path}"
    exit 0
    ;;
esac

select_compression() {
  case "${compression_mode}" in
    auto)
      if command -v zstd >/dev/null 2>&1; then
        printf 'zstd\n'
        return 0
      fi
      if command -v xz >/dev/null 2>&1; then
        printf 'xz\n'
        return 0
      fi
      printf 'none\n'
      ;;
    none|xz|zstd)
      printf '%s\n' "${compression_mode}"
      ;;
    *)
      echo "unsupported remote compression mode: ${compression_mode}" >&2
      exit 1
      ;;
  esac
}

mode="$(select_compression)"
case "${mode}" in
  none)
    cat "${artifact_path}"
    ;;
  zstd)
    if ! command -v zstd >/dev/null 2>&1; then
      echo "zstd requested but not available on Namespace builder" >&2
      exit 1
    fi
    zstd -T0 -19 -c "${artifact_path}"
    ;;
  xz)
    if ! command -v xz >/dev/null 2>&1; then
      echo "xz requested but not available on Namespace builder" >&2
      exit 1
    fi
    xz -T0 -c "${artifact_path}"
    ;;
esac
EOF
}

printf 'Building raw image on Namespace builder %s\n' "${BUILDER_ID}" >&2
run_remote_build

remote_store_path="$(resolve_remote_store_path)"
if [[ -z "${remote_store_path}" ]]; then
  echo "remote build did not return a store path" >&2
  exit 1
fi

remote_artifact_path="$(resolve_remote_artifact_path "${remote_store_path}")"
if [[ -z "${remote_artifact_path}" ]]; then
  echo "remote build did not return an artifact path" >&2
  exit 1
fi

transfer_plan="$(plan_remote_artifact_transfer "${remote_artifact_path}" "${REMOTE_COMPRESSION}")"
local_artifact_name="$(printf '%s\n' "${transfer_plan}" | cut -f1)"
transfer_compression="$(printf '%s\n' "${transfer_plan}" | cut -f2)"
if [[ -z "${local_artifact_name}" || -z "${transfer_compression}" ]]; then
  echo "unable to determine artifact transfer plan for ${remote_artifact_path}" >&2
  exit 1
fi

output_hash="$(basename "${remote_store_path}")"
output_hash="${output_hash%%-*}"
local_artifact="${TMPDIR_BURROW_NSC}/${local_artifact_name}"

printf 'Streaming built artifact back from Namespace builder %s (%s)\n' "${BUILDER_ID}" "${transfer_compression}" >&2
stream_remote_artifact "${remote_artifact_path}" "${REMOTE_COMPRESSION}" "${local_artifact}"

cmd=(
  "${SCRIPT_DIR}/hcloud-upload-nixos-image.sh"
  --config "${CONFIG}"
  --flake "${FLAKE}"
  --location "${LOCATION}"
  --token-file "${TOKEN_FILE}"
  --artifact-path "${local_artifact}"
  --output-hash "${output_hash}"
)

if [[ -n "${UPLOAD_SERVER_TYPE}" ]]; then
  cmd+=(--server-type "${UPLOAD_SERVER_TYPE}")
fi

if [[ "${NO_UPDATE}" -eq 1 ]]; then
  cmd+=(--no-update)
fi
if [[ "${#EXTRA_LABELS[@]}" -gt 0 ]]; then
  for label in "${EXTRA_LABELS[@]}"; do
    cmd+=(--label "${label}")
  done
fi

"${cmd[@]}"
