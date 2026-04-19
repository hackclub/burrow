#!/usr/bin/env bash
set -euo pipefail

authentik_url="${AUTHENTIK_URL:-https://auth.burrow.net}"
bootstrap_token="${AUTHENTIK_BOOTSTRAP_TOKEN:-}"
application_slug="${AUTHENTIK_LINEAR_APPLICATION_SLUG:-linear}"
application_name="${AUTHENTIK_LINEAR_APPLICATION_NAME:-Linear}"
provider_name="${AUTHENTIK_LINEAR_PROVIDER_NAME:-Linear}"
launch_url="${AUTHENTIK_LINEAR_LAUNCH_URL:-https://linear.app/burrownet}"
acs_url="${AUTHENTIK_LINEAR_ACS_URL:-}"
audience="${AUTHENTIK_LINEAR_AUDIENCE:-}"
issuer="${AUTHENTIK_LINEAR_ISSUER:-${authentik_url}/application/saml/${application_slug}/metadata/}"
default_relay_state="${AUTHENTIK_LINEAR_DEFAULT_RELAY_STATE:-}"

usage() {
  cat <<'EOF'
Usage: Scripts/authentik-sync-linear-saml.sh

Required environment:
  AUTHENTIK_BOOTSTRAP_TOKEN
  AUTHENTIK_LINEAR_ACS_URL
  AUTHENTIK_LINEAR_AUDIENCE

Optional environment:
  AUTHENTIK_URL
  AUTHENTIK_LINEAR_APPLICATION_SLUG
  AUTHENTIK_LINEAR_APPLICATION_NAME
  AUTHENTIK_LINEAR_PROVIDER_NAME
  AUTHENTIK_LINEAR_LAUNCH_URL
  AUTHENTIK_LINEAR_ISSUER
  AUTHENTIK_LINEAR_DEFAULT_RELAY_STATE
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

if [[ -z "$acs_url" ]]; then
  echo "error: AUTHENTIK_LINEAR_ACS_URL is required" >&2
  exit 1
fi

if [[ -z "$audience" ]]; then
  echo "error: AUTHENTIK_LINEAR_AUDIENCE is required" >&2
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

lookup_oauth_template_field() {
  local field="$1"

  api GET "/api/v3/providers/oauth2/?page_size=200" \
    | jq -r --arg field "$field" '.results[]? | select(.assigned_application_slug == "ts") | .[$field]' \
    | head -n1
}

reconcile_property_mapping() {
  local name="$1"
  local saml_name="$2"
  local friendly_name="$3"
  local expression="$4"
  local payload existing_pk

  payload="$(
    jq -n \
      --arg name "$name" \
      --arg saml_name "$saml_name" \
      --arg friendly_name "$friendly_name" \
      --arg expression "$expression" \
      '{
        name: $name,
        saml_name: $saml_name,
        friendly_name: $friendly_name,
        expression: $expression
      }'
  )"

  existing_pk="$(
    api GET "/api/v3/propertymappings/provider/saml/?page_size=200" \
      | jq -r --arg name "$name" '.results[]? | select(.name == $name) | .pk' \
      | head -n1
  )"

  if [[ -n "$existing_pk" ]]; then
    api PATCH "/api/v3/propertymappings/provider/saml/${existing_pk}/" "$payload" >/dev/null
    printf '%s\n' "$existing_pk"
  else
    api POST "/api/v3/propertymappings/provider/saml/" "$payload" | jq -r '.pk // empty'
  fi
}

wait_for_authentik

authorization_flow="$(lookup_oauth_template_field authorization_flow)"
invalidation_flow="$(lookup_oauth_template_field invalidation_flow)"
signing_kp="$(lookup_oauth_template_field signing_key)"

if [[ -z "$authorization_flow" || -z "$invalidation_flow" || -z "$signing_kp" ]]; then
  echo "error: could not resolve Authentik provider defaults from Burrow Tailnet template" >&2
  exit 1
fi

email_mapping_pk="$(
  reconcile_property_mapping \
    "Burrow Linear SAML Email" \
    "http://schemas.xmlsoap.org/ws/2005/05/identity/claims/emailaddress" \
    "email" \
    'return request.user.email'
)"

