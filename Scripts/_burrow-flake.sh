#!/usr/bin/env bash

burrow_require_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "missing required command: $1" >&2
    exit 1
  fi
}

burrow_cleanup_flake_tmpdirs() {
  if [[ "${#BURROW_FLAKE_TMPDIRS[@]}" -eq 0 ]]; then
    return
  fi
  rm -rf "${BURROW_FLAKE_TMPDIRS[@]}"
}

burrow_prepare_flake_ref() {
  local input="${1:-.}"

  case "${input}" in
    path:*|git+*|github:*|tarball+*|http://*|https://*)
      printf '%s\n' "${input}"
      return 0
      ;;
  esac

  local resolved
  resolved="$(cd "${input}" && pwd)"

  local cache_root="${HOME}/.cache/burrow"
  mkdir -p "${cache_root}"

  local copy_root
  copy_root="$(mktemp -d "${cache_root}/flake-XXXXXX")"
  mkdir -p "${copy_root}/repo"

  rsync -a \
    --delete \
    --exclude '.git' \
    --exclude '.direnv' \
    --exclude 'result' \
    --exclude 'burrow.sock' \
    --exclude 'node_modules' \
    --exclude 'target' \
    --exclude 'build' \
    "${resolved}/" "${copy_root}/repo/"

  BURROW_FLAKE_TMPDIRS+=("${copy_root}")
  printf 'path:%s/repo\n' "${copy_root}"
}

burrow_resolve_image_artifact() {
  local store_path="$1"

  if [[ -f "${store_path}" ]]; then
    printf '%s\n' "${store_path}"
    return 0
  fi

  if [[ -d "${store_path}" ]]; then
    local candidate
    candidate="$(
      find "${store_path}" -type f \
        \( -name '*.raw' -o -name '*.raw.*' -o -name '*.img' -o -name '*.img.*' \) \
        | sort \
        | head -n1
    )"
    if [[ -n "${candidate}" ]]; then
      printf '%s\n' "${candidate}"
      return 0
    fi
  fi

  echo "unable to locate disk image artifact under ${store_path}" >&2
  exit 1
}

burrow_detect_compression() {
  local artifact="$1"

  case "${artifact}" in
    *.bz2)
      printf 'bz2\n'
      ;;
    *.xz)
      printf 'xz\n'
      ;;
    *.zst|*.zstd)
      printf 'zstd\n'
      ;;
    *)
      printf '\n'
      ;;
  esac
}
