#!/usr/bin/env bash

# Shared, side-effect-free helpers for the podman-compose provider bootstrap.
# The caller owns downloading, checksum verification, virtual-environment
# creation, and installation.

provider_artifact_filename() {
  if [[ $# != 1 ]]; then
    printf 'usage: provider_artifact_filename ARTIFACT_URL\n' >&2
    return 64
  fi

  python3 - "$1" << 'PY'
import re
import sys
from pathlib import PurePosixPath
from urllib.parse import unquote, urlsplit

url = urlsplit(sys.argv[1])
if url.scheme != "https" or url.netloc == "" or url.query or url.fragment:
    raise SystemExit("provider artifact URL must be an HTTPS URL without query or fragment")

filename = PurePosixPath(unquote(url.path)).name
if not re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9._+-]*", filename):
    raise SystemExit("provider artifact URL must end in a portable filename")

print(filename)
PY
}

hash_locked_local_wheel_requirement() {
  if [[ $# != 3 ]]; then
    printf 'usage: hash_locked_local_wheel_requirement DISTRIBUTION WHEEL SHA256\n' >&2
    return 64
  fi

  local requirement_distribution=$1
  local requirement_wheel=$2
  local requirement_digest=$3

  if [[ ! "${requirement_distribution}" =~ ^[A-Za-z0-9][A-Za-z0-9._-]*$ ]]; then
    printf 'Python distribution name is not portable: %s\n' "${requirement_distribution}" >&2
    return 64
  fi
  if [[ ! "${requirement_wheel}" = /* || ! -f "${requirement_wheel}" || "${requirement_wheel##*/}" != *.whl ]]; then
    printf 'local Python artifact must be an existing absolute wheel path: %s\n' "${requirement_wheel}" >&2
    return 64
  fi
  if [[ ! "${requirement_digest}" =~ ^[0-9a-f]{64}$ ]]; then
    printf 'local Python artifact must carry one lowercase SHA-256 hash\n' >&2
    return 64
  fi

  local wheel_uri
  wheel_uri="$(
    python3 - "${requirement_wheel}" << 'PY'
import pathlib
import sys

print(pathlib.Path(sys.argv[1]).resolve(strict=True).as_uri())
PY
  )"
  printf '%s @ %s --hash=sha256:%s\n' \
    "${requirement_distribution}" "${wheel_uri}" "${requirement_digest}"
}
