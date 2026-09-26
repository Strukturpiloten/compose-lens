#!/usr/bin/env bash

set -euo pipefail

check_python_venv() (
  set -euo pipefail
  venv_probe_root="$(mktemp -d "${TMPDIR:-/tmp}/composelens-venv-preflight.XXXXXX")"
  trap 'rm -r -- "${venv_probe_root}"' EXIT
  trap 'exit 130' INT
  trap 'exit 143' TERM

  if ! python3 -m venv "${venv_probe_root}/venv" > /dev/null 2>&1 ||
    ! "${venv_probe_root}/venv/bin/python" -I -m pip --version > /dev/null 2>&1; then
    printf 'ComposeLens requires python3-venv with ensurepip and pip; rebuild the Dev Container.\n' >&2
    exit 1
  fi
)

if [[ "${1:-}" == "--check-python-venv" && "$#" -eq 1 ]]; then
  check_python_venv
  exit 0
fi
if (($# != 0)); then
  printf 'Usage: %s [--check-python-venv]\n' "$0" >&2
  exit 2
fi

check_python_venv

for cargo_directory_name in CARGO_HOME CARGO_TARGET_DIR; do
  cargo_directory="${!cargo_directory_name:-}"
  if [[ -z "${cargo_directory}" ]]; then
    printf 'ComposeLens Dev Container is missing %s.\n' "${cargo_directory_name}" >&2
    exit 1
  fi
  if [[ ! -w "${cargo_directory}" ]]; then
    sudo chown -R "$(id -u):$(id -g)" "${cargo_directory}"
  fi
  if [[ ! -w "${cargo_directory}" ]]; then
    printf 'ComposeLens Dev Container cannot make %s writable: %s\n' \
      "${cargo_directory_name}" "${cargo_directory}" >&2
    exit 1
  fi
done

tools=(
  actionlint
  cargo
  cargo-clippy
  cargo-deny
  cargo-llvm-cov
  cargo-semver-checks
  curl
  gh
  git
  hadolint
  jq
  lychee
  markdownlint-cli2
  node
  npm
  prettier
  python3
  rustc
  rustfmt
  rustup
  shellcheck
  shfmt
  tombi
  zizmor
)

for tool in "${tools[@]}"; do
  if ! command -v "${tool}" > /dev/null 2>&1; then
    printf 'ComposeLens Dev Container is missing required tool: %s\n' "${tool}" >&2
    exit 1
  fi
done

if ! rustup component list --installed | grep -q '^llvm-tools-'; then
  printf 'ComposeLens Dev Container is missing Rust component: llvm-tools-preview\n' >&2
  exit 1
fi

printf 'ComposeLens Dev Container tooling is ready.\n'
