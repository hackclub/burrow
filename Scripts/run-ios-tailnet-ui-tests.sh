#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
bundle_id="${BURROW_UI_TEST_APP_BUNDLE_ID:-com.hackclub.burrow}"
simulator_name="${BURROW_UI_TEST_SIMULATOR_NAME:-iPhone 17 Pro}"
simulator_os="${BURROW_UI_TEST_SIMULATOR_OS:-26.4}"
simulator_id="${BURROW_UI_TEST_SIMULATOR_ID:-}"
derived_data_path="${BURROW_UI_TEST_DERIVED_DATA_PATH:-/tmp/burrow-ui-tests-deriveddata}"
source_packages_path="${BURROW_UI_TEST_SOURCE_PACKAGES_PATH:-/tmp/burrow-ui-tests-sourcepackages}"
fallback_dir="/tmp/${bundle_id}/SimulatorFallback"
socket_path="${fallback_dir}/burrow.sock"
tailnet_state_root="/tmp/${bundle_id}/SimulatorTailnetState"
daemon_log="${BURROW_UI_TEST_DAEMON_LOG:-/tmp/burrow-ui-test-daemon.log}"
ui_test_config_path="${BURROW_UI_TEST_CONFIG_PATH:-/tmp/burrow-ui-test-config.json}"
ui_test_runner_bundle_id="${bundle_id}.uitests.xctrunner"
ui_test_email="${BURROW_UI_TEST_EMAIL:-ui-test@burrow.net}"
ui_test_username="${BURROW_UI_TEST_USERNAME:-ui-test}"
ui_test_tailnet_mode="${BURROW_UI_TEST_TAILNET_MODE:-tailscale}"
password_secret="${repo_root}/secrets/infra/authentik-ui-test-password.age"
age_identity="${BURROW_UI_TEST_AGE_IDENTITY:-${HOME}/.ssh/id_ed25519}"

ui_test_password="${BURROW_UI_TEST_PASSWORD:-}"
if [[ -z "$ui_test_password" ]]; then
  if [[ -f "$password_secret" && -f "$age_identity" ]]; then
    ui_test_password="$(age -d -i "$age_identity" "$password_secret" | tr -d '\r\n')"
  else
    echo "error: BURROW_UI_TEST_PASSWORD is unset and ${password_secret} could not be decrypted" >&2
    exit 1
  fi
fi

rm -rf "$fallback_dir" "$tailnet_state_root"
mkdir -p "$fallback_dir" "$tailnet_state_root" "$derived_data_path" "$source_packages_path"
rm -f "$socket_path"

resolve_simulator_id() {
  xcrun simctl list devices available -j | python3 -c '
import json
import os
import sys

target_name = sys.argv[1]
target_os = sys.argv[2]
target_runtime = "com.apple.CoreSimulator.SimRuntime.iOS-" + target_os.replace(".", "-")
devices = json.load(sys.stdin).get("devices", {})
healthy = []
for runtime, entries in devices.items():
    if runtime != target_runtime:
        continue
    for entry in entries:
        if not entry.get("isAvailable", False):
            continue
        if not os.path.isdir(entry.get("dataPath", "")):
            continue
        healthy.append(entry)
for entry in healthy:
    if entry.get("name") == target_name:
        print(entry["udid"])
        raise SystemExit(0)
for entry in healthy:
    if target_name in entry.get("name", ""):
        print(entry["udid"])
        raise SystemExit(0)
raise SystemExit(1)
' "$simulator_name" "$simulator_os"
}

if [[ -z "$simulator_id" ]]; then
  simulator_id="$(resolve_simulator_id || true)"
fi

if [[ -n "$simulator_id" ]]; then
  xcrun simctl boot "$simulator_id" >/dev/null 2>&1 || true
  xcrun simctl bootstatus "$simulator_id" -b
  xcrun simctl terminate "$simulator_id" "$bundle_id" >/dev/null 2>&1 || true
  xcrun simctl terminate "$simulator_id" "$ui_test_runner_bundle_id" >/dev/null 2>&1 || true
  xcrun simctl uninstall "$simulator_id" "$bundle_id" >/dev/null 2>&1 || true
  xcrun simctl uninstall "$simulator_id" "$ui_test_runner_bundle_id" >/dev/null 2>&1 || true
  destination="id=${simulator_id}"
else
  destination="platform=iOS Simulator,name=${simulator_name},OS=${simulator_os}"
fi

cleanup() {
  rm -f "$ui_test_config_path"
  if [[ -n "${daemon_pid:-}" ]]; then
    kill "$daemon_pid" >/dev/null 2>&1 || true
    wait "$daemon_pid" >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT

umask 077
python3 - <<'PY' "$ui_test_config_path" "$ui_test_email" "$ui_test_username" "$ui_test_password" "$ui_test_tailnet_mode"
import json
import pathlib
import sys

config_path = pathlib.Path(sys.argv[1])
config_path.write_text(
    json.dumps(
        {
            "email": sys.argv[2],
            "username": sys.argv[3],
            "password": sys.argv[4],
            "mode": sys.argv[5],
        }
    ),
    encoding="utf-8",
)
PY

cargo build -p burrow --bin burrow

(
  cd "$fallback_dir"
  RUST_LOG="${BURROW_UI_TEST_RUST_LOG:-info,burrow=debug}" \
  BURROW_SOCKET_PATH="burrow.sock" \
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

common_xcodebuild_args=(
  -quiet
  -skipPackagePluginValidation
  -project "${repo_root}/Apple/Burrow.xcodeproj"
  -scheme App
  -configuration Debug
  -destination "$destination"
  -derivedDataPath "$derived_data_path"
  -clonedSourcePackagesDirPath "$source_packages_path"
  -only-testing:BurrowUITests
  -parallel-testing-enabled NO
  -maximum-concurrent-test-simulator-destinations 1
  -maximum-parallel-testing-workers 1
  CODE_SIGNING_ALLOWED=NO
)

xcodebuild \
  "${common_xcodebuild_args[@]}" \
  build-for-testing

BURROW_UI_TEST_EMAIL="$ui_test_email" \
BURROW_UI_TEST_USERNAME="$ui_test_username" \
BURROW_UI_TEST_PASSWORD="$ui_test_password" \
BURROW_UI_TEST_CONFIG_PATH="$ui_test_config_path" \
BURROW_UI_TEST_EPHEMERAL_AUTH=1 \
xcodebuild \
  "${common_xcodebuild_args[@]}" \
  test-without-building
