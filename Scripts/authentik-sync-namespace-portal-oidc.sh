#!/usr/bin/env bash
set -euo pipefail

authentik_url="${AUTHENTIK_URL:-https://auth.burrow.net}"
bootstrap_token="${AUTHENTIK_BOOTSTRAP_TOKEN:-}"
application_slug="${AUTHENTIK_NAMESPACE_PORTAL_APPLICATION_SLUG:-namespace}"
application_name="${AUTHENTIK_NAMESPACE_PORTAL_APPLICATION_NAME:-Namespace Portal}"
provider_name="${AUTHENTIK_NAMESPACE_PORTAL_PROVIDER_NAME:-Namespace Portal}"
template_slug="${AUTHENTIK_NAMESPACE_PORTAL_TEMPLATE_SLUG:-ts}"
client_id="${AUTHENTIK_NAMESPACE_PORTAL_CLIENT_ID:-nsc.burrow.net}"
client_secret="${AUTHENTIK_NAMESPACE_PORTAL_CLIENT_SECRET:-}"
launch_url="${AUTHENTIK_NAMESPACE_PORTAL_LAUNCH_URL:-https://nsc.burrow.net/}"
redirect_uris_json="${AUTHENTIK_NAMESPACE_PORTAL_REDIRECT_URIS_JSON:-[
  \"https://nsc.burrow.net/oauth/callback\"
]}"

usage() {
  cat <<'EOF'
Usage: Scripts/authentik-sync-namespace-portal-oidc.sh

Required environment:
  AUTHENTIK_BOOTSTRAP_TOKEN

Optional environment:
  AUTHENTIK_URL
  AUTHENTIK_NAMESPACE_PORTAL_APPLICATION_SLUG
  AUTHENTIK_NAMESPACE_PORTAL_APPLICATION_NAME
  AUTHENTIK_NAMESPACE_PORTAL_PROVIDER_NAME
  AUTHENTIK_NAMESPACE_PORTAL_TEMPLATE_SLUG
  AUTHENTIK_NAMESPACE_PORTAL_CLIENT_ID
  AUTHENTIK_NAMESPACE_PORTAL_CLIENT_SECRET
  AUTHENTIK_NAMESPACE_PORTAL_LAUNCH_URL
  AUTHENTIK_NAMESPACE_PORTAL_REDIRECT_URIS_JSON
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

if ! printf '%s' "$redirect_uris_json" | jq -e 'type == "array" and length > 0' >/dev/null; then
  echo "error: AUTHENTIK_NAMESPACE_PORTAL_REDIRECT_URIS_JSON must be a non-empty JSON array" >&2
  exit 1
fi

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

api_with_status() {
  local method="$1"
  local path="$2"
  local data="${3:-}"
  local response_file status

  response_file="$(mktemp)"
  trap 'rm -f "$response_file"' RETURN

  if [[ -n "$data" ]]; then
    status="$(
      curl -sS \
        -o "$response_file" \
        -w '%{http_code}' \
        -X "$method" \
        -H "Authorization: Bearer ${bootstrap_token}" \
        -H "Content-Type: application/json" \
        -d "$data" \
        "${authentik_url}${path}"
    )"
  else
    status="$(
      curl -sS \
        -o "$response_file" \
        -w '%{http_code}' \
        -X "$method" \
        -H "Authorization: Bearer ${bootstrap_token}" \
        "${authentik_url}${path}"
    )"
  fi

  printf '%s\n' "$status"
  cat "$response_file"
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

wait_for_authentik

template_provider="$(
  api GET "/api/v3/providers/oauth2/?page_size=200" \
    | jq -c --arg template_slug "$template_slug" '.results[]? | select(.assigned_application_slug == $template_slug)' \
    | head -n1
)"

if [[ -z "$template_provider" ]]; then
  echo "error: could not resolve the Authentik OAuth provider template ${template_slug}" >&2
  exit 1
fi

authorization_flow="$(printf '%s\n' "$template_provider" | jq -r '.authorization_flow')"
invalidation_flow="$(printf '%s\n' "$template_provider" | jq -r '.invalidation_flow')"
property_mappings="$(printf '%s\n' "$template_provider" | jq -c '.property_mappings')"
signing_key="$(printf '%s\n' "$template_provider" | jq -r '.signing_key')"

