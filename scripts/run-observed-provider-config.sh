#!/usr/bin/env bash
# Execute only the reviewed provider-configuration rows from the repository matrix.
#
# This is a release-validation worker, not a record-promoting tool.  It writes fresh,
# unreviewed captures beneath its caller-selected directory and never contacts a
# container runtime.  The Rust harness supplies a cleared environment for every probe.
set -euo pipefail

script_directory="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
# shellcheck source=scripts/provider-python-bootstrap.sh
source "${script_directory}/provider-python-bootstrap.sh"

if [[ $# != 2 ]]; then
  echo "usage: $0 TARGET OUTPUT_DIRECTORY" >&2
  exit 64
fi

readonly target="$1"
readonly output_directory="$2"
repository_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly repository_root
readonly matrix="${repository_root}/conformance/provider-config-matrix.toml"

if [[ ! "${target}" =~ ^[a-z0-9-]+$ ]] || [[ ! "${output_directory}" = /* ]] || [[ -e "${output_directory}" ]]; then
  echo 'target must be a matrix slug and output directory must be a new absolute path.' >&2
  exit 64
fi

metadata="$(
  python3 - "${matrix}" "${target}" << 'PY'
import sys
import tomllib

matrix = tomllib.load(open(sys.argv[1], "rb"))
target_id = sys.argv[2]
target = next((entry for entry in matrix["targets"] if entry["id"] == target_id), None)
if target is None:
    raise SystemExit("selected target is not in the provider matrix")
probes = [run["probe"] for run in matrix["runs"] if run["target"] == target_id and run["status"] == "observed"]
planned = [run["probe"] for run in matrix["runs"] if run["target"] == target_id and run["status"] == "planned"]
if len(probes) != 8 or not planned:
    raise SystemExit("selected target must retain exactly eight observed rows and planned rows")
requirements = []
for entry in target["execution"]:
    if not entry.startswith(("python-dotenv=", "PyYAML=")):
        continue
    package, separator, digest = entry.partition(";sha256=")
    if not separator or len(digest) != 64 or any(character not in "0123456789abcdef" for character in digest):
        raise SystemExit("bootstrap requirements must carry one lowercase SHA-256 hash")
    requirements.append(f"{package.replace('=', '==', 1)}|--hash=sha256:{digest}")
runtimes = [entry.removeprefix("python=") for entry in target["execution"] if entry.startswith("python=")]
if target["provider"] == "podman-compose" and (len(requirements) != 2 or len(runtimes) != 1):
    raise SystemExit("podman-compose target must declare its Python runtime and two bootstrap pins in execution")
print(target["provider"])
print(target["artifact_url"])
print(target["artifact_sha256"])
print(" ".join(probes))
print(" ".join(requirements))
print(runtimes[0] if runtimes else "-")
print(target["version"])
PY
)"
mapfile -t fields <<< "${metadata}"
readonly provider="${fields[0]}"
readonly artifact_url="${fields[1]}"
readonly artifact_sha256="${fields[2]}"
readonly probes="${fields[3]}"
readonly python_requirements="${fields[4]}"
readonly python_runtime="${fields[5]}"
readonly provider_version="${fields[6]}"
readonly conformance_runner_label="${COMPOSE_LENS_CONFORMANCE_RUNNER_LABEL:?workflow must provide the provider conformance runner label}"
if [[ ! "${conformance_runner_label}" =~ ^ubuntu-[0-9]+\.[0-9]+$ ]]; then
  echo 'provider conformance runner label is not the reviewed Ubuntu runner contract.' >&2
  exit 64
fi
readonly conformance_platform="github-actions-${conformance_runner_label/./-}_provider-config-only_runtime-not-invoked"
read -r -a python_requirement_array <<< "${python_requirements}"

scratch="$(mktemp -d)"
cleanup() { rm -rf "${scratch}"; }
trap cleanup EXIT
mkdir -p "${output_directory}"
artifact_filename="$(provider_artifact_filename "${artifact_url}")"
readonly artifact_filename
artifact="${scratch}/${artifact_filename}"
curl --fail --location --retry 3 --retry-all-errors --connect-timeout 15 --max-time 180 \
  --output "${artifact}" "${artifact_url}"
printf '%s  %s\n' "${artifact_sha256}" "${artifact}" | sha256sum --check --status

case "${provider}" in
  docker-compose)
    launcher="${scratch}/docker-compose"
    mv "${artifact}" "${launcher}"
    chmod 0755 "${launcher}"
    command_path='/usr/bin:/bin'
    ;;
  podman-compose)
    actual_python_runtime="$(python3 -c 'import sys; print(".".join(map(str, sys.version_info[:3])))')"
    if [[ "${actual_python_runtime}" != "${python_runtime}" ]]; then
      echo "selected Python runtime ${actual_python_runtime} does not match matrix ${python_runtime}" >&2
      exit 1
    fi
    venv="${scratch}/venv-${provider_version}"
    python3 -m venv "${venv}"
    requirements_file="${scratch}/requirements.txt"
    hash_locked_local_wheel_requirement \
      "${provider}" "${artifact}" "${artifact_sha256}" > "${requirements_file}"
    for requirement in "${python_requirement_array[@]}"; do
      printf '%s\n' "${requirement/|/ }" >> "${requirements_file}"
    done
    "${venv}/bin/python" -m pip install --disable-pip-version-check --no-input --require-hashes \
      --requirement "${requirements_file}"
    launcher="${venv}/bin/podman-compose"
    [[ -x "${launcher}" ]]
    command_path="${venv}/bin:/usr/bin:/bin"
    ;;
  *)
    echo "unsupported matrix provider: ${provider}" >&2
    exit 1
    ;;
esac

launcher_sha256="$(sha256sum "${launcher}" | awk '{print $1}')"
for probe in ${probes}; do
  result="${output_directory}/${probe}"
  COMPOSE_LENS_CONFORMANCE_TARGET="${target}" \
    COMPOSE_LENS_CONFORMANCE_PROBE="${probe}" \
    COMPOSE_LENS_CONFORMANCE_LAUNCHER="${launcher}" \
    COMPOSE_LENS_CONFORMANCE_LAUNCHER_SHA256="${launcher_sha256}" \
    COMPOSE_LENS_CONFORMANCE_ACQUISITION_ROOT="${scratch}" \
    COMPOSE_LENS_CONFORMANCE_PLATFORM="${conformance_platform}" \
    COMPOSE_LENS_CONFORMANCE_PATH="${command_path}" \
    COMPOSE_LENS_CONFORMANCE_RESULT_DIRECTORY="${result}" \
    cargo test --locked --test conformance -- --ignored --exact run_selected_provider_config_probe
done