name_mapping_pk="$(
  reconcile_property_mapping \
    "Burrow Linear SAML Name" \
    "name" \
    "name" \
    'return request.user.name or request.user.username'
)"

first_name_mapping_pk="$(
  reconcile_property_mapping \
    "Burrow Linear SAML First Name" \
    "firstName" \
    "firstName" \
    $'parts = (request.user.name or "").split(" ", 1)\nif len(parts) > 0 and parts[0]:\n    return parts[0]\nreturn request.user.username'
)"

last_name_mapping_pk="$(
  reconcile_property_mapping \
    "Burrow Linear SAML Last Name" \
    "lastName" \
    "lastName" \
    $'parts = (request.user.name or "").rsplit(" ", 1)\nif len(parts) == 2 and parts[1]:\n    return parts[1]\nreturn request.user.username'
)"

if [[ -z "$email_mapping_pk" || -z "$name_mapping_pk" || -z "$first_name_mapping_pk" || -z "$last_name_mapping_pk" ]]; then
  echo "error: failed to reconcile Linear SAML property mappings" >&2
  exit 1
fi

provider_payload="$(
  jq -n \
    --arg name "$provider_name" \
    --arg authorization_flow "$authorization_flow" \
    --arg invalidation_flow "$invalidation_flow" \
    --arg acs_url "$acs_url" \
    --arg audience "$audience" \
    --arg issuer "$issuer" \
    --arg signing_kp "$signing_kp" \
    --arg default_relay_state "$default_relay_state" \
    --arg name_id_mapping "$email_mapping_pk" \
    --arg email_mapping "$email_mapping_pk" \
    --arg name_mapping "$name_mapping_pk" \
    --arg first_name_mapping "$first_name_mapping_pk" \
    --arg last_name_mapping "$last_name_mapping_pk" \
    '{
      name: $name,
      authorization_flow: $authorization_flow,
      invalidation_flow: $invalidation_flow,
      acs_url: $acs_url,
      audience: $audience,
      issuer: $issuer,
      signing_kp: $signing_kp,
      sign_assertion: true,
      sign_response: true,
      sp_binding: "post",
      name_id_mapping: $name_id_mapping,
      property_mappings: [
        $email_mapping,
        $name_mapping,
        $first_name_mapping,
        $last_name_mapping
      ]
    }
    + (if $default_relay_state == "" then {} else {default_relay_state: $default_relay_state} end)'
)"

existing_provider="$(
  api GET "/api/v3/providers/saml/?page_size=200" \
    | jq -c \
      --arg application_slug "$application_slug" \
      --arg provider_name "$provider_name" \
      '.results[]? | select(.assigned_application_slug == $application_slug or .name == $provider_name)' \
    | head -n1
)"

if [[ -n "$existing_provider" ]]; then
  provider_pk="$(printf '%s\n' "$existing_provider" | jq -r '.pk')"
  api PATCH "/api/v3/providers/saml/${provider_pk}/" "$provider_payload" >/dev/null
else
  provider_pk="$(
    api POST "/api/v3/providers/saml/" "$provider_payload" \
      | jq -r '.pk // empty'
  )"
fi

if [[ -z "${provider_pk:-}" ]]; then
  echo "error: Linear SAML provider did not return a primary key" >&2
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
  api PATCH "/api/v3/core/applications/${application_pk}/" "$application_payload" >/dev/null
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
  echo "error: Linear SAML application did not return a primary key" >&2
  exit 1
fi

for _ in $(seq 1 30); do
  metadata_status="$(
    curl -sS \
      -o /dev/null \
      -w '%{http_code}' \
      --max-redirs 0 \
      "${authentik_url}/application/saml/${application_slug}/metadata/" \
      || true
  )"
  case "$metadata_status" in
    200|301|302|307|308)
      echo "Synced Authentik Linear SAML application ${application_slug} (${application_name})."
      exit 0
      ;;
  esac
  sleep 2
done

echo "warning: Linear SAML metadata for ${application_slug} was not immediately readable; keeping reconciled config." >&2
echo "Synced Authentik Linear SAML application ${application_slug} (${application_name})."
