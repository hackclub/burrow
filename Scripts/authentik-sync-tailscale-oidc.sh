#!/usr/bin/env bash
set -euo pipefail

authentik_url="${AUTHENTIK_URL:-https://auth.burrow.net}"
bootstrap_token="${AUTHENTIK_BOOTSTRAP_TOKEN:-}"
application_slug="${AUTHENTIK_TAILSCALE_APPLICATION_SLUG:-tailscale}"
application_name="${AUTHENTIK_TAILSCALE_APPLICATION_NAME:-Tailscale}"
provider_name="${AUTHENTIK_TAILSCALE_PROVIDER_NAME:-Tailscale}"
template_slug="${AUTHENTIK_TAILSCALE_TEMPLATE_SLUG:-ts}"
client_id="${AUTHENTIK_TAILSCALE_CLIENT_ID:-tailscale.burrow.net}"
client_secret="${AUTHENTIK_TAILSCALE_CLIENT_SECRET:-}"
launch_url="${AUTHENTIK_TAILSCALE_LAUNCH_URL:-https://login.tailscale.com/start/oidc}"
access_group="${AUTHENTIK_TAILSCALE_ACCESS_GROUP:-}"
default_external_application_slug="${AUTHENTIK_DEFAULT_EXTERNAL_APPLICATION_SLUG:-}"
redirect_uris_json="${AUTHENTIK_TAILSCALE_REDIRECT_URIS_JSON:-[
  \"https://login.tailscale.com/a/oauth_response\"
]}"

usage() {
  cat <<'EOF'
Usage: Scripts/authentik-sync-tailscale-oidc.sh

Required environment:
  AUTHENTIK_BOOTSTRAP_TOKEN
  AUTHENTIK_TAILSCALE_CLIENT_SECRET

Optional environment:
  AUTHENTIK_URL
  AUTHENTIK_TAILSCALE_APPLICATION_SLUG
  AUTHENTIK_TAILSCALE_APPLICATION_NAME
  AUTHENTIK_TAILSCALE_PROVIDER_NAME
  AUTHENTIK_TAILSCALE_TEMPLATE_SLUG
  AUTHENTIK_TAILSCALE_CLIENT_ID
  AUTHENTIK_TAILSCALE_LAUNCH_URL
  AUTHENTIK_TAILSCALE_REDIRECT_URIS_JSON
  AUTHENTIK_TAILSCALE_ACCESS_GROUP
  AUTHENTIK_DEFAULT_EXTERNAL_APPLICATION_SLUG
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

if [[ -z "$client_secret" || "$client_secret" == PENDING* ]]; then
  echo "Tailscale OIDC client secret is not configured; skipping Authentik Tailscale sync." >&2
  exit 0
fi

if ! printf '%s' "$redirect_uris_json" | jq -e 'type == "array" and length > 0' >/dev/null; then
  echo "error: AUTHENTIK_TAILSCALE_REDIRECT_URIS_JSON must be a non-empty JSON array" >&2
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

lookup_group_pk() {
  local group_name="$1"

  api GET "/api/v3/core/groups/?page_size=200" \
    | jq -r --arg group_name "$group_name" '.results[]? | select(.name == $group_name) | .pk // empty' \
    | head -n1
}

lookup_application_pk() {
  local slug="$1"

  api GET "/api/v3/core/applications/?page_size=200" \
    | jq -r --arg slug "$slug" '.results[]? | select(.slug == $slug) | .pk // empty' \
    | head -n1
}

ensure_application_group_binding() {
  local application_slug="$1"
  local group_name="$2"
  local application_pk group_pk existing payload binding_pk

  application_pk="$(lookup_application_pk "$application_slug")"
  if [[ -z "$application_pk" ]]; then
    echo "warning: could not resolve Authentik application ${application_slug}; skipping application group binding" >&2
    return 0
  fi

  group_pk="$(lookup_group_pk "$group_name")"
  if [[ -z "$group_pk" ]]; then
    echo "error: could not resolve Authentik group ${group_name}" >&2
    exit 1
  fi

  existing="$(
    api GET "/api/v3/policies/bindings/?page_size=200&target=${application_pk}" \
      | jq -c --arg group_pk "$group_pk" '.results[]? | select(.group == $group_pk)' \
      | head -n1
  )"

  payload="$(
    jq -cn \
      --arg target "$application_pk" \
      --arg group "$group_pk" \
      '{
        group: $group,
        target: $target,
        negate: false,
        enabled: true,
        order: 100,
        timeout: 30,
        failure_result: false
      }'
  )"

  if [[ -n "$existing" ]]; then
    binding_pk="$(printf '%s\n' "$existing" | jq -r '.pk')"
    api PATCH "/api/v3/policies/bindings/${binding_pk}/" "$payload" >/dev/null
  else
    api POST "/api/v3/policies/bindings/" "$payload" >/dev/null
  fi
}

ensure_default_external_application() {
  local application_slug="$1"
  local application_pk default_brand brand_payload

  application_pk="$(lookup_application_pk "$application_slug")"
  if [[ -z "$application_pk" ]]; then
    echo "error: could not resolve Authentik application ${application_slug} for brand default application" >&2
    exit 1
  fi

  default_brand="$(
    api GET "/api/v3/core/brands/?page_size=200" \
      | jq -c '.results[]? | select(.default == true)' \
      | head -n1
  )"

  if [[ -z "$default_brand" ]]; then
    echo "warning: could not resolve the default Authentik brand; skipping external default application" >&2
    return 0
  fi

  brand_payload="$(
    printf '%s\n' "$default_brand" \
      | jq --arg application_pk "$application_pk" '.default_application = $application_pk'
  )"

  api PUT "/api/v3/core/brands/$(printf '%s\n' "$default_brand" | jq -r '.brand_uuid')/" "$brand_payload" >/dev/null
}

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
      client_type: "confidential",
      client_id: $client_id,
      client_secret: $client_secret,
      include_claims_in_id_token: true,
      redirect_uris: ($redirect_uris | map({matching_mode: "strict", url: .})),
      property_mappings: $property_mappings,
      signing_key: $signing_key,
      issuer_mode: "per_provider",
      sub_mode: "hashed_user_id"
    }'
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
  echo "error: Tailscale OIDC provider did not return a primary key" >&2
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
      open_in_new_tab: true,
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
  api PATCH "/api/v3/core/applications/${application_slug}/" "$application_payload" >/dev/null
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
  echo "error: Tailscale OIDC application did not return a primary key" >&2
  exit 1
fi

if [[ -n "$access_group" ]]; then
  ensure_application_group_binding "$application_slug" "$access_group"
fi

if [[ -n "$default_external_application_slug" ]]; then
  ensure_default_external_application "$default_external_application_slug"
fi

for _ in $(seq 1 30); do
  if curl -fsS "${authentik_url}/application/o/${application_slug}/.well-known/openid-configuration" >/dev/null 2>&1; then
    echo "Synced Authentik Tailscale OIDC application ${application_slug} (${application_name})."
    exit 0
  fi
  sleep 2
done

echo "warning: Tailscale OIDC issuer document for ${application_slug} was not immediately readable; keeping reconciled config." >&2
echo "Synced Authentik Tailscale OIDC application ${application_slug} (${application_name})."
