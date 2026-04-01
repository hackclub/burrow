#!/usr/bin/env bash
set -euo pipefail

authentik_url="${AUTHENTIK_URL:-https://auth.burrow.net}"
bootstrap_token="${AUTHENTIK_BOOTSTRAP_TOKEN:-}"
google_client_id="${AUTHENTIK_GOOGLE_CLIENT_ID:-}"
google_client_secret="${AUTHENTIK_GOOGLE_CLIENT_SECRET:-}"
source_slug="${AUTHENTIK_GOOGLE_SOURCE_SLUG:-google}"
source_name="${AUTHENTIK_GOOGLE_SOURCE_NAME:-Google}"
identification_stage_name="${AUTHENTIK_GOOGLE_IDENTIFICATION_STAGE_NAME:-default-authentication-identification}"
authentication_flow_slug="${AUTHENTIK_GOOGLE_AUTHENTICATION_FLOW_SLUG:-default-source-authentication}"
enrollment_flow_slug="${AUTHENTIK_GOOGLE_ENROLLMENT_FLOW_SLUG:-default-source-enrollment}"
login_mode="${AUTHENTIK_GOOGLE_LOGIN_MODE:-redirect}"
user_matching_mode="${AUTHENTIK_GOOGLE_USER_MATCHING_MODE:-email_link}"
policy_engine_mode="${AUTHENTIK_GOOGLE_POLICY_ENGINE_MODE:-any}"
google_account_map_json="${AUTHENTIK_GOOGLE_ACCOUNT_MAP_JSON:-[]}"
property_mapping_name="${AUTHENTIK_GOOGLE_PROPERTY_MAPPING_NAME:-Burrow Google Account Map}"

usage() {
  cat <<'EOF'
Usage: Scripts/authentik-sync-google-source.sh

Required environment:
  AUTHENTIK_BOOTSTRAP_TOKEN
  AUTHENTIK_GOOGLE_CLIENT_ID
  AUTHENTIK_GOOGLE_CLIENT_SECRET

Optional environment:
  AUTHENTIK_URL
  AUTHENTIK_GOOGLE_SOURCE_SLUG
  AUTHENTIK_GOOGLE_SOURCE_NAME
  AUTHENTIK_GOOGLE_IDENTIFICATION_STAGE_NAME
  AUTHENTIK_GOOGLE_AUTHENTICATION_FLOW_SLUG
  AUTHENTIK_GOOGLE_ENROLLMENT_FLOW_SLUG
  AUTHENTIK_GOOGLE_LOGIN_MODE          promoted|redirect
  AUTHENTIK_GOOGLE_USER_MATCHING_MODE  identifier|email_link|email_deny|username_link|username_deny
  AUTHENTIK_GOOGLE_POLICY_ENGINE_MODE  all|any
  AUTHENTIK_GOOGLE_ACCOUNT_MAP_JSON    JSON array of alias mappings
  AUTHENTIK_GOOGLE_PROPERTY_MAPPING_NAME
EOF
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

if [[ -z "$bootstrap_token" ]]; then
  echo "error: AUTHENTIK_BOOTSTRAP_TOKEN is required" >&2
  exit 1
fi

if [[ -z "$google_client_id" || -z "$google_client_secret" || "$google_client_id" == PENDING* || "$google_client_secret" == PENDING* ]]; then
  echo "Google OAuth credentials are not configured; skipping Authentik Google source sync." >&2
  echo "Set Authorized redirect URI in Google to ${authentik_url}/source/oauth/callback/${source_slug}/" >&2
  exit 0
fi

if ! printf '%s' "$google_account_map_json" | jq -e 'type == "array"' >/dev/null; then
  echo "error: AUTHENTIK_GOOGLE_ACCOUNT_MAP_JSON must be a JSON array" >&2
  exit 1
fi

case "$login_mode" in
  promoted|redirect) ;;
  *)
    echo "warning: unsupported AUTHENTIK_GOOGLE_LOGIN_MODE=$login_mode; falling back to redirect" >&2
    login_mode="redirect"
    ;;
esac

api() {
  local method="$1"
  local path="$2"
  local data="${3:-}"

  if [[ -n "$data" ]]; then
    curl -fsS \
      -X "$method" \
      -H "Authorization: Bearer ${bootstrap_token}" \
      -H "Content-Type: application/json" \
      -d "$data" \
      "${authentik_url}${path}"
  else
    curl -fsS \
      -X "$method" \
      -H "Authorization: Bearer ${bootstrap_token}" \
      "${authentik_url}${path}"
  fi
}

wait_for_authentik() {
  for _ in $(seq 1 90); do
    if curl -fsS "${authentik_url}/-/health/ready/" >/dev/null 2>&1; then
      return 0
    fi
    sleep 2
  done
  echo "error: Authentik did not become ready at ${authentik_url}" >&2
  exit 1
}

lookup_single_result() {
  local path="$1"
  local jq_filter="$2"

  api GET "$path" | jq -r "$jq_filter" | head -n1
}

wait_for_authentik

flow_pk="$(
  lookup_single_result \
    "/api/v3/flows/instances/?slug=${authentication_flow_slug}" \
    '.results[] | select(.slug != null) | .pk // empty'
)"
if [[ -z "$flow_pk" ]]; then
  echo "error: could not resolve Authentik authentication flow slug ${authentication_flow_slug}" >&2
  exit 1
