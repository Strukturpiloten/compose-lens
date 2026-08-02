# Compose implementation conformance

Reviewed: 2026-07-31.

ComposeLens separates documented syntax, public issue reproductions, and ComposeLens-controlled
runtime observations. A released version is a candidate for measurement, not proof of support.

## Current release review

The current stable releases found in their owners' release channels on the review date are:

| Component | Current stable candidate | Primary source |
| --- | --- | --- |
| Docker Compose | 5.3.1 | [docker/compose v5.3.1](https://github.com/docker/compose/releases/tag/v5.3.1) |
| `containers/podman-compose` | 1.5.0 | [containers/podman-compose v1.5.0](https://github.com/containers/podman-compose/releases/tag/v1.5.0) |
| Docker Engine | 29.6.2 | [Docker Engine 29.6.2 release notes](https://docs.docker.com/engine/release-notes/29/#2962) |
| Podman | 6.0.2 | [Podman v6.0.2](https://github.com/containers/podman/releases/tag/v6.0.2) |

These entries will become stale. The exact provider-config matrix remains reproducible when they
do because it never resolves a moving release alias.

## Matrix dimensions

The completed provider-only `config` slice selects exact Docker Compose
versions 2.24.3, 2.24.4, 2.40.3, and 5.3.1 plus exact `podman-compose` versions 1.3.0 and 1.5.0.
Those versions cover Docker's documented `!override` boundary, the provider from the published
SELinux reproduction, one older packaged `podman-compose` line, and both current provider releases.
The `podman-compose` targets use its own `--dry-run` and `version --short` paths so provider-config
collection does not require a working Podman runtime. Both selected releases expose those paths in
their versioned source: [`podman-compose` 1.3.0](https://github.com/containers/podman-compose/blob/v1.3.0/podman_compose.py)
and [`podman-compose` 1.5.0](https://github.com/containers/podman-compose/blob/v1.5.0/podman_compose.py).

All 48 combined and feature-specific provider observations are reviewed and retained. See the
[result table](research/provider-config-conformance-2026-07-31.md). Runtime-effect probes remain
separate because accepting or rendering a configuration does not show that its requested behavior
occurred. The runtime matrix includes:

- Podman 5.4.0, the minimum BoxFerry target;
- Podman 5.6.2, the exact published SELinux reproduction;
- Podman 5.8.2, the final selected 5.x maintenance point;
- Podman 6.0.2, the current stable candidate at review time; and
- Docker Engine 29.6.2, the current stable candidate at review time.

Rootless and rootful execution, host SELinux state, operating system, architecture, storage and
network backends, and Compose provider remain explicit dimensions. They will not be collapsed into
one “works with Podman” value.

## Evidence lifecycle

```text
exact release candidate
        │
        ▼
planned matrix run ──▶ isolated invocation ──▶ unreviewed captured result
                                                    │
                                                    ▼
                                          reviewed retained record
                                                    │
                                                    ▼
                                      compatibility-rule evidence
```

The matrix contract and ignored provider-config runner are in [`../conformance/`](../conformance/README.md).
Ordinary ComposeLens tests validate matrix completeness and fixture provenance but do not contact
providers or runtimes. A generated record must pass human review before its matrix status changes
to `observed`. A compatibility rule additionally needs an exact feature outcome and evidence scope;
the existence of a successful `config` command alone is insufficient.

## Runtime matrix status

The repository contains exact Podman 5.4.0, 5.6.2, 5.8.2, and 6.0.2 plus Docker Engine 29.6.2
contexts for Docker Compose and `podman-compose`, split into rootless and rootful execution. The
36 short/long SELinux entries remain `planned`. Their fail-closed contract requires an enforcing
SELinux host, a new workspace, a caller-supplied preloaded digest-pinned image, no registry or
network access, unconditional `down` cleanup, and a post-cleanup runtime resource audit.

This host reports no SELinux filesystem, so executing those entries here would provide no valid
relabel evidence. Scheduled runtime execution is a later operational activity and cannot change
provider-only outcomes retroactively.
