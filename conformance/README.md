# Compose implementation conformance

This directory owns versioned evidence from released Compose providers and container runtimes. The
public `compose-lens` library remains pure and never invokes them.

Accepting syntax, producing provider configuration, and observing a runtime effect are different
claims. A release is a candidate for measurement, not proof of support.

## Contents

- `provider-config-matrix.toml` defines exact provider artifacts and authored configuration probes.
- `runtime-effect-matrix.toml` defines exact provider/runtime/privilege contexts and fail-closed
  isolation requirements.
- `records/` contains reviewed observations from explicitly invoked capture tests.
- Inputs stay under `fixtures/conformance/` and follow the common fixture contract.

Each matrix entry is either:

- `planned` — a reproducible question that makes no support claim; or
- `observed` — a reviewed record linked from the exact matrix entry.

The provider matrix retains 48 reviewed records. Additional provider rows and every current runtime
effect row remain planned. The matrix files, not prose release lists, are the source of truth for
exact versions and status.

## Evidence lifecycle

```text
exact matrix entry -> isolated invocation -> unreviewed capture
                                              |
                                              v
                                  reviewed retained record
                                              |
                                              v
                               scoped compatibility evidence
```

A successful `config` command proves only the recorded provider output. It does not prove that a
container runtime applied the requested behavior. Runtime effects require their own rootless or
rootful context, host features, image identity, cleanup, and resource audit.

Ordinary tests validate matrix structure, exact fixture hashes, reviewed-record links, and fail-closed
runtime policy. They do not execute a provider or runtime.

## Capture one provider observation

The ignored provider runner requires an exact matrix target, probe, launcher, checksum, platform,
restricted path, and new absolute result directory:

```console
COMPOSE_LENS_CONFORMANCE_TARGET=docker-compose-5-3-1 \
COMPOSE_LENS_CONFORMANCE_PROBE=implementation-sensitive-config \
COMPOSE_LENS_CONFORMANCE_LAUNCHER=/absolute/path/to/docker-compose \
COMPOSE_LENS_CONFORMANCE_LAUNCHER_SHA256=<64-lowercase-hex-digits> \
COMPOSE_LENS_CONFORMANCE_PLATFORM=<explicit-platform-description> \
COMPOSE_LENS_CONFORMANCE_PATH=/usr/bin:/bin \
COMPOSE_LENS_CONFORMANCE_RESULT_DIRECTORY=/absolute/new/result-directory \
cargo test --locked --test conformance -- --ignored --exact run_selected_provider_config_probe
```

The runner verifies fixture bytes, the caller-supplied launcher hash, and exact version output. It
uses a cleared environment and isolated home, config, cache, and runtime directories. A non-zero
probe exit is retained as an observation rather than converted into a harness failure.

The matrix records immutable artifact URLs and published checksums. The actual launcher checksum may
differ when an artifact is installed or unpacked; both identities remain explicit.

## Review a capture

Do not commit generated output directly. Review:

1. target, probe, arguments, exact version, artifact source, and both checksums;
2. fixture identity and bytes;
3. platform, execution dependencies, working directory, and controlled environment;
4. raw stdout, stderr, and exit state;
5. deterministic normalization of local acquisition roots only; and
6. absence of credentials, tokens, private names, personal paths, and other sensitive data.

Copy an accepted result into a stable `records/` directory, change only the matching matrix run to
`observed`, and link its record path. A compatibility rule may cite it only after the feature outcome
and version scope are reviewed separately.

Record-specific retention and redaction rules are in [`records/README.md`](records/README.md).

## Runtime evidence

Runtime rows remain planned unless every declared precondition can be enforced. A valid run may
require rootless or rootful isolation, SELinux state, a new workspace, a caller-supplied preloaded
digest-pinned image, disabled registry/network access, unconditional teardown, and a post-cleanup
resource audit.

If the host cannot meet a requirement, do not run the row and do not weaken it. An unsupported host
is not evidence for or against a Compose feature.

Versioned interpretation behind the initial provider records is retained in
[`docs/research/provider-config-conformance-2026-07-31.md`](../docs/research/provider-config-conformance-2026-07-31.md).
[ADR 0012](../docs/decisions/0012-repository-conformance-harness.md) defines the repository-side
evidence boundary.