fi

enrollment_flow_pk="$(
  lookup_single_result \
    "/api/v3/flows/instances/?slug=${enrollment_flow_slug}" \
    '.results[] | select(.slug != null) | .pk // empty'
)"
if [[ -z "$enrollment_flow_pk" ]]; then
  echo "error: could not resolve Authentik enrollment flow slug ${enrollment_flow_slug}" >&2
  exit 1
fi

identification_stage="$(
  api GET "/api/v3/stages/identification/" \
    | jq -c --arg name "$identification_stage_name" '.results[] | select(.name == $name)'
)"
if [[ -z "$identification_stage" ]]; then
  echo "error: could not resolve Authentik identification stage ${identification_stage_name}" >&2
  exit 1
fi

stage_pk="$(printf '%s\n' "$identification_stage" | jq -r '.pk')"

property_mapping_payload='[]'
if [[ "$(printf '%s' "$google_account_map_json" | jq 'length')" -gt 0 ]]; then
  alias_map_python="$(
    printf '%s' "$google_account_map_json" \
      | jq -c '
          map({
            key: (.source_email | ascii_downcase),
            value: {
              username: .username,
              email: .email,
              name: .name
            }
          })
          | from_entries
        '
  )"

  oauth_property_mapping_expression="$(
    cat <<EOF
email = (info.get("email") or "").strip().lower()
alias_map = ${alias_map_python}
mapped = alias_map.get(email)
if not mapped:
    return {}
result = {}
for key in ("username", "email", "name"):
    value = mapped.get(key)
    if value:
        result[key] = value
return result
EOF
  )"

  oauth_property_mapping_payload="$(
    jq -n \
      --arg name "$property_mapping_name" \
      --arg expression "$oauth_property_mapping_expression" \
      '{
        name: $name,
        expression: $expression
      }'
  )"

  existing_property_mapping="$(
    api GET "/api/v3/propertymappings/source/oauth/?page_size=200" \
      | jq -c --arg name "$property_mapping_name" '.results[]? | select(.name == $name)'
  )"

  if [[ -n "$existing_property_mapping" ]]; then
    property_mapping_pk="$(printf '%s\n' "$existing_property_mapping" | jq -r '.pk')"
    api PATCH "/api/v3/propertymappings/source/oauth/${property_mapping_pk}/" "$oauth_property_mapping_payload" >/dev/null
  else
    property_mapping_pk="$(
      api POST "/api/v3/propertymappings/source/oauth/" "$oauth_property_mapping_payload" \
        | jq -r '.pk // empty'
    )"
  fi

  if [[ -z "${property_mapping_pk:-}" ]]; then
    echo "error: Google OAuth property mapping did not return a primary key" >&2
    exit 1
  fi

  property_mapping_payload="$(jq -cn --arg property_mapping_pk "$property_mapping_pk" '[$property_mapping_pk]')"
fi

oauth_source_payload="$(
  jq -n \
    --arg name "$source_name" \
    --arg slug "$source_slug" \
    --arg authentication_flow "$flow_pk" \
    --arg enrollment_flow "$enrollment_flow_pk" \
    --arg user_matching_mode "$user_matching_mode" \
    --arg policy_engine_mode "$policy_engine_mode" \
    --argjson user_property_mappings "$property_mapping_payload" \
    --arg consumer_key "$google_client_id" \
    --arg consumer_secret "$google_client_secret" \
    '{
      name: $name,
      slug: $slug,
      enabled: true,
      promoted: true,
      authentication_flow: $authentication_flow,
      enrollment_flow: $enrollment_flow,
      user_property_mappings: $user_property_mappings,
      group_property_mappings: [],
      policy_engine_mode: $policy_engine_mode,
      user_matching_mode: $user_matching_mode,
      provider_type: "google",
      consumer_key: $consumer_key,
      consumer_secret: $consumer_secret
    }'
)"

existing_source="$(
  api GET "/api/v3/sources/oauth/?slug=${source_slug}" \
    | jq -c '.results[]?'
)"

if [[ -n "$existing_source" ]]; then
  source_pk="$(printf '%s\n' "$existing_source" | jq -r '.pk')"
  api PATCH "/api/v3/sources/oauth/${source_slug}/" "$oauth_source_payload" >/dev/null
else
  source_pk="$(
    api POST "/api/v3/sources/oauth/" "$oauth_source_payload" \
      | jq -r '.pk // empty'
  )"
fi

if [[ -z "$source_pk" ]]; then
  echo "error: Google OAuth source did not return a primary key" >&2
  exit 1
fi

stage_patch="$(
  printf '%s\n' "$identification_stage" \
    | jq -c \
      --arg source_pk "$source_pk" \
      --arg login_mode "$login_mode" '
      .sources = (
        if $login_mode == "redirect" then
          [$source_pk]
        else
          ([ $source_pk ] + ((.sources // []) | map(select(. != $source_pk))))
        end
      )
      | .show_source_labels = true
      | if $login_mode == "redirect" then
          .user_fields = []
        else
          .
        end
      | {
          sources,
          show_source_labels,
          user_fields
        }'
)"

api PATCH "/api/v3/stages/identification/${stage_pk}/" "$stage_patch" >/dev/null

echo "Synced Authentik Google source ${source_slug} (${source_pk}) in ${login_mode} mode."
