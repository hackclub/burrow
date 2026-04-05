#!/usr/bin/env bash
set -euo pipefail

: "${API_URL:?API_URL is required}"
: "${REPOSITORY:?REPOSITORY is required}"
: "${RELEASE_TAG:?RELEASE_TAG is required}"
: "${TOKEN:?TOKEN is required}"

release_api="${API_URL}/repos/${REPOSITORY}/releases"
tag_api="${release_api}/tags/${RELEASE_TAG}"
release_json="$(mktemp)"
create_json="$(mktemp)"
trap 'rm -f "${release_json}" "${create_json}"' EXIT

status="$(
  curl -sS -o "${release_json}" -w '%{http_code}' \
    -H "Authorization: token ${TOKEN}" \
    "${tag_api}"
)"

if [[ "${status}" == "404" ]]; then
  jq -n \
    --arg tag "${RELEASE_TAG}" \
    --arg name "Burrow ${RELEASE_TAG}" \
    '{
      tag_name: $tag,
      target_commitish: $tag,
      name: $name,
      body: "Automated prerelease built on Forgejo Namespace runners.",
      draft: false,
      prerelease: true
    }' > "${create_json}"

  curl -fsS \
    -H "Authorization: token ${TOKEN}" \
    -H "Content-Type: application/json" \
    -d @"${create_json}" \
    "${release_api}" > "${release_json}"
elif [[ "${status}" != "200" ]]; then
  echo "failed to query Forgejo release for ${RELEASE_TAG} (HTTP ${status})" >&2
  cat "${release_json}" >&2
  exit 1
fi

release_id="$(jq -r '.id' "${release_json}")"
if [[ -z "${release_id}" || "${release_id}" == "null" ]]; then
  echo "Forgejo release payload is missing an id" >&2
  cat "${release_json}" >&2
  exit 1
fi

for file in dist/*; do
  name="$(basename "${file}")"
  asset_id="$(jq -r --arg name "${name}" '.assets[]? | select(.name == $name) | .id' "${release_json}" | head -n1)"
  if [[ -n "${asset_id}" ]]; then
    curl -fsS -X DELETE \
      -H "Authorization: token ${TOKEN}" \
      "${release_api}/${release_id}/assets/${asset_id}" >/dev/null
  fi

  curl -fsS \
    -H "Authorization: token ${TOKEN}" \
    -F "attachment=@${file}" \
    "${release_api}/${release_id}/assets?name=${name}" >/dev/null
done
