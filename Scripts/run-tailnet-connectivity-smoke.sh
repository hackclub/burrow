#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
bundle_id="${BURROW_UI_TEST_APP_BUNDLE_ID:-com.hackclub.burrow}"
smoke_root="${BURROW_TAILNET_SMOKE_ROOT:-/tmp/burrow-tailnet-connectivity}"
socket_path="${smoke_root}/burrow.sock"
db_path="${smoke_root}/burrow.db"
daemon_log="${BURROW_TAILNET_SMOKE_DAEMON_LOG:-${smoke_root}/daemon.log}"
payload_path="${smoke_root}/tailnet.json"
authority="${BURROW_TAILNET_SMOKE_AUTHORITY:-https://ts.burrow.net}"
account_name="${BURROW_TAILNET_SMOKE_ACCOUNT:-ui-test}"
identity_name="${BURROW_TAILNET_SMOKE_IDENTITY:-apple}"
hostname="${BURROW_TAILNET_SMOKE_HOSTNAME:-burrow-apple}"
message="${BURROW_TAILNET_SMOKE_MESSAGE:-burrow-tailnet-smoke}"
timeout_ms="${BURROW_TAILNET_SMOKE_TIMEOUT_MS:-8000}"
remote_ip="${BURROW_TAILNET_SMOKE_REMOTE_IP:-}"
remote_port="${BURROW_TAILNET_SMOKE_REMOTE_PORT:-18081}"
remote_hostname="${BURROW_TAILNET_SMOKE_REMOTE_HOSTNAME:-burrow-echo}"
remote_authkey="${BURROW_TAILNET_SMOKE_REMOTE_AUTHKEY:-}"
helper_bin="${BURROW_TAILNET_SMOKE_HELPER_BIN:-${smoke_root}/tailscale-login-bridge}"
remote_state_root="${BURROW_TAILNET_SMOKE_REMOTE_STATE_ROOT:-${smoke_root}/remote-state}"
remote_stdout="${smoke_root}/remote-helper.stdout"
remote_stderr="${BURROW_TAILNET_SMOKE_REMOTE_LOG:-${smoke_root}/remote-helper.log}"

if [[ -n "${TS_AUTHKEY:-}" ]]; then
  default_tailnet_state_root="${smoke_root}/local-state"
else
  default_tailnet_state_root="/tmp/${bundle_id}/SimulatorTailnetState"
fi
tailnet_state_root="${BURROW_TAILNET_STATE_ROOT:-${default_tailnet_state_root}}"

need_login=0
if [[ -z "${TS_AUTHKEY:-}" ]] && { [[ ! -d "$tailnet_state_root" ]] || [[ -z "$(find "$tailnet_state_root" -mindepth 1 -maxdepth 2 -print -quit 2>/dev/null)" ]]; }; then
  need_login=1
fi

if [[ "$need_login" -eq 1 ]]; then
  echo "Tailnet state root is empty; running iOS login bootstrap first..."
  "${repo_root}/Scripts/run-ios-tailnet-ui-tests.sh"
fi

rm -rf "$smoke_root"
mkdir -p "$smoke_root"

