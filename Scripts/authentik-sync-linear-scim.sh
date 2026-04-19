#!/usr/bin/env bash
set -euo pipefail

authentik_url="${AUTHENTIK_URL:-https://auth.burrow.net}"
bootstrap_token="${AUTHENTIK_BOOTSTRAP_TOKEN:-}"
application_slug="${AUTHENTIK_LINEAR_APPLICATION_SLUG:-linear}"
provider_name="${AUTHENTIK_LINEAR_SCIM_PROVIDER_NAME:-Linear SCIM}"
scim_url="${AUTHENTIK_LINEAR_SCIM_URL:-}"
scim_token_file="${AUTHENTIK_LINEAR_SCIM_TOKEN_FILE:-}"
user_identifier="${AUTHENTIK_LINEAR_SCIM_USER_IDENTIFIER:-email}"
owner_group="${AUTHENTIK_LINEAR_OWNER_GROUP:-linear-owners}"
admin_group="${AUTHENTIK_LINEAR_ADMIN_GROUP:-linear-admins}"
guest_group="${AUTHENTIK_LINEAR_GUEST_GROUP:-linear-guests}"

usage() {
  cat <<'EOF'
Usage: Scripts/authentik-sync-linear-scim.sh

Required environment:
  AUTHENTIK_BOOTSTRAP_TOKEN
  AUTHENTIK_LINEAR_SCIM_URL
  AUTHENTIK_LINEAR_SCIM_TOKEN_FILE

Optional environment:
  AUTHENTIK_URL
  AUTHENTIK_LINEAR_APPLICATION_SLUG
  AUTHENTIK_LINEAR_SCIM_PROVIDER_NAME
  AUTHENTIK_LINEAR_SCIM_USER_IDENTIFIER
  AUTHENTIK_LINEAR_OWNER_GROUP
  AUTHENTIK_LINEAR_ADMIN_GROUP
  AUTHENTIK_LINEAR_GUEST_GROUP
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

if [[ -z "$scim_url" ]]; then
  echo "error: AUTHENTIK_LINEAR_SCIM_URL is required" >&2
  exit 1
fi

if [[ -z "$scim_token_file" || ! -s "$scim_token_file" ]]; then
  echo "error: AUTHENTIK_LINEAR_SCIM_TOKEN_FILE is required and must be readable" >&2
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

lookup_group_pk() {
  local group_name="$1"

  api GET "/api/v3/core/groups/?page_size=200&search=${group_name}" \
    | jq -r --arg name "$group_name" '.results[]? | select(.name == $name) | .pk // empty' \
    | head -n1
}

ensure_group() {
  local group_name="$1"
  local payload group_pk

  payload="$(jq -cn --arg name "$group_name" '{name: $name}')"
  group_pk="$(lookup_group_pk "$group_name")"

  if [[ -n "$group_pk" ]]; then
    api PATCH "/api/v3/core/groups/${group_pk}/" "$payload" >/dev/null
  else
    group_pk="$(
      api POST "/api/v3/core/groups/" "$payload" \
        | jq -r '.pk // empty'
    )"
  fi

  if [[ -z "$group_pk" ]]; then
    echo "error: could not reconcile Authentik group ${group_name}" >&2
    exit 1
  fi

  printf '%s\n' "$group_pk"
}

lookup_application() {
  api GET "/api/v3/core/applications/?page_size=200" \
    | jq -c --arg slug "$application_slug" '.results[]? | select(.slug == $slug)' \
    | head -n1
}

lookup_scim_provider() {
  api GET "/api/v3/providers/scim/?page_size=200" \
    | jq -c \
      --arg application_slug "$application_slug" \
      --arg provider_name "$provider_name" \
      '.results[]? | select(.assigned_backchannel_application_slug == $application_slug or .name == $provider_name)' \
    | head -n1
}

lookup_scim_mapping_pk() {
  local managed_name="$1"

  api GET "/api/v3/propertymappings/provider/scim/?page_size=200" \
    | jq -r --arg managed "$managed_name" '.results[]? | select(.managed == $managed) | .pk // empty' \
      | head -n1
}

