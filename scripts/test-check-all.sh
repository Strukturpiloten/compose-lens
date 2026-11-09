#!/usr/bin/env bash
# Exercise the real gate's command dispatch without invoking compilers or downloading tools.
set -Eeuo pipefail

script_directory="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
# Capture the selected Bash before PATH is populated with mocks (macOS ships an older /bin/bash).
bash_executable="$(command -v bash)"
test_root="$(mktemp -d)"
trap 'rm -r -- "${test_root}"' EXIT
mkdir -p "${test_root}/repository/scripts" "${test_root}/repository/site" "${test_root}/bin"
cp -- "${script_directory}/check-all.sh" "${test_root}/repository/scripts/check-all.sh"
cp -- "${script_directory}/../README.md" "${test_root}/repository/README.md"

# These mock commands record exactly what the unmodified gate requests.
cat > "${test_root}/bin/mock" << 'MOCK'
#!/bin/bash
set -Eeuo pipefail
command_name="${0##*/}"
# Metadata is piped to jq, so its command order is deliberately not asserted.
case "${command_name}:$*" in
  cargo:metadata* | jq:*) ;;
  *)
    printf -v command_line '%s ' "${command_name}" "$@"
    printf '%s\n' "${command_line% }" >> "${CHECK_ALL_TEST_LOG}"
    ;;
esac
if [[ "${CHECK_ALL_TEST_FAIL:-}" == "${command_name}" ||
  "${CHECK_ALL_TEST_FAIL_COMMAND:-}" == "${command_name} $*" ]]; then
  exit 17
fi
case "${command_name}:$*" in
  git:ls-files*) printf 'README.md\0' ;;
  jq:-er\ .rust_channel) printf '9.8.7\n' ;;
  jq:*) printf '1.98.1\n' ;;
  python3:scripts/shared-ci.py\ cargo-tools\ verify*--package\ cargo-llvm-cov\ --require-candidate)
    [[ "${CARGO_HOME}" == "${CHECK_ALL_TEST_COVERAGE_HOME}" ]]
    printf '{"rust_channel":"9.8.7"}\n' ;;
  cargo:llvm-cov*)
    [[ "${CARGO_HOME}" == "${CHECK_ALL_TEST_COVERAGE_HOME}" ]]
    [[ "${CARGO_TARGET_DIR}" == */check-all/compose-lens/coverage ]]
    [[ "${RUSTUP_TOOLCHAIN}" == '9.8.7' ]] ;;
  python3:scripts/shared-ci.py\ cargo-tools\ verify*)
    [[ "${CARGO_HOME}" == "${CARGO_TARGET_DIR%/cargo-semver-checks-target}/cargo-home" ]]
    [[ "${CARGO_TARGET_DIR}" == */check-all/compose-lens/cargo-semver-checks-target ]]
    printf '{"rust_channel":"9.8.7"}\n' ;;
  python3:scripts/shared-ci.py\ run\ --event\ local\ --task\ api-development*)
    [[ "${CARGO_HOME}" == "${CARGO_TARGET_DIR%/cargo-semver-checks-target}/cargo-home" ]]
    [[ "${CARGO_TARGET_DIR}" == */check-all/compose-lens/cargo-semver-checks-target ]]
    [[ "${RUSTUP_TOOLCHAIN}" == '9.8.7' ]] ;;
esac
MOCK
chmod +x "${test_root}/bin/mock"
for tool in actionlint bash cargo cargo-deny cargo-llvm-cov cargo-semver-checks cspell curl git \
  hadolint jq lychee markdownlint-cli2 npm prettier python3 rustup shellcheck shfmt tombi uv zizmor; do
  ln -s mock "${test_root}/bin/${tool}"
done

run_gate() {
  local label=$1
  shift
  : > "${test_root}/${label}.commands"
  PATH="${test_root}/bin:${PATH}" \
    CHECK_ALL_TEST_LOG="${test_root}/${label}.commands" \
    CARGO_HOME="${test_root}/ambient-cargo-home" \
    CHECK_ALL_TEST_COVERAGE_HOME="${test_root}/ambient-cargo-home" \
    CARGO_TARGET_DIR="${CHECK_ALL_TEST_TARGET_DIR:-${test_root}/repository/target}" \
    BOXFERRY_SEMVER_RELEASE_TYPE="" PODMAN_LENS_SEMVER_CHECK=0 \
    BOXFERRY_WEBSITE_SOURCE_MODE=local \
    "${bash_executable}" "${CHECK_ALL_TEST_REPOSITORY_ROOT:-${test_root}/repository}/scripts/check-all.sh" "$@" \
    > "${test_root}/${label}.output" 2>&1
}

assert_contains() {
  if ! grep -Fxq -- "$2" "${test_root}/$1.commands"; then
    printf 'Missing expected command: %s\n' "$2" >&2
    return 1
  fi
}

