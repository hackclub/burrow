#!/usr/bin/env bash
set -euo pipefail

authentik_url="${AUTHENTIK_URL:-https://auth.burrow.net}"
bootstrap_token="${AUTHENTIK_BOOTSTRAP_TOKEN:-}"
provider_slug="${AUTHENTIK_TAILNET_PROVIDER_SLUG:-ts}"
authentication_flow_name="${AUTHENTIK_TAILNET_AUTHENTICATION_FLOW_NAME:-Burrow Tailnet Authentication}"
authentication_flow_slug="${AUTHENTIK_TAILNET_AUTHENTICATION_FLOW_SLUG:-burrow-tailnet-authentication}"
identification_stage_name="${AUTHENTIK_TAILNET_IDENTIFICATION_STAGE_NAME:-burrow-tailnet-identification-stage}"
password_stage_name="${AUTHENTIK_TAILNET_PASSWORD_STAGE_NAME:-burrow-tailnet-password-stage}"
user_login_stage_name="${AUTHENTIK_TAILNET_USER_LOGIN_STAGE_NAME:-burrow-tailnet-user-login-stage}"
google_source_slug="${AUTHENTIK_TAILNET_GOOGLE_SOURCE_SLUG:-google}"

usage() {
  cat <<'EOF'
Usage: Scripts/authentik-sync-tailnet-auth-flow.sh

Required environment:
  AUTHENTIK_BOOTSTRAP_TOKEN

Optional environment:
  AUTHENTIK_URL
  AUTHENTIK_TAILNET_PROVIDER_SLUG
  AUTHENTIK_TAILNET_AUTHENTICATION_FLOW_NAME
  AUTHENTIK_TAILNET_AUTHENTICATION_FLOW_SLUG
  AUTHENTIK_TAILNET_IDENTIFICATION_STAGE_NAME
  AUTHENTIK_TAILNET_PASSWORD_STAGE_NAME
  AUTHENTIK_TAILNET_USER_LOGIN_STAGE_NAME
  AUTHENTIK_TAILNET_GOOGLE_SOURCE_SLUG
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

lookup_stage_by_name() {
  local path="$1"
  local name="$2"

  api GET "${path}?page_size=200" \
    | jq -c --arg name "$name" '.results[]? | select(.name == $name)' \
    | head -n1
}

lookup_flow_pk() {
  local slug="$1"

  api GET "/api/v3/flows/instances/?slug=${slug}" \
    | jq -r '.results[]? | select(.slug != null) | .pk // empty' \
    | head -n1
}

lookup_source_pk() {
  local slug="$1"

  api GET "/api/v3/sources/oauth/?page_size=200&slug=${slug}" \
    | jq -r --arg slug "$slug" '.results[]? | select(.slug == $slug) | .pk // empty' \
    | head -n1
}

ensure_password_stage() {
  local existing payload stage_pk

  existing="$(lookup_stage_by_name "/api/v3/stages/password/" "$password_stage_name")"
  payload="$(
    jq -cn \
      --arg name "$password_stage_name" \
      '{
        name: $name,
        backends: [
          "authentik.core.auth.InbuiltBackend",
          "authentik.core.auth.TokenBackend"
        ],
        allow_show_password: false,
        failed_attempts_before_cancel: 5
      }'
  )"

  if [[ -n "$existing" ]]; then
    stage_pk="$(printf '%s\n' "$existing" | jq -r '.pk')"
    api PATCH "/api/v3/stages/password/${stage_pk}/" "$payload" >/dev/null
  else
    stage_pk="$(
      api POST "/api/v3/stages/password/" "$payload" \
        | jq -r '.pk // empty'
    )"
  fi

  printf '%s\n' "$stage_pk"
}

ensure_identification_stage() {
  local password_stage_pk="$1"
  local google_source_pk="$2"
  local existing payload stage_pk sources_json

  existing="$(lookup_stage_by_name "/api/v3/stages/identification/" "$identification_stage_name")"
  if [[ -n "$google_source_pk" ]]; then
    sources_json="$(jq -cn --arg source "$google_source_pk" '[$source]')"
  else
    sources_json='[]'
  fi

  payload="$(
    jq -cn \
      --arg name "$identification_stage_name" \
      --arg password_stage "$password_stage_pk" \
      --argjson sources "$sources_json" \
      '{
        name: $name,
        user_fields: ["username", "email"],
        password_stage: $password_stage,
        case_insensitive_matching: true,
        show_matched_user: true,
        sources: $sources,
        show_source_labels: true,
        pretend_user_exists: false,
        enable_remember_me: false
      }'
  )"

  if [[ -n "$existing" ]]; then
    stage_pk="$(printf '%s\n' "$existing" | jq -r '.pk')"
    api PATCH "/api/v3/stages/identification/${stage_pk}/" "$payload" >/dev/null
  else
    stage_pk="$(
      api POST "/api/v3/stages/identification/" "$payload" \
        | jq -r '.pk // empty'
    )"
  fi

  printf '%s\n' "$stage_pk"
}