cleanup() {
  rm -f "$payload_path"
  if [[ -n "${daemon_pid:-}" ]]; then
    kill "$daemon_pid" >/dev/null 2>&1 || true
    wait "$daemon_pid" >/dev/null 2>&1 || true
  fi
  if [[ -n "${remote_pid:-}" ]]; then
    kill "$remote_pid" >/dev/null 2>&1 || true
    wait "$remote_pid" >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT

wait_for_helper_listen() {
  python3 - <<'PY' "$1"
import json
import pathlib
import sys
import time

path = pathlib.Path(sys.argv[1])
deadline = time.time() + 20
while time.time() < deadline:
    if path.exists():
        with path.open("r", encoding="utf-8") as handle:
            line = handle.readline().strip()
        if line:
            hello = json.loads(line)
            print(hello["listen_addr"])
            raise SystemExit(0)
    time.sleep(0.1)
raise SystemExit("timed out waiting for helper startup line")
PY
}

wait_for_helper_ip() {
  python3 - <<'PY' "$1"
import json
import sys
import time
import urllib.request

url = sys.argv[1]
deadline = time.time() + 30
while time.time() < deadline:
    with urllib.request.urlopen(url, timeout=5) as response:
        status = json.load(response)
    if status.get("running") and status.get("tailscale_ips"):
        print(status["tailscale_ips"][0])
        raise SystemExit(0)
    time.sleep(0.25)
raise SystemExit("timed out waiting for helper to become ready")
PY
}

python3 - <<'PY' "$payload_path" "$authority" "$account_name" "$identity_name" "$hostname"
import json
import pathlib
import sys

path = pathlib.Path(sys.argv[1])
payload = {
    "authority": sys.argv[2],
    "account": sys.argv[3],
    "identity": sys.argv[4],
    "hostname": sys.argv[5],
}
path.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
PY

cargo build -p burrow --bin burrow
(
  cd "${repo_root}/Tools/tailscale-login-bridge"
  GOWORK=off go build -o "$helper_bin" .
)

if [[ -z "$remote_ip" ]]; then
  if [[ -z "$remote_authkey" ]] && { [[ ! -d "$remote_state_root" ]] || [[ -z "$(find "$remote_state_root" -mindepth 1 -maxdepth 1 -print -quit 2>/dev/null)" ]]; }; then
    echo "error: set BURROW_TAILNET_SMOKE_REMOTE_IP, BURROW_TAILNET_SMOKE_REMOTE_AUTHKEY, or BURROW_TAILNET_SMOKE_REMOTE_STATE_ROOT to an existing logged-in helper state" >&2
    exit 1
  fi

  if [[ -n "$remote_authkey" ]]; then
    rm -rf "$remote_state_root"
    mkdir -p "$remote_state_root"
  fi

  (
    cd "$repo_root"
    if [[ -n "$remote_authkey" ]]; then
      export TS_AUTHKEY="$remote_authkey"
    fi
    "$helper_bin" \
      --listen 127.0.0.1:0 \
      --state-dir "$remote_state_root" \
      --hostname "$remote_hostname" \
      --control-url "$authority" \
      --udp-echo-port "$remote_port" \
      >"$remote_stdout" 2>"$remote_stderr"
  ) &
  remote_pid=$!

  remote_listen_addr="$(wait_for_helper_listen "$remote_stdout")"
  remote_ip="$(wait_for_helper_ip "http://${remote_listen_addr}/status")"
fi

(
  cd "$smoke_root"
  RUST_LOG="${BURROW_TAILNET_SMOKE_RUST_LOG:-info,burrow=debug}" \
  BURROW_SOCKET_PATH="$socket_path" \
  BURROW_TAILSCALE_STATE_ROOT="$tailnet_state_root" \
    "${repo_root}/target/debug/burrow" daemon >"$daemon_log" 2>&1
) &
daemon_pid=$!

for _ in $(seq 1 50); do
  [[ -S "$socket_path" ]] && break
  sleep 0.2
done

if [[ ! -S "$socket_path" ]]; then
  echo "error: Burrow daemon did not create ${socket_path}" >&2
  [[ -f "$daemon_log" ]] && cat "$daemon_log" >&2
  exit 1
fi

run_burrow() {
  BURROW_SOCKET_PATH="$socket_path" \
  BURROW_TAILSCALE_STATE_ROOT="$tailnet_state_root" \
    "${repo_root}/target/debug/burrow" "$@"
}

run_burrow network-add 1 1 "$payload_path"
run_burrow start
run_burrow tunnel-config
run_burrow tailnet-udp-echo "${remote_ip}:${remote_port}" --message "$message" --timeout-ms "$timeout_ms"

echo
echo "Tailnet connectivity smoke passed."
echo "State root: $tailnet_state_root"
echo "Remote: ${remote_ip}:${remote_port}"
