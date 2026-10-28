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
  shift 2
  env \
    COMPOSE_LENS_RELEASE_METADATA_TEST_MODE=1 \
    COMPOSE_LENS_TEST_CHANGELOG="${changelog}" \
    COMPOSE_LENS_TEST_LATEST_TAG="${latest_tag}" \
    COMPOSE_LENS_TEST_VERSION="${version}" \
    bash scripts/check-release-metadata.sh "$@"
}

expect_failure() {
  local label=$1
  local version=$2
  local latest_tag=$3
  shift 3
  if check_metadata "${version}" "${latest_tag}" "$@" > "${temporary_directory}/output" 2>&1; then
    printf 'Release metadata negative case unexpectedly passed: %s\n' "${label}" >&2
    exit 1
  fi
}

expect_failure_in_both_modes() {
  local mode
  for mode in --development --release; do
    expect_failure "$1 (${mode})" "$2" "$3" "${mode}"
  done
}

# A prepared but unpublished minor release must not block the next development notes.
write_changelog \
  '# Changelog' \
  '' \
  '## [Unreleased]' \
  '' \
  '### Fixed' \
  '' \
  '- Pending fix.' \
  '' \
  '## [0.10.0] - 2026-10-27' \
  '' \
  '### Added' \
  '' \
  '- Complete release notes.'
for latest_tag in v0.9.0 v0.10.0 ''; do
  check_metadata 0.10.0 "${latest_tag}" --development > /dev/null
  expect_failure 'pending notes at release boundary' 0.10.0 "${latest_tag}" --release
  if ! grep --quiet 'Unreleased must be empty' "${temporary_directory}/output"; then
    printf 'Strict release validation must diagnose pending notes.\n' >&2
    exit 1
  fi
  expect_failure 'no argument remains strict' 0.10.0 "${latest_tag}"
done

write_changelog \
  '# Changelog' \
  '' \
  '## [Unreleased]' \
  '' \
  '## [0.10.0] - 2026-10-27' \
  '' \
  '### Added' \
  '' \
  '- Complete release notes.'
for latest_tag in v0.9.0 v0.10.0 ''; do
  check_metadata 0.10.0 "${latest_tag}" --development > /dev/null
  check_metadata 0.10.0 "${latest_tag}" --release > /dev/null
  check_metadata 0.10.0 "${latest_tag}" > /dev/null
done

for argument in --unknown '' development; do
  expect_failure 'invalid argument' 0.10.0 v0.9.0 "${argument}"
  if ! grep --quiet 'Usage:' "${temporary_directory}/output"; then
    printf 'Invalid arguments must fail with usage.\n' >&2
    exit 1
  fi
done
expect_failure 'conflicting modes' 0.10.0 v0.9.0 --development --release
expect_failure 'extra argument' 0.10.0 v0.9.0 --release extra
expect_failure 'repeated argument' 0.10.0 v0.9.0 --development --development
expect_failure_in_both_modes 'newest release differs from package version' 0.9.0 v0.9.0

write_changelog \
  '# Changelog' \
  '' \
  '## [Unreleased]' \
  '' \
  '## [0.10.0] - 2026-10-27' \
  '' \
  '- First section.' \
  '' \
  '## [0.10.0] - 2026-10-27' \
  '' \
  '- Duplicate section.'
expect_failure_in_both_modes 'duplicate current-version release section' 0.10.0 v0.9.0

write_changelog \
  '# Changelog' \
  '' \
  '## [Unreleased]' \
  '' \
  '## [Unreleased]' \
  '' \
  '## [0.10.0] - 2026-10-27' \
  '' \
  '- Released fix.'
expect_failure_in_both_modes 'duplicate Unreleased section' 0.10.0 v0.9.0

write_changelog \
  '# Changelog' \
  '' \
  '## [0.10.0] - 2026-10-27' \
  '' \
  '- Released fix.'
expect_failure_in_both_modes 'missing Unreleased section' 0.10.0 v0.9.0

write_changelog \
  '# Changelog' \
  '' \
  '## [Unreleased]' \
  '' \
  '## [v0.10.0] - 2026-10-27' \
  '' \
  '- Released fix.'
expect_failure_in_both_modes 'malformed package version' v0.10.0 v0.9.0

write_changelog '# Changelog' '' '## [Unreleased]'
expect_failure_in_both_modes 'missing current-version release section' 0.10.0 v0.9.0

write_changelog \
  '# Changelog' \
  '' \
  '## [Unreleased]' \
  '' \
  '## [0.10.0] - not-a-date' \
  '' \
  '- Released fix.'
expect_failure_in_both_modes 'malformed release date' 0.10.0 v0.9.0

write_changelog '# Changelog' '' '## [Unreleased]' '' '## [0.10.0] - 2026-10-27'
expect_failure_in_both_modes 'empty release notes' 0.10.0 v0.9.0
write_changelog '# Changelog' '' '## [Unreleased]' '' '## [0.10.0] - 2026-10-27' '' '---'
expect_failure_in_both_modes 'unusable release notes' 0.10.0 v0.9.0

rm -- "${changelog}"
expect_failure_in_both_modes 'missing changelog' 0.10.0 v0.9.0

printf 'ComposeLens release metadata policy tests passed.\n'
