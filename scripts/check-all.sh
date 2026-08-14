#!/usr/bin/env bash

set -Eeuo pipefail

script_directory="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
repository_root="$(cd -- "${script_directory}/.." && pwd -P)"
readonly repository_root

cd -- "${repository_root}"

current_step="preflight"
step=0
readonly total_steps=23

fail() {
  printf 'ComposeLens local validation failed: %s\n' "$1" >&2
  exit 2
}

report_failure() {
  local status=$?
  printf '\nComposeLens local validation failed during: %s (exit %d)\n' \
    "${current_step}" "${status}" >&2
  exit "${status}"
}

trap report_failure ERR

run_step() {
  local label=$1
  shift

  step=$((step + 1))
  current_step="${label}"
  printf '\n[%02d/%02d] %s\n  +' "${step}" "${total_steps}" "${label}"
  printf ' %q' "$@"
  printf '\n'
  "$@"
}

required_tools=(
  actionlint
  cargo
  cargo-deny
  cargo-llvm-cov
  cargo-semver-checks
  git
  hadolint
  jq
  lychee
  markdownlint-cli2
  prettier
  rustup
  shellcheck
  shfmt
  taplo
  zizmor
)

missing_tools=()
for tool in "${required_tools[@]}"; do
  if ! command -v "${tool}" > /dev/null 2>&1; then
    missing_tools+=("${tool}")
  fi
done

if ((${#missing_tools[@]} != 0)); then
  printf -v missing_list ' %s' "${missing_tools[@]}"
  fail "missing required tool(s):${missing_list}. Use the ComposeLens Dev Container."
fi

mapfile -d '' markdown_files < <(
  git ls-files --cached --others --exclude-standard -z -- '*.md'
)
if ((${#markdown_files[@]} == 0)); then
  fail "the repository contains no tracked or untracked Markdown files"
fi

msrv="$({
  cargo metadata --locked --no-deps --format-version 1
} | jq -er '
  [.packages[].rust_version]
  | unique
  | if length == 1 and .[0] != null
    then .[0]
    else error("workspace packages must declare one rust-version")
    end
')"
readonly msrv

if ! rustup run "${msrv}" rustc --version > /dev/null 2>&1; then
  printf 'Installing the workspace MSRV toolchain %s.\n' "${msrv}"
  if ! rustup toolchain install "${msrv}" --profile minimal; then
    rustup run "${msrv}" rustc --version > /dev/null 2>&1 ||
      fail "rustup could not install or run the workspace MSRV toolchain ${msrv}"
    printf 'The workspace MSRV toolchain %s is installed and usable.\n' "${msrv}"
  fi
fi

semver_cargo_home="${CARGO_TARGET_DIR:-${repository_root}/target}/cargo-home"
if ! mkdir -p -- "${semver_cargo_home}"; then
  fail "cannot create the cargo-semver-checks cache: ${semver_cargo_home}"
fi
if [[ ! -w "${semver_cargo_home}" ]]; then
  fail "cargo-semver-checks cache is not writable: ${semver_cargo_home}"
fi
readonly semver_cargo_home
printf 'Using isolated cargo-semver-checks cache: %s\n' "${semver_cargo_home}"

run_step "Format Rust" cargo fmt --all
run_step "Format and lint non-Rust files" bash scripts/check-files.sh --fix
run_step "Check whitespace errors" git --no-pager diff --check
run_step "Lint GitHub Actions syntax" actionlint
run_step "Audit GitHub Actions security" zizmor .github/workflows
run_step "Check all workspace targets and features" cargo ci-check
run_step "Check repository policies" cargo ci-policy
run_step "Run Clippy with warnings denied" cargo ci-clippy
run_step "Run workspace tests" cargo ci-test
run_step "Run documentation tests" cargo ci-doctest
run_step "Build documentation with warnings denied" env RUSTDOCFLAGS="-D warnings" cargo ci-doc
run_step "Check provider conformance contracts" cargo test --locked --test conformance
run_step "Check runtime conformance contracts" cargo test --locked --test runtime_conformance
run_step "Run licensed real-world fixture tests" cargo test --locked --test real_world
run_step "Check the public API contract" cargo test --locked --test public_api
run_step "Check generated rendering" cargo test --locked --test generated_rendering
run_step "Verify the release package" cargo package --locked --allow-dirty
run_step "Enforce coverage ratchets" cargo llvm-cov --locked --workspace --all-features \
  --all-targets --summary-only --fail-under-regions 88 --fail-under-functions 87 \
  --fail-under-lines 89
run_step "Check all targets with the MSRV" cargo "+${msrv}" ci-check
run_step "Check repository policies with the MSRV" cargo "+${msrv}" ci-policy
run_step "Audit dependencies, licenses, bans, and sources" cargo deny --all-features check
run_step "Check local documentation links" lychee --config lychee.toml --root-dir . --offline \
  "${markdown_files[@]}"
run_step "Check published API compatibility" env CARGO_HOME="${semver_cargo_home}" \
  cargo semver-checks check-release \
  --package compose-lens

printf '\nComposeLens local validation passed all %d steps.\n' "${total_steps}"