export BOXFERRY_WEBSITE_FORMAT_MODE=fix
run_gate default
run_gate fix --fix
run_gate check --check

# A target owned by a former worktree must not be reused after relocation.
mkdir -p "${test_root}/relocated/scripts"
cp -- "${script_directory}/check-all.sh" "${test_root}/relocated/scripts/check-all.sh"
status=0
CHECK_ALL_TEST_REPOSITORY_ROOT="${test_root}/relocated" \
  CHECK_ALL_TEST_TARGET_DIR="${test_root}/repository/target" \
  run_gate relocated --check || status=$?
[[ "${status}" == 2 ]]
[[ "$(cat "${test_root}/relocated.commands")" == 'bash .devcontainer/verify-tools.sh --check-python-venv' ]]
grep --fixed-strings --quiet -- 'CARGO_TARGET_DIR must be inside this worktree' \
  "${test_root}/relocated.output"

# Declared-tool verification is a required preflight in both modes. Its selected
# compiler is deliberately different from the metadata MSRV to prove forwarding.
semver_verification="python3 scripts/shared-ci.py cargo-tools verify --repository-root ${test_root}/repository --package cargo-semver-checks"
for mode in default fix check; do
  assert_contains "${mode}" "${semver_verification}"
  [[ "$(grep -Fxc -- "${semver_verification}" "${test_root}/${mode}.commands")" == 1 ]]
  assert_contains "${mode}" 'python3 scripts/shared-ci.py run --event local --task api-development --report-json target/api-development-task.json'
done
for mode in fix check; do
  status=0
  CHECK_ALL_TEST_FAIL_COMMAND="${semver_verification}" run_gate invalid_semver "--${mode}" || status=$?
  [[ "${status}" == 17 ]]
  [[ "$(tail -n 1 "${test_root}/invalid_semver.commands")" == "${semver_verification}" ]]
  grep -Fq 'Verify exact local Cargo SemVer selection' "${test_root}/invalid_semver.output"
  if grep -Eq '^cargo (fmt|semver-checks)' "${test_root}/invalid_semver.commands"; then
    printf 'Invalid SemVer selection reached formatting or compatibility.\n' >&2
    exit 1
  fi
done

# Coverage shares its ambient Cargo home with the preflight, keeps isolated
# artifacts, and forwards the verified channel to clean and the ratchet.
coverage_verification="python3 scripts/shared-ci.py cargo-tools verify --repository-root ${test_root}/repository --package cargo-llvm-cov --require-candidate"
for mode in default fix check; do
  assert_contains "${mode}" "${coverage_verification}"
  [[ "$(grep -Fxc -- "${coverage_verification}" "${test_root}/${mode}.commands")" == 1 ]]
  assert_contains "${mode}" 'cargo llvm-cov clean --locked'
  assert_contains "${mode}" 'cargo llvm-cov --locked --no-clean --workspace --all-features --all-targets --summary-only --fail-under-regions 88 --fail-under-functions 87 --fail-under-lines 89'
done
for mode in fix check; do
  status=0
  CHECK_ALL_TEST_FAIL_COMMAND="${coverage_verification}" run_gate invalid_coverage "--${mode}" || status=$?
  [[ "${status}" == 17 ]]
  [[ "$(tail -n 1 "${test_root}/invalid_coverage.commands")" == "${coverage_verification}" ]]
  grep -Fq 'Verify exact local Cargo coverage selection' "${test_root}/invalid_coverage.output"
  if grep -Eq '^cargo (fmt|llvm-cov)' "${test_root}/invalid_coverage.commands"; then
    printf 'Invalid coverage selection reached formatting or coverage.\n' >&2
    exit 1
  fi
  status=0
  CHECK_ALL_TEST_FAIL_COMMAND='cargo llvm-cov clean --locked' run_gate failed_clean "--${mode}" || status=$?
  [[ "${status}" == 17 ]]
  [[ "$(tail -n 1 "${test_root}/failed_clean.commands")" == 'cargo llvm-cov clean --locked' ]]
done

api_development='python3 scripts/shared-ci.py run --event local --task api-development --report-json target/api-development-task.json'
for mode in fix check; do
  status=0
  CHECK_ALL_TEST_FAIL_COMMAND="${api_development}" run_gate api_failure "--${mode}" || status=$?
  [[ "${status}" == 17 ]]
  [[ "$(tail -n 1 "${test_root}/api_failure.commands")" == "${api_development}" ]]
  grep -Fq 'Check development API compatibility' "${test_root}/api_failure.output"
  if grep -Fq 'local validation passed' "${test_root}/api_failure.output"; then
    printf 'Failed API development check reported complete gate success.\n' >&2
    exit 1
  fi
