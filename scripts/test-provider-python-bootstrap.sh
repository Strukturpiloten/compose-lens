#!/usr/bin/env bash

# Reproduce pip's handling of the locally downloaded provider artifact without
# contacting an index or a Compose provider.

set -Eeuo pipefail

script_directory="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
# shellcheck source=scripts/provider-python-bootstrap.sh
source "${script_directory}/provider-python-bootstrap.sh"

test_root="$(mktemp -d)"
trap 'rm -rf -- "${test_root}"' EXIT

readonly distribution='bootstrap-regression'
readonly version='1.0.0'
readonly artifact_url="https://example.invalid/files/bootstrap_regression-${version}-py3-none-any.whl"
artifact_filename="$(provider_artifact_filename "${artifact_url}")"
readonly artifact_filename
readonly artifact_directory="${test_root}/downloaded provider"
readonly artifact="${artifact_directory}/${artifact_filename}"
readonly build_directory="${test_root}/wheel"
readonly package_directory="${build_directory}/bootstrap_regression"
readonly metadata_directory="${build_directory}/bootstrap_regression-${version}.dist-info"

mkdir -p -- "${artifact_directory}" "${package_directory}" "${metadata_directory}"
printf '__version__ = "%s"\n' "${version}" > "${package_directory}/__init__.py"
printf 'Metadata-Version: 2.1\nName: %s\nVersion: %s\n' \
  "${distribution}" "${version}" > "${metadata_directory}/METADATA"
printf 'Wheel-Version: 1.0\nGenerator: ComposeLens regression\nRoot-Is-Purelib: true\nTag: py3-none-any\n' \
  > "${metadata_directory}/WHEEL"
: > "${metadata_directory}/RECORD"
(
  cd -- "${build_directory}"
  python3 -m zipfile -c "${artifact}" bootstrap_regression "bootstrap_regression-${version}.dist-info"
)

artifact_sha256="$(sha256sum "${artifact}" | cut -d ' ' -f 1)"
readonly artifact_sha256
requirement="$(hash_locked_local_wheel_requirement \
  "${distribution}" "${artifact}" "${artifact_sha256}")"
readonly requirement

expected_uri="file://${artifact// /%20}"
readonly expected_uri
readonly expected_requirement="${distribution} @ ${expected_uri} --hash=sha256:${artifact_sha256}"
if [[ "${requirement}" != "${expected_requirement}" ]]; then
  printf 'unexpected local-wheel requirement:\nexpected: %s\nactual:   %s\n' \
    "${expected_requirement}" "${requirement}" >&2
  exit 1
fi

extensionless_artifact="${test_root}/provider-artifact"
cp -- "${artifact}" "${extensionless_artifact}"
if hash_locked_local_wheel_requirement \
  "${distribution}" "${extensionless_artifact}" "${artifact_sha256}" > /dev/null 2>&1; then
  printf 'extensionless local artifacts must be rejected before pip sees them\n' >&2
  exit 1
fi

readonly requirements_file="${test_root}/requirements.txt"
printf '%s\n' "${requirement}" > "${requirements_file}"
python3 -m venv "${test_root}/venv"
PIP_NO_CACHE_DIR=1 "${test_root}/venv/bin/python" -m pip install \
  --disable-pip-version-check --no-input --no-deps --require-hashes \
  --requirement "${requirements_file}"
installed_version="$("${test_root}/venv/bin/python" -c \
  'import bootstrap_regression; print(bootstrap_regression.__version__)')"
if [[ "${installed_version}" != "${version}" ]]; then
  printf 'installed synthetic wheel has version %s, expected %s\n' \
    "${installed_version}" "${version}" >&2
  exit 1
fi

readonly wrong_hash='0000000000000000000000000000000000000000000000000000000000000000'
printf '%s @ %s --hash=sha256:%s\n' \
  "${distribution}" "${expected_uri}" "${wrong_hash}" > "${requirements_file}"
if PIP_NO_CACHE_DIR=1 "${test_root}/venv/bin/python" -m pip install \
  --disable-pip-version-check --no-input --no-deps --require-hashes --force-reinstall \
  --requirement "${requirements_file}" > /dev/null 2>&1; then
  printf 'pip accepted a local provider wheel with the wrong SHA-256 hash\n' >&2
  exit 1
fi

printf 'Provider Python bootstrap regression tests passed.\n'