provider_payload="$(
  jq -n \
    --arg name "$provider_name" \
    --arg authorization_flow "$authorization_flow" \
    --arg invalidation_flow "$invalidation_flow" \
    --arg client_id "$client_id" \
    --arg client_secret "$client_secret" \
    --arg signing_key "$signing_key" \
    --argjson property_mappings "$property_mappings" \
    --argjson redirect_uris "$redirect_uris_json" \
    '{
      name: $name,
      authorization_flow: $authorization_flow,
      invalidation_flow: $invalidation_flow,
      client_type: (if $client_secret == "" then "public" else "confidential" end),
      client_id: $client_id,
      include_claims_in_id_token: true,
      redirect_uris: ($redirect_uris | map({matching_mode: "strict", url: .})),
      property_mappings: $property_mappings,
      signing_key: $signing_key,
      issuer_mode: "per_provider",
      sub_mode: "hashed_user_id"
    }
    + (if $client_secret == "" then {} else {client_secret: $client_secret} end)'
)"

existing_provider="$(
  api GET "/api/v3/providers/oauth2/?page_size=200" \
    | jq -c \
      --arg application_slug "$application_slug" \
      --arg provider_name "$provider_name" \
      '.results[]? | select(.assigned_application_slug == $application_slug or .name == $provider_name)' \
    | head -n1
)"

if [[ -n "$existing_provider" ]]; then
  provider_pk="$(printf '%s\n' "$existing_provider" | jq -r '.pk')"
  api PATCH "/api/v3/providers/oauth2/${provider_pk}/" "$provider_payload" >/dev/null
else
  provider_pk="$(
    api POST "/api/v3/providers/oauth2/" "$provider_payload" \
      | jq -r '.pk // empty'
  )"
fi

if [[ -z "${provider_pk:-}" ]]; then
  echo "error: Namespace portal OIDC provider did not return a primary key" >&2
  exit 1
fi

application_payload="$(
  jq -n \
    --arg name "$application_name" \
    --arg slug "$application_slug" \
    --arg provider "$provider_pk" \
    --arg launch_url "$launch_url" \
    '{
      name: $name,
      slug: $slug,
      provider: ($provider | tonumber),
      meta_launch_url: $launch_url,
      open_in_new_tab: false,
      policy_engine_mode: "any"
    }'
)"

existing_application="$(
  api GET "/api/v3/core/applications/?page_size=200" \
    | jq -c --arg slug "$application_slug" '.results[]? | select(.slug == $slug)' \
    | head -n1
)"

if [[ -n "$existing_application" ]]; then
  application_pk="$(printf '%s\n' "$existing_application" | jq -r '.pk')"
else
  create_application_result="$(
    api_with_status POST "/api/v3/core/applications/" "$application_payload"
  )"
  create_application_status="$(printf '%s\n' "$create_application_result" | sed -n '1p')"
  create_application_body="$(printf '%s\n' "$create_application_result" | sed '1d')"

  if [[ "$create_application_status" =~ ^20[01]$ ]]; then
    application_pk="$(printf '%s\n' "$create_application_body" | jq -r '.pk // empty')"
  elif [[ "$create_application_status" == "400" ]] && printf '%s\n' "$create_application_body" | jq -e '
      (.slug // [] | index("Application with this slug already exists.")) != null
      or (.provider // [] | index("Application with this provider already exists.")) != null
    ' >/dev/null; then
    application_pk="existing-duplicate"
  else
    printf '%s\n' "$create_application_body" >&2
    echo "error: could not reconcile Authentik application ${application_slug}" >&2
    exit 1
  fi
fi

if [[ -z "${application_pk:-}" ]]; then
  echo "error: Namespace portal OIDC application did not return a primary key" >&2
  exit 1
fi

for _ in $(seq 1 30); do
  if curl -fsS "${authentik_url}/application/o/${application_slug}/.well-known/openid-configuration" >/dev/null 2>&1; then
    echo "Synced Authentik Namespace portal OIDC application ${application_slug} (${application_name})."
    exit 0
  fi
  sleep 2
done

echo "warning: Namespace portal OIDC issuer document for ${application_slug} was not immediately readable; keeping reconciled config." >&2
echo "Synced Authentik Namespace portal OIDC application ${application_slug} (${application_name})."
