# Compose implementation conformance

This directory owns repository-side probes of released Compose implementations. It does not add
runtime access to the `compose-lens` library.

## Contents

- `provider-config-matrix.toml` selects exact provider releases, checksum-pinned artifacts, and authored probes.
- `runtime-effect-matrix.toml` defines exact planned provider/runtime/privilege contexts and fail-closed isolation.
- `records/` contains reviewed observations produced by the ignored conformance test.
- the referenced inputs remain normal versioned fixtures under `fixtures/conformance/`.

Matrix entries have one of two states:

- `planned` means the exact combination should be measured and makes no support claim;
- `observed` means a reviewed record is retained in this repository and linked from the entry.

There is deliberately no `latest` state or version alias. Moving release candidates are reviewed
in [the conformance guide](../docs/conformance.md), then entered here as exact versions.

## Running one provider-config probe

The runner is an ignored Rust integration test so ordinary builds never invoke Docker Compose,
`podman-compose`, Docker Engine, or Podman. It accepts only an absolute launcher path and clears the
inherited environment before invoking it.

```shell
COMPOSE_LENS_CONFORMANCE_TARGET=docker-compose-5-3-1 \
COMPOSE_LENS_CONFORMANCE_PROBE=implementation-sensitive-config \
COMPOSE_LENS_CONFORMANCE_LAUNCHER=/absolute/path/to/docker-compose \
COMPOSE_LENS_CONFORMANCE_LAUNCHER_SHA256=<64-lowercase-hex-digits> \
COMPOSE_LENS_CONFORMANCE_PLATFORM=<explicit-platform-description> \
COMPOSE_LENS_CONFORMANCE_PATH=/usr/bin:/bin \
COMPOSE_LENS_CONFORMANCE_RESULT_DIRECTORY=/absolute/new/result-directory \
cargo test --locked --test conformance -- --ignored --exact run_selected_provider_config_probe
```

The matrix owns the immutable artifact URL and published SHA-256. The caller supplies the actual
launcher's SHA-256, which the runner verifies before execution; this may differ from the artifact
hash for an installed Python wheel. Fixture bytes are checked against the matrix hash, and runner
version output is checked exactly against the matrix version before the probe runs. The new result directory
receives metadata plus unmodified stdout and stderr files. A non-zero probe exit is an observation
and is recorded instead of being converted into a test failure.

Do not commit a generated result directly. Review its command, provider identity, artifact source
and checksum, launcher checksum, execution dependencies, platform assumptions, output, fixture
hash, and absence of sensitive data. Deterministic replacement of local acquisition and repository
roots must be declared in the record. Then
copy it into a stable record directory, change the matching matrix run to `observed`, and add the
record path. Compatibility rules may cite it only after that review.

The Phase 5 provider matrix contains 48 reviewed records. Each dated target/probe directory owns
one `record.toml` plus raw and normalized output evidence. The runtime-effect matrix remains
planned because it requires an enforcing SELinux host and exact external runtime installations;
its policy test performs no runtime operation.
