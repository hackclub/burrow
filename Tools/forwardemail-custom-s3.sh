#!/usr/bin/env bash

set -euo pipefail
umask 077

usage() {
  cat <<'EOF'
Usage:
  Tools/forwardemail-custom-s3.sh \
    --domain burrow.net \
    --api-token-file intake/forwardemail_api_token.txt \
    --s3-endpoint https://<endpoint> \
    --s3-region <region> \
    --s3-bucket <bucket> \
    --s3-access-key-file intake/hetzner-s3-user.txt \
    --s3-secret-key-file intake/hetzner-s3-secret.txt

Options:
  --domain <domain>                 Forward Email domain to update.
  --api-token-file <path>           File containing the Forward Email API token.
  --s3-endpoint <url>               S3-compatible endpoint URL.
  --s3-region <region>              S3 region string expected by Forward Email.
  --s3-bucket <name>                Bucket used for alias backup uploads.
  --s3-access-key-file <path>       File containing the S3 access key id.
  --s3-secret-key-file <path>       File containing the S3 secret access key.
  --test-only                       Skip the update call and only test the saved connection.
  --help                            Show this help text.

Notes:
  - Secrets are passed to curl through a temporary config file to avoid putting
    them in the process list.
  - By default the script updates the domain settings and then calls
    /test-s3-connection.
  - For Hetzner Object Storage, use the regional S3 endpoint such as
    https://hel1.your-objectstorage.com, not an account alias endpoint.
EOF
}

fail() {
  printf 'error: %s\n' "$*" >&2
  exit 1
}

require_file() {
  local path="$1"
  [[ -f "$path" ]] || fail "missing file: $path"
}

read_secret() {
  local path="$1"
  local value
  value="$(tr -d '\r\n' < "$path")"
  [[ -n "$value" ]] || fail "empty secret file: $path"
  printf '%s' "$value"
}

domain=""
api_token_file=""
s3_endpoint=""
s3_region=""
s3_bucket=""
s3_access_key_file=""
s3_secret_key_file=""
test_only=false

while [[ $# -gt 0 ]]; do
  case "$1" in
    --domain)
      domain="${2:-}"
      shift 2
      ;;
    --api-token-file)
      api_token_file="${2:-}"
      shift 2
      ;;
    --s3-endpoint)
      s3_endpoint="${2:-}"
      shift 2
      ;;
    --s3-region)
      s3_region="${2:-}"
      shift 2
      ;;
    --s3-bucket)
      s3_bucket="${2:-}"
      shift 2
      ;;
    --s3-access-key-file)
      s3_access_key_file="${2:-}"
      shift 2
      ;;
    --s3-secret-key-file)
      s3_secret_key_file="${2:-}"
      shift 2
      ;;
    --test-only)
      test_only=true
      shift
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    *)
      fail "unknown argument: $1"
      ;;
  esac
done

[[ -n "$domain" ]] || fail "--domain is required"
[[ -n "$api_token_file" ]] || fail "--api-token-file is required"
[[ -n "$s3_endpoint" || "$test_only" == true ]] || fail "--s3-endpoint is required unless --test-only is set"
[[ -n "$s3_region" || "$test_only" == true ]] || fail "--s3-region is required unless --test-only is set"
[[ -n "$s3_bucket" || "$test_only" == true ]] || fail "--s3-bucket is required unless --test-only is set"
[[ -n "$s3_access_key_file" || "$test_only" == true ]] || fail "--s3-access-key-file is required unless --test-only is set"
[[ -n "$s3_secret_key_file" || "$test_only" == true ]] || fail "--s3-secret-key-file is required unless --test-only is set"

require_file "$api_token_file"
api_token="$(read_secret "$api_token_file")"

if [[ "$test_only" == false ]]; then
  require_file "$s3_access_key_file"
  require_file "$s3_secret_key_file"
  s3_access_key_id="$(read_secret "$s3_access_key_file")"
  s3_secret_access_key="$(read_secret "$s3_secret_key_file")"

  case "$s3_endpoint" in
    http://*|https://*)
      ;;
    *)
      fail "--s3-endpoint must start with http:// or https://"
      ;;
  esac
fi

curl_config="$(mktemp)"
trap 'rm -f "$curl_config"' EXIT

if [[ "$test_only" == false ]]; then
  cat >"$curl_config" <<EOF
silent
show-error
fail-with-body
url = "https://api.forwardemail.net/v1/domains/${domain}"
request = "PUT"
user = "${api_token}:"
data = "has_custom_s3=true"
data-urlencode = "s3_endpoint=${s3_endpoint}"
data-urlencode = "s3_access_key_id=${s3_access_key_id}"
data-urlencode = "s3_secret_access_key=${s3_secret_access_key}"
data-urlencode = "s3_region=${s3_region}"
data-urlencode = "s3_bucket=${s3_bucket}"
EOF

  printf 'Configuring Forward Email custom S3 for %s\n' "$domain" >&2
  curl --config "$curl_config"
  printf '\n' >&2
fi

cat >"$curl_config" <<EOF
silent
show-error
fail-with-body
url = "https://api.forwardemail.net/v1/domains/${domain}/test-s3-connection"
request = "POST"
user = "${api_token}:"
EOF

printf 'Testing Forward Email custom S3 for %s\n' "$domain" >&2
curl --config "$curl_config"
printf '\n' >&2
