#!/usr/bin/env bash
set -euo pipefail

authentik_url="${AUTHENTIK_URL:-https://auth.burrow.net}"
bootstrap_token="${AUTHENTIK_BOOTSTRAP_TOKEN:-}"
application_slug="${AUTHENTIK_ZULIP_APPLICATION_SLUG:-zulip}"
application_name="${AUTHENTIK_ZULIP_APPLICATION_NAME:-Zulip}"
provider_name="${AUTHENTIK_ZULIP_PROVIDER_NAME:-Zulip}"
acs_url="${AUTHENTIK_ZULIP_ACS_URL:-https://chat.burrow.net/complete/saml/}"
audience="${AUTHENTIK_ZULIP_AUDIENCE:-https://chat.burrow.net}"
launch_url="${AUTHENTIK_ZULIP_LAUNCH_URL:-https://chat.burrow.net/}"
access_group="${AUTHENTIK_ZULIP_ACCESS_GROUP:-}"
issuer="${AUTHENTIK_ZULIP_ISSUER:-$authentik_url}"

usage() {
  cat <<'EOF'
Usage: Scripts/authentik-sync-zulip-saml.sh

Required environment:
  AUTHENTIK_BOOTSTRAP_TOKEN

Optional environment:
  AUTHENTIK_URL
  AUTHENTIK_ZULIP_APPLICATION_SLUG
  AUTHENTIK_ZULIP_APPLICATION_NAME
  AUTHENTIK_ZULIP_PROVIDER_NAME
  AUTHENTIK_ZULIP_ACS_URL
  AUTHENTIK_ZULIP_AUDIENCE
  AUTHENTIK_ZULIP_LAUNCH_URL
  AUTHENTIK_ZULIP_ACCESS_GROUP
  AUTHENTIK_ZULIP_ISSUER
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
    "Burrow Zulip SAML Email" \
    "http://schemas.xmlsoap.org/ws/2005/05/identity/claims/emailaddress" \
    "email" \
    'return request.user.email'
)"

name_mapping_pk="$(
  reconcile_property_mapping \
    "Burrow Zulip SAML Name" \
    "name" \
    "name" \
    'return request.user.name or request.user.username'
)"

first_name_mapping_pk="$(
  reconcile_property_mapping \
    "Burrow Zulip SAML First Name" \
    "firstName" \
    "firstName" \
    $'parts = (request.user.name or "").split(" ", 1)\nif len(parts) > 0 and parts[0]:\n    return parts[0]\nreturn request.user.username'
)"

last_name_mapping_pk="$(
  reconcile_property_mapping \
    "Burrow Zulip SAML Last Name" \
    "lastName" \
    "lastName" \
    $'parts = (request.user.name or "").rsplit(" ", 1)\nif len(parts) == 2 and parts[1]:\n    return parts[1]\nreturn request.user.username'
)"

if [[ -z "$email_mapping_pk" || -z "$name_mapping_pk" || -z "$first_name_mapping_pk" || -z "$last_name_mapping_pk" ]]; then
  echo "error: failed to reconcile Zulip SAML property mappings" >&2
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
    }'
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
  echo "error: Zulip SAML provider did not return a primary key" >&2
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
  application_pk="existing"
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
  echo "error: Zulip SAML application did not return a primary key" >&2
  exit 1
fi

if [[ -n "$access_group" ]]; then
  ensure_application_group_binding "$application_slug" "$access_group"
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
      echo "Synced Authentik Zulip SAML application ${application_slug} (${application_name})."
      exit 0
      ;;
  esac
  sleep 2
done

echo "warning: Zulip SAML metadata for ${application_slug} was not immediately readable; keeping reconciled config." >&2
echo "Synced Authentik Zulip SAML application ${application_slug} (${application_name})."