reconcile_property_mapping() {
  local name="$1"
  local expression="$2"
  local payload existing_pk

  payload="$(
    jq -n \
      --arg name "$name" \
      --arg expression "$expression" \
      '{
        name: $name,
        expression: $expression
      }'
  )"

  existing_pk="$(
    api GET "/api/v3/propertymappings/provider/scim/?page_size=200" \
      | jq -r --arg name "$name" '.results[]? | select(.name == $name) | .pk // empty' \
      | head -n1
  )"

  if [[ -n "$existing_pk" ]]; then
    api PATCH "/api/v3/propertymappings/provider/scim/${existing_pk}/" "$payload" >/dev/null
    printf '%s\n' "$existing_pk"
  else
    api POST "/api/v3/propertymappings/provider/scim/" "$payload" \
      | jq -r '.pk // empty'
  fi
}

sync_object() {
  local provider_pk="$1"
  local model="$2"
  local object_id="$3"

  api POST "/api/v3/providers/scim/${provider_pk}/sync/object/" "$(
    jq -cn \
      --arg model "$model" \
      --arg object_id "$object_id" \
      '{
        sync_object_model: $model,
        sync_object_id: $object_id,
        override_dry_run: false
      }'
  )" >/dev/null
}

wait_for_authentik

group_mapping_pk="$(lookup_scim_mapping_pk "goauthentik.io/providers/scim/group")"
case "$user_identifier" in
  email)
    user_mapping_expression=$'# Some implementations require givenName and familyName to be set\ngivenName, familyName = request.user.name, " "\nformatted = request.user.name + " "\nif " " in request.user.name:\n    givenName, _, familyName = request.user.name.partition(" ")\n    formatted = request.user.name\n\navatar = request.user.avatar\nphotos = None\nif "://" in avatar:\n    photos = [{"value": avatar, "type": "photo"}]\n\nlocale = request.user.locale()\nif locale == "":\n    locale = None\n\nemails = []\nif request.user.email != "":\n    emails = [{\n        "value": request.user.email,\n        "type": "other",\n        "primary": True,\n    }]\n\nidentifier = request.user.email\nif identifier == "":\n    identifier = request.user.username\n\nreturn {\n    "userName": identifier,\n    "name": {\n        "formatted": formatted,\n        "givenName": givenName,\n        "familyName": familyName,\n    },\n    "displayName": request.user.name,\n    "photos": photos,\n    "locale": locale,\n    "active": request.user.is_active,\n    "emails": emails,\n}'
    ;;
  username)
    user_mapping_expression=$'# Some implementations require givenName and familyName to be set\ngivenName, familyName = request.user.name, " "\nformatted = request.user.name + " "\nif " " in request.user.name:\n    givenName, _, familyName = request.user.name.partition(" ")\n    formatted = request.user.name\n\navatar = request.user.avatar\nphotos = None\nif "://" in avatar:\n    photos = [{"value": avatar, "type": "photo"}]\n\nlocale = request.user.locale()\nif locale == "":\n    locale = None\n\nemails = []\nif request.user.email != "":\n    emails = [{\n        "value": request.user.email,\n        "type": "other",\n        "primary": True,\n    }]\nreturn {\n    "userName": request.user.username,\n    "name": {\n        "formatted": formatted,\n        "givenName": givenName,\n        "familyName": familyName,\n    },\n    "displayName": request.user.name,\n    "photos": photos,\n    "locale": locale,\n    "active": request.user.is_active,\n    "emails": emails,\n}'
    ;;
  *)
    echo "error: unsupported AUTHENTIK_LINEAR_SCIM_USER_IDENTIFIER value: ${user_identifier}" >&2
    exit 1
    ;;
esac
user_mapping_pk="$(reconcile_property_mapping "Burrow Linear SCIM User" "$user_mapping_expression")"

if [[ -z "$user_mapping_pk" || -z "$group_mapping_pk" ]]; then
  echo "error: could not resolve managed Authentik SCIM property mappings" >&2
  exit 1
