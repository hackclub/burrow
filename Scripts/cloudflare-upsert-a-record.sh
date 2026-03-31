#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage: Scripts/cloudflare-upsert-a-record.sh --zone <zone> --name <fqdn> --ipv4 <address> [options]

Upsert a DNS-only or proxied Cloudflare A record without putting the API token on
the process list.

Options:
  --zone <zone>              Cloudflare zone name, for example burrow.net
  --name <fqdn>              Fully-qualified DNS record name
  --ipv4 <address>           IPv4 address for the A record
  --token-file <path>        Cloudflare API token file
                             default: intake/cloudflare-token.txt
  --ttl <seconds|auto>       Record TTL, or auto
                             default: auto
  --proxied <true|false>     Whether to proxy through Cloudflare
                             default: false
  -h, --help                 Show this help
EOF
}

ZONE_NAME=""
RECORD_NAME=""
IPV4=""
TOKEN_FILE="intake/cloudflare-token.txt"
TTL_VALUE="auto"
PROXIED="false"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --zone)
      ZONE_NAME="${2:?missing value for --zone}"
      shift 2
      ;;
    --name)
      RECORD_NAME="${2:?missing value for --name}"
      shift 2
      ;;
    --ipv4)
      IPV4="${2:?missing value for --ipv4}"
      shift 2
      ;;
    --token-file)
      TOKEN_FILE="${2:?missing value for --token-file}"
      shift 2
      ;;
    --ttl)
      TTL_VALUE="${2:?missing value for --ttl}"
      shift 2
      ;;
    --proxied)
      PROXIED="${2:?missing value for --proxied}"
      shift 2
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

if [[ -z "${ZONE_NAME}" || -z "${RECORD_NAME}" || -z "${IPV4}" ]]; then
  usage >&2
  exit 2
fi

if [[ ! -f "${TOKEN_FILE}" ]]; then
  echo "Cloudflare token file not found: ${TOKEN_FILE}" >&2
  exit 1
fi

if [[ ! "${IPV4}" =~ ^([0-9]{1,3}\.){3}[0-9]{1,3}$ ]]; then
  echo "Invalid IPv4 address: ${IPV4}" >&2
  exit 1
fi

case "${PROXIED}" in
  true|false)
    ;;
  *)
    echo "--proxied must be true or false" >&2
    exit 1
    ;;
esac

case "${TTL_VALUE}" in
  auto)
    TTL_JSON=1
    ;;
  ''|*[!0-9]*)
    echo "--ttl must be a number of seconds or auto" >&2
    exit 1
    ;;
  *)
    TTL_JSON="${TTL_VALUE}"
    ;;
esac

TOKEN="$(tr -d '\r\n' < "${TOKEN_FILE}")"
if [[ -z "${TOKEN}" ]]; then
  echo "Cloudflare token file is empty: ${TOKEN_FILE}" >&2
  exit 1
fi

cf_api() {
  local method="$1"
  local path="$2"
  local body="${3-}"
  if [[ -n "${body}" ]]; then
    curl -fsS -X "${method}" \
      -H "Authorization: Bearer ${TOKEN}" \
      -H "Content-Type: application/json" \
      --data "${body}" \
      "https://api.cloudflare.com/client/v4${path}"
  else
    curl -fsS -X "${method}" \
      -H "Authorization: Bearer ${TOKEN}" \
      -H "Content-Type: application/json" \
      "https://api.cloudflare.com/client/v4${path}"
  fi
}

zone_lookup="$(cf_api GET "/zones?name=${ZONE_NAME}&status=active")"
zone_id="$(jq -r '.result[0].id // empty' <<<"${zone_lookup}")"

if [[ -z "${zone_id}" ]]; then
  echo "Active Cloudflare zone not found: ${ZONE_NAME}" >&2
  exit 1
fi

payload="$(jq -cn \
  --arg type "A" \
  --arg name "${RECORD_NAME}" \
  --arg content "${IPV4}" \
  --argjson proxied "${PROXIED}" \
  --argjson ttl "${TTL_JSON}" \
  '{type: $type, name: $name, content: $content, proxied: $proxied, ttl: $ttl}')"

record_lookup="$(cf_api GET "/zones/${zone_id}/dns_records?type=A&name=${RECORD_NAME}")"
record_id="$(jq -r '.result[0].id // empty' <<<"${record_lookup}")"

if [[ -n "${record_id}" ]]; then
  result="$(cf_api PUT "/zones/${zone_id}/dns_records/${record_id}" "${payload}")"
  action="updated"
else
  result="$(cf_api POST "/zones/${zone_id}/dns_records" "${payload}")"
  action="created"
fi

jq -r --arg action "${action}" '
  if .success != true then
    .errors | tostring | halt_error(1)
  else
    "Cloudflare DNS " + $action + ": " + .result.name + " -> " + .result.content +
    " (proxied=" + (.result.proxied | tostring) + ", ttl=" + (.result.ttl | tostring) + ")"
  end
' <<<"${result}"