done
diff -u "${test_root}/default.commands" "${test_root}/fix.commands"
assert_contains fix "bash scripts/check-files.sh --fix"
assert_contains check "bash scripts/check-files.sh --check"
assert_contains check "bash .devcontainer/verify-tools.sh --check-python-venv"
for mode in fix check; do
  assert_contains "${mode}" 'cargo +1.98.1 ci-check'
  assert_contains "${mode}" 'cargo +1.98.1 ci-policy'
done
[[ "$(head -n 1 "${test_root}/check.commands")" == "bash .devcontainer/verify-tools.sh --check-python-venv" ]]
if grep -q '^cargo fmt' "${test_root}/fix.commands"; then
  assert_contains fix "cargo fmt --all"
  assert_contains check "cargo fmt --all -- --check"
else
  assert_contains fix "uv run --frozen ruff format scripts tests"
  assert_contains check "uv run --frozen ruff format --check scripts tests"
  assert_contains check "uv run --frozen python scripts/generate_brand_assets.py --check"
  # The explicit flag wins over the website's existing environment default.
  export BOXFERRY_WEBSITE_FORMAT_MODE=check
  run_gate explicit_fix --fix
  run_gate environment_check
  diff -u "${test_root}/fix.commands" "${test_root}/explicit_fix.commands"
  diff -u "${test_root}/check.commands" "${test_root}/environment_check.commands"
fi

# Both modes must request every non-formatting validation step, in the same order.
for mode in fix check; do
  grep -vE '^(cargo fmt|bash scripts/check-files.sh|python3 scripts/shared-ci.py run --task python-format|uv run --frozen ruff format|uv run --frozen python scripts/generate_brand_assets.py)' \
    "${test_root}/${mode}.commands" > "${test_root}/${mode}.validation"
done
diff -u "${test_root}/fix.validation" "${test_root}/check.validation"

for invalid in --unknown --checks; do
  status=0
  run_gate invalid "${invalid}" || status=$?
  [[ "${status}" == 2 && ! -s "${test_root}/invalid.commands" ]]
done
status=0
run_gate extra --check --fix || status=$?
[[ "${status}" == 2 && ! -s "${test_root}/extra.commands" ]]

# Ordinary validation retains all checks and ends at API development validation.
# Image qualification belongs to the separately reviewed hosted lane.
for mode in default fix check; do
  if grep -Fq 'shared-ci.py devcontainer-local' "${test_root}/${mode}.commands"; then
    printf 'Ordinary validation invoked deferred image qualification.\n' >&2
    exit 1
  fi
  [[ "$(tail -n 1 "${test_root}/${mode}.commands")" == *'--task api-development'* ]]
  grep -Fq 'passed all 29 steps' "${test_root}/${mode}.output"
done

# A failed check must stop the gate and preserve its exit status in either mode.
export CHECK_ALL_TEST_FAIL=actionlint
for mode in fix check; do
  status=0
  run_gate failure "--${mode}" || status=$?
  [[ "${status}" == 17 ]]
  [[ "$(tail -n 1 "${test_root}/failure.commands")" == "actionlint" ]]
done

# Exercise the real early venv probe with controlled Python behavior. The mock
# distinguishes missing ensurepip from a venv that exists but has no pip.
mkdir -p "${test_root}/venv-bin" "${test_root}/venv-temp"
cat > "${test_root}/venv-bin/python3" << 'MOCK_PYTHON'
#!/usr/bin/env bash
set -euo pipefail
[[ "$#" == 3 && "$1" == '-m' && "$2" == 'venv' ]] || exit 71
if [[ "${PYTHON_VENV_TEST_MODE}" == 'missing' ]]; then
  exit 72
fi
mkdir -p "$3/bin"
cat > "$3/bin/python" <<'MOCK_VENV_PYTHON'
#!/usr/bin/env bash
[[ "$*" == '-I -m pip --version' && "${PYTHON_VENV_TEST_MODE}" == 'working' ]]
MOCK_VENV_PYTHON
chmod +x "$3/bin/python"
MOCK_PYTHON
chmod +x "${test_root}/venv-bin/python3"

for mode in missing no_pip working; do
  status=0
  TMPDIR="${test_root}/venv-temp" \
    PATH="${test_root}/venv-bin:${PATH}" \
    PYTHON_VENV_TEST_MODE="${mode}" \
    "${bash_executable}" "${script_directory}/../.devcontainer/verify-tools.sh" --check-python-venv \
    > "${test_root}/venv-${mode}.output" 2>&1 || status=$?
  if [[ "${mode}" == 'working' ]]; then
    [[ "${status}" == 0 ]]
  else
    [[ "${status}" == 1 ]]
    grep -Fq 'requires python3-venv with ensurepip and pip' "${test_root}/venv-${mode}.output"
  fi
  if compgen -G "${test_root}/venv-temp/composelens-venv-preflight.*" > /dev/null; then
    printf 'The venv preflight left a temporary directory behind.\n' >&2
    exit 1
  fi
done
printf 'Complete gate mode regression tests passed.\n'