fi

owner_group_pk="$(ensure_group "$owner_group")"
admin_group_pk="$(ensure_group "$admin_group")"
guest_group_pk="$(ensure_group "$guest_group")"

provider_payload="$(
  jq -n \
    --arg name "$provider_name" \
    --arg url "$scim_url" \
    --arg token "$(tr -d '\r\n' < "$scim_token_file")" \
    --arg user_mapping_pk "$user_mapping_pk" \
    --arg group_mapping_pk "$group_mapping_pk" \
    --arg owner_group_pk "$owner_group_pk" \
    --arg admin_group_pk "$admin_group_pk" \
    --arg guest_group_pk "$guest_group_pk" \
    '{
      name: $name,
      url: $url,
      token: $token,
      auth_mode: "token",
      verify_certificates: true,
      compatibility_mode: "default",
      property_mappings: [$user_mapping_pk],
      property_mappings_group: [$group_mapping_pk],
      group_filters: [
        $owner_group_pk,
        $admin_group_pk,
        $guest_group_pk
      ],
      dry_run: false
    }'
)"

existing_provider="$(lookup_scim_provider)"
if [[ -n "$existing_provider" ]]; then
  provider_pk="$(printf '%s\n' "$existing_provider" | jq -r '.pk')"
  api PATCH "/api/v3/providers/scim/${provider_pk}/" "$provider_payload" >/dev/null
else
  provider_pk="$(
    api POST "/api/v3/providers/scim/" "$provider_payload" \
      | jq -r '.pk // empty'
  )"
fi

if [[ -z "${provider_pk:-}" ]]; then
  echo "error: Linear SCIM provider did not return a primary key" >&2
  exit 1
fi

application="$(lookup_application)"
if [[ -z "$application" ]]; then
  echo "error: could not resolve Authentik application ${application_slug}" >&2
  exit 1
fi

application_payload="$(
  printf '%s\n' "$application" \
    | jq \
      --arg provider_pk "$provider_pk" \
      '{
        name: .name,
        slug: .slug,
        provider: .provider,
        backchannel_providers: ((.backchannel_providers // []) + [($provider_pk | tonumber)] | unique),
        open_in_new_tab: .open_in_new_tab,
        meta_launch_url: .meta_launch_url,
        policy_engine_mode: .policy_engine_mode
      }'
)"
api PATCH "/api/v3/core/applications/${application_slug}/" "$application_payload" >/dev/null

group_pks_json="$(jq -cn --arg owner "$owner_group_pk" --arg admin "$admin_group_pk" --arg guest "$guest_group_pk" '[$owner, $admin, $guest]')"
user_pks_json="$(
  api GET "/api/v3/core/users/?page_size=200" \
    | jq -c \
      --argjson group_pks "$group_pks_json" \
      '[.results[]?
        | select(
            ([((.groups // [])[] | tostring)] as $user_groups
            | ($group_pks | map(. as $wanted | ($user_groups | index($wanted)) != null) | any))
          )
        | .pk]'
)"

while IFS= read -r group_pk; do
  [[ -z "$group_pk" ]] && continue
  sync_object "$provider_pk" "authentik.core.models.Group" "$group_pk"
done < <(printf '%s\n' "$group_pks_json" | jq -r '.[]')

while IFS= read -r user_pk; do
  [[ -z "$user_pk" ]] && continue
  sync_object "$provider_pk" "authentik.core.models.User" "$user_pk"
done < <(printf '%s\n' "$user_pks_json" | jq -r '.[]')

status_json="$(api GET "/api/v3/providers/scim/${provider_pk}/sync/status/" || true)"
if ! printf '%s\n' "$status_json" | jq -e 'has("last_sync_status")' >/dev/null 2>&1; then
  echo "warning: could not read Linear SCIM sync status for provider ${provider_pk}; keeping reconciled configuration." >&2
fi

echo "Synced Authentik Linear SCIM provider ${provider_name} (${provider_pk}) with groups ${owner_group}, ${admin_group}, ${guest_group}."
