#!/usr/bin/env bash
set -euo pipefail

source_nix_profile() {
  local candidate
  for candidate in \
    "/nix/var/nix/profiles/default/etc/profile.d/nix-daemon.sh" \
    "${HOME}/.nix-profile/etc/profile.d/nix.sh"
  do
    if [[ -f "${candidate}" ]]; then
      # shellcheck disable=SC1090
      . "${candidate}"
      return 0
    fi
  done
  return 1
}

linux_cp_supports_preserve() {
  cp --help 2>&1 | grep -q -- '--preserve'
}

ensure_root_owned_home() {
  if [[ "$(id -u)" -ne 0 ]]; then
    return 0
  fi

  if [[ ! -d "${HOME}" ]] || [[ ! -O "${HOME}" ]]; then
    export HOME="/root"
  fi

  mkdir -p "${HOME}"
}

ensure_linux_nixbld_accounts() {
  if [[ "$(id -u)" -ne 0 ]]; then
    return 0
  fi

  if command -v getent >/dev/null 2>&1 && getent group nixbld >/dev/null 2>&1; then
    return 0
  fi

  if command -v addgroup >/dev/null 2>&1 && ! command -v groupadd >/dev/null 2>&1; then
    addgroup -S nixbld >/dev/null 2>&1 || true
    for i in $(seq 1 10); do
      adduser -S -D -H -h /var/empty -s /sbin/nologin -G nixbld "nixbld${i}" >/dev/null 2>&1 || true
    done
    return 0
  fi

  if command -v groupadd >/dev/null 2>&1; then
    groupadd -r nixbld >/dev/null 2>&1 || true
    for i in $(seq 1 10); do
      useradd \
        --system \
        --no-create-home \
        --home-dir /var/empty \
        --shell /usr/sbin/nologin \
        --gid nixbld \
        "nixbld${i}" >/dev/null 2>&1 || true
    done
    return 0
  fi

  echo "linux nix bootstrap requires nixbld group creation support" >&2
  exit 1
}

ensure_linux_nix_bootstrap_prereqs() {
  if linux_cp_supports_preserve; then
    ensure_root_owned_home
    ensure_linux_nixbld_accounts
    return 0
  fi

  if command -v apk >/dev/null 2>&1; then
    apk add --no-cache coreutils xz >/dev/null
  elif command -v apt-get >/dev/null 2>&1; then
    export DEBIAN_FRONTEND=noninteractive
    apt-get update -y >/dev/null
    apt-get install -y coreutils xz-utils >/dev/null
  elif command -v dnf >/dev/null 2>&1; then
    dnf install -y coreutils xz >/dev/null
  elif command -v yum >/dev/null 2>&1; then
    yum install -y coreutils xz >/dev/null
  else
    echo "linux nix bootstrap requires GNU cp but no supported package manager was found" >&2
    exit 1
  fi

  linux_cp_supports_preserve || {
    echo "linux nix bootstrap still lacks GNU cp after installing prerequisites" >&2
    exit 1
  }

  ensure_root_owned_home
  ensure_linux_nixbld_accounts
}

if ! command -v nix >/dev/null 2>&1; then
  if ! command -v curl >/dev/null 2>&1; then
    echo "curl is required to install nix" >&2
    exit 1
  fi

  case "$(uname -s)" in
    Linux)
      ensure_linux_nix_bootstrap_prereqs
      curl -fsSL https://nixos.org/nix/install | sh -s -- --no-daemon
      ;;
    Darwin)
      installer="$(mktemp -t burrow-nix.XXXXXX)"
      trap 'rm -f "${installer}"' EXIT
      curl -fsSL -o "${installer}" https://install.determinate.systems/nix
      chmod +x "${installer}"
      if command -v sudo >/dev/null 2>&1; then
        if sudo -n true 2>/dev/null; then
          sudo -n sh "${installer}" install --no-confirm
        else
          sudo sh "${installer}" install --no-confirm
        fi
      else
        sh "${installer}" install --no-confirm
      fi
      ;;
    *)
      echo "unsupported platform for nix bootstrap: $(uname -s)" >&2
      exit 1
      ;;
  esac
fi

source_nix_profile || true
export PATH="${HOME}/.nix-profile/bin:/nix/var/nix/profiles/default/bin:/nix/var/nix/profiles/default/sbin:${PATH}"

config_root="${XDG_CONFIG_HOME:-$HOME/.config}"
config_file="${config_root}/nix/nix.conf"
if [[ -e "${config_file}" && ! -w "${config_file}" ]]; then
  config_root="$(mktemp -d -t burrow-nix-config.XXXXXX)"
  export XDG_CONFIG_HOME="${config_root}"
  config_file="${XDG_CONFIG_HOME}/nix/nix.conf"
fi

mkdir -p "$(dirname -- "${config_file}")"
cat > "${config_file}" <<'EOF'
experimental-features = nix-command flakes
sandbox = true
fallback = true
substituters = https://cache.nixos.org
trusted-public-keys = cache.nixos.org-1:6NCHdD59X431o0gWypbMrAURkbJ16ZPMQFGspcDShjY=
EOF

command -v nix >/dev/null 2>&1 || {
  echo "nix is still unavailable after bootstrap" >&2
  exit 1
}