ensure_user_login_stage() {
  local existing payload stage_pk

  existing="$(lookup_stage_by_name "/api/v3/stages/user_login/" "$user_login_stage_name")"
  payload="$(
    jq -cn \
      --arg name "$user_login_stage_name" \
      '{
        name: $name,
        session_duration: "hours=12",
        terminate_other_sessions: false,
        remember_me_offset: "seconds=0",
        network_binding: "no_binding",
        geoip_binding: "no_binding"
      }'
  )"

  if [[ -n "$existing" ]]; then
    stage_pk="$(printf '%s\n' "$existing" | jq -r '.pk')"
    api PATCH "/api/v3/stages/user_login/${stage_pk}/" "$payload" >/dev/null
  else
    stage_pk="$(
      api POST "/api/v3/stages/user_login/" "$payload" \
        | jq -r '.pk // empty'
    )"
  fi

  printf '%s\n' "$stage_pk"
}

ensure_authentication_flow() {
  local existing_pk payload

  existing_pk="$(lookup_flow_pk "$authentication_flow_slug")"
  payload="$(
    jq -cn \
      --arg name "$authentication_flow_name" \
      --arg slug "$authentication_flow_slug" \
      '{
        name: $name,
        title: $name,
        slug: $slug,
        designation: "authentication",
        policy_engine_mode: "any",
        layout: "stacked"
      }'
  )"

  if [[ -n "$existing_pk" ]]; then
    api PATCH "/api/v3/flows/instances/${authentication_flow_slug}/" "$payload" >/dev/null
    printf '%s\n' "$existing_pk"
  else
    api POST "/api/v3/flows/instances/" "$payload" \
      | jq -r '.pk // empty'
  fi
}

ensure_flow_binding() {
  local flow_pk="$1"
  local stage_pk="$2"
  local order="$3"
  local existing payload binding_pk

  existing="$(
    api GET "/api/v3/flows/bindings/?target=${flow_pk}&stage=${stage_pk}&page_size=200" \
      | jq -c '.results[]?' \
      | head -n1
  )"

  payload="$(
    jq -cn \
      --arg target "$flow_pk" \
      --arg stage "$stage_pk" \
      --argjson order "$order" \
      '{
        target: $target,
        stage: $stage,
        order: $order,
        policy_engine_mode: "any"
      }'
  )"

  if [[ -n "$existing" ]]; then
    binding_pk="$(printf '%s\n' "$existing" | jq -r '.pk')"
    api PATCH "/api/v3/flows/bindings/${binding_pk}/" "$payload" >/dev/null
  else
    api POST "/api/v3/flows/bindings/" "$payload" >/dev/null
  fi
}

wait_for_authentik

provider_pk="$(
  api GET "/api/v3/providers/oauth2/?page_size=200" \
    | jq -r --arg provider_slug "$provider_slug" '
        .results[]?
        | select(.assigned_application_slug == $provider_slug or .slug == $provider_slug)
        | .pk // empty
      ' \
    | head -n1
)"

if [[ -z "$provider_pk" ]]; then
  echo "error: could not resolve Authentik Tailnet OAuth provider ${provider_slug}" >&2
  exit 1
fi

google_source_pk="$(lookup_source_pk "$google_source_slug" || true)"
password_stage_pk="$(ensure_password_stage)"
identification_stage_pk="$(ensure_identification_stage "$password_stage_pk" "$google_source_pk")"
user_login_stage_pk="$(ensure_user_login_stage)"
authentication_flow_pk="$(ensure_authentication_flow)"

ensure_flow_binding "$authentication_flow_pk" "$identification_stage_pk" 10
ensure_flow_binding "$authentication_flow_pk" "$user_login_stage_pk" 30

api PATCH "/api/v3/providers/oauth2/${provider_pk}/" "$(
  jq -cn --arg flow "$authentication_flow_pk" '{authentication_flow: $flow}'
)" >/dev/null

echo "Synced Burrow Tailnet authentication flow for provider ${provider_slug}."
