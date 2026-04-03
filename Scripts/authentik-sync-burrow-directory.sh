#!/usr/bin/env bash
set -euo pipefail

authentik_url="${AUTHENTIK_URL:-https://auth.burrow.net}"
bootstrap_token="${AUTHENTIK_BOOTSTRAP_TOKEN:-}"
directory_json="${AUTHENTIK_BURROW_DIRECTORY_JSON:-[]}"
users_group="${AUTHENTIK_BURROW_USERS_GROUP:-burrow-users}"
admins_group="${AUTHENTIK_BURROW_ADMINS_GROUP:-burrow-admins}"
forgejo_application_slug="${AUTHENTIK_FORGEJO_APPLICATION_SLUG:-}"

usage() {
  cat <<'EOF'
Usage: Scripts/authentik-sync-burrow-directory.sh

Required environment:
  AUTHENTIK_BOOTSTRAP_TOKEN
  AUTHENTIK_BURROW_DIRECTORY_JSON

Optional environment:
  AUTHENTIK_URL
  AUTHENTIK_BURROW_USERS_GROUP
  AUTHENTIK_BURROW_ADMINS_GROUP
  AUTHENTIK_FORGEJO_APPLICATION_SLUG
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

if ! printf '%s' "$directory_json" | jq -e 'type == "array"' >/dev/null; then
  echo "error: AUTHENTIK_BURROW_DIRECTORY_JSON must be a JSON array" >&2
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

  payload="$(
    jq -cn \
      --arg name "$group_name" \
      '{name: $name}'
  )"

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
    echo "error: could not create Authentik group ${group_name}" >&2
    exit 1
  fi

  printf '%s\n' "$group_pk"
}

lookup_user_pk() {
  local username="$1"

  api GET "/api/v3/core/users/?page_size=200&search=${username}" \
    | jq -r --arg username "$username" '.results[]? | select(.username == $username) | .pk // empty' \
    | head -n1
}

ensure_user() {
  local user_spec="$1"
  local username name email is_admin groups_json password_file effective_groups_json group_name
  local group_pks_json payload user_pk

  username="$(printf '%s\n' "$user_spec" | jq -r '.username')"
  name="$(printf '%s\n' "$user_spec" | jq -r '.name')"
  email="$(printf '%s\n' "$user_spec" | jq -r '.email')"
  is_admin="$(printf '%s\n' "$user_spec" | jq -r '.isAdmin // false')"
  groups_json="$(printf '%s\n' "$user_spec" | jq -c '.groups // []')"
  password_file="$(printf '%s\n' "$user_spec" | jq -r '.passwordFile // empty')"

  if [[ -z "$username" || "$username" == "null" || -z "$email" || "$email" == "null" ]]; then
    echo "error: each Burrow Authentik user requires username and email" >&2
    exit 1
  fi

  effective_groups_json="$(
    printf '%s\n' "$groups_json" \
      | jq -c --arg users_group "$users_group" --arg admins_group "$admins_group" --argjson is_admin "$is_admin" '
          . + [$users_group] + (if $is_admin then [$admins_group] else [] end) | unique
        '
  )"

  group_pks_json='[]'
  while IFS= read -r group_name; do
    group_pk="$(ensure_group "$group_name")"
    group_pks_json="$(
      jq -cn \
        --argjson current "$group_pks_json" \
        --arg next "$group_pk" \
        '$current + [$next]'
    )"
  done < <(printf '%s\n' "$effective_groups_json" | jq -r '.[]')

  payload="$(
    jq -cn \
      --arg username "$username" \
      --arg name "$name" \
      --arg email "$email" \
      --argjson groups "$group_pks_json" \
      '{
        username: $username,
        name: $name,
        email: $email,
        is_active: true,
        path: "users",
        groups: $groups
      }'
  )"

  user_pk="$(lookup_user_pk "$username")"
  if [[ -n "$user_pk" ]]; then
    api PATCH "/api/v3/core/users/${user_pk}/" "$payload" >/dev/null
  else
    user_pk="$(
      api POST "/api/v3/core/users/" "$payload" \
        | jq -r '.pk // empty'
    )"
  fi

  if [[ -z "$user_pk" ]]; then
    echo "error: could not create Authentik user ${username}" >&2
    exit 1
  fi

  if [[ -n "$password_file" ]]; then
    if [[ ! -s "$password_file" ]]; then
      echo "error: password file for Authentik user ${username} is missing: ${password_file}" >&2
      exit 1
    fi

    api POST "/api/v3/core/users/${user_pk}/set_password/" "$(
      jq -cn \
        --arg password "$(tr -d '\r\n' < "$password_file")" \
        '{password: $password}'
    )" >/dev/null
  fi
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

wait_for_authentik
ensure_group "$users_group" >/dev/null
ensure_group "$admins_group" >/dev/null

while IFS= read -r user_spec; do
  ensure_user "$user_spec"
done < <(printf '%s\n' "$directory_json" | jq -c '.[]')

if [[ -n "$forgejo_application_slug" ]]; then
  ensure_application_group_binding "$forgejo_application_slug" "$users_group"
fi

echo "Synced Burrow Authentik directory."
