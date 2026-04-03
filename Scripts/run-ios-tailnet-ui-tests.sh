#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
bundle_id="${BURROW_UI_TEST_APP_BUNDLE_ID:-com.hackclub.burrow}"
simulator_name="${BURROW_UI_TEST_SIMULATOR_NAME:-iPhone 17 Pro}"
simulator_os="${BURROW_UI_TEST_SIMULATOR_OS:-26.4}"
derived_data_path="${BURROW_UI_TEST_DERIVED_DATA_PATH:-/tmp/burrow-ui-tests-deriveddata}"
source_packages_path="${BURROW_UI_TEST_SOURCE_PACKAGES_PATH:-/tmp/burrow-ui-tests-sourcepackages}"
fallback_dir="${HOME}/Library/Application Support/${bundle_id}/SimulatorFallback"
socket_path="${fallback_dir}/burrow.sock"
daemon_log="${BURROW_UI_TEST_DAEMON_LOG:-/tmp/burrow-ui-test-daemon.log}"
ui_test_email="${BURROW_UI_TEST_EMAIL:-ui-test@burrow.net}"
ui_test_username="${BURROW_UI_TEST_USERNAME:-ui-test}"
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

mkdir -p "$fallback_dir" "$derived_data_path" "$source_packages_path"
rm -f "$socket_path"

cleanup() {
  if [[ -n "${daemon_pid:-}" ]]; then
    kill "$daemon_pid" >/dev/null 2>&1 || true
    wait "$daemon_pid" >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT

cargo build -p burrow --bin burrow

(
  cd "$fallback_dir"
  BURROW_SOCKET_PATH="burrow.sock" \
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

BURROW_UI_TEST_EMAIL="$ui_test_email" \
BURROW_UI_TEST_USERNAME="$ui_test_username" \
BURROW_UI_TEST_PASSWORD="$ui_test_password" \
xcodebuild \
  -quiet \
  -skipPackagePluginValidation \
  -project "${repo_root}/Apple/Burrow.xcodeproj" \
  -scheme App \
  -configuration Debug \
  -destination "platform=iOS Simulator,name=${simulator_name},OS=${simulator_os}" \
  -derivedDataPath "$derived_data_path" \
  -clonedSourcePackagesDirPath "$source_packages_path" \
  -only-testing:BurrowUITests \
  CODE_SIGNING_ALLOWED=NO \
  test
