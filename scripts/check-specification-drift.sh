#!/usr/bin/env bash

set -Eeuo pipefail

script_directory="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
repository_root="$(cd -- "${script_directory}/.." && pwd -P)"
readonly repository_root

snapshot_path="${repository_root}/schema/compose-spec.json"
inventory_path="${repository_root}/schema/compose-key-inventory.json"
upstream_url="${COMPOSE_SPECIFICATION_URL:-https://raw.githubusercontent.com/compose-spec/compose-spec/main/schema/compose-spec.json}"
readonly snapshot_path inventory_path upstream_url

for required_tool in awk comm curl jq mktemp sha256sum sort; do
  if ! command -v "${required_tool}" > /dev/null 2>&1; then
    printf 'ComposeLens specification-drift check requires %s.\n' "${required_tool}" >&2
    exit 2
  fi
done

upstream_snapshot="$(mktemp)"
readonly upstream_snapshot
trap 'rm -f -- "${upstream_snapshot}"' EXIT

curl --fail --location --silent --show-error "${upstream_url}" --output "${upstream_snapshot}"

expected_digest="$(jq --raw-output '.upstream.sha256' "${inventory_path}")"
snapshot_digest="$(sha256sum "${snapshot_path}" | awk '{print $1}')"
upstream_digest="$(sha256sum "${upstream_snapshot}" | awk '{print $1}')"

printf 'Committed snapshot SHA-256: %s\n' "${snapshot_digest}"
printf 'Inventory SHA-256:          %s\n' "${expected_digest}"
printf 'Upstream main SHA-256:      %s\n' "${upstream_digest}"

if [[ "${snapshot_digest}" != "${expected_digest}" ]]; then
  printf 'Committed snapshot does not match its pinned inventory digest.\n' >&2
  exit 1
fi

inventory_key_drift=false

print_key_changes() {
  local label="$1"
  local selector="$2"
  local diff_output
  diff_output="$(
    comm -3 \
      <(jq --raw-output "${selector} | keys[]" "${snapshot_path}" | sort) \
      <(jq --raw-output "${selector} | keys[]" "${upstream_snapshot}" | sort)
  )"

  printf '\n%s properties:\n' "${label}"
  if [[ -n "${diff_output}" ]]; then
    inventory_key_drift=true
    while IFS= read -r line; do
      if [[ "${line}" == $'\t'* ]]; then
        printf '  added: %s\n' "${line#$'\t'}"
      else
        printf '  removed: %s\n' "${line}"
      fi
    done <<< "${diff_output}"
  else
    printf '  no inventory-key changes\n'
  fi
}

print_key_changes "Root" '.properties'
print_key_changes "Service" ".\"\$defs\".service.properties"

if [[ "${snapshot_digest}" != "${upstream_digest}" ]]; then
  if [[ "${inventory_key_drift}" == true ]]; then
    printf '\nInventory drift detected: review every added or removed root/service key, classify it, and deliberately update the snapshot and inventory.\n' >&2
  else
    printf '\nContent-only drift detected: root and service inventory key sets are unchanged. Review upstream nested, prose, or other non-inventory schema changes, then deliberately update the snapshot metadata/digest without changing this inventory unless its key classifications change.\n' >&2
  fi
  exit 1
fi

printf '\nCompose specification snapshot matches upstream main.\n'
