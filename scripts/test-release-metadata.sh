#!/usr/bin/env bash

set -Eeuo pipefail

temporary_directory="$(mktemp -d)"
readonly temporary_directory
trap 'rm -rf -- "${temporary_directory}"' EXIT
changelog="${temporary_directory}/CHANGELOG.md"
readonly changelog

write_changelog() {
  printf '%s\n' "$@" > "${changelog}"
}

check_metadata() {
  local version=$1
  local latest_tag=$2
  env \
    COMPOSE_LENS_RELEASE_METADATA_TEST_MODE=1 \
    COMPOSE_LENS_TEST_CHANGELOG="${changelog}" \
    COMPOSE_LENS_TEST_LATEST_TAG="${latest_tag}" \
    COMPOSE_LENS_TEST_VERSION="${version}" \
    bash scripts/check-release-metadata.sh
}

expect_failure() {
  local label=$1
  local version=$2
  local latest_tag=$3
  if check_metadata "${version}" "${latest_tag}" > /dev/null 2>&1; then
    printf 'Release metadata negative case unexpectedly passed: %s\n' "${label}" >&2
    exit 1
  fi
}

write_changelog \
  '# Changelog' \
  '' \
  '## [Unreleased]' \
  '' \
  '### Fixed' \
  '' \
  '- Pending fix.' \
  '' \
  '## [0.3.0] - 2026-08-25' \
  '' \
  '### Fixed' \
  '' \
  '- Released fix.'
check_metadata 0.3.0 v0.3.0 > /dev/null

write_changelog \
  '# Changelog' \
  '' \
  '## [Unreleased]' \
  '' \
  '## [0.3.1] - 2026-08-26' \
  '' \
  '### Fixed' \
  '' \
  '- Complete release notes.'
check_metadata 0.3.1 v0.3.0 > /dev/null

write_changelog \
  '# Changelog' \
  '' \
  '## [Unreleased]' \
  '' \
  '### Fixed' \
  '' \
  '- Included code omitted from release notes.' \
  '' \
  '## [0.3.1] - 2026-08-26' \
  '' \
  '### Fixed' \
  '' \
  '- A different fix.'
expect_failure 'non-empty Unreleased during release preparation' 0.3.1 v0.3.0

write_changelog \
  '# Changelog' \
  '' \
  '## [Unreleased]' \
  '' \
  '## [0.3.1] - 2026-08-26' \
  '' \
  '### Fixed' \
  '' \
  '- Notes for the wrong package version.'
expect_failure 'newest release differs from package version' 0.3.0 v0.3.0

write_changelog \
  '# Changelog' \
  '' \
  '## [Unreleased]' \
  '' \
  '## [0.3.1] - 2026-08-26' \
  '' \
  '### Fixed' \
  '' \
  '- First section.' \
  '' \
  '## [0.3.1] - 2026-08-26' \
  '' \
  '### Fixed' \
  '' \
  '- Duplicate section.'
expect_failure 'duplicate current-version release section' 0.3.1 v0.3.0

write_changelog \
  '# Changelog' \
  '' \
  '## [Unreleased]' \
  '' \
  '## [Unreleased]' \
  '' \
  '## [0.3.0] - 2026-08-25' \
  '' \
  '### Fixed' \
  '' \
  '- Released fix.'
expect_failure 'duplicate Unreleased section' 0.3.0 v0.3.0

printf 'ComposeLens release metadata policy tests passed.\n'
