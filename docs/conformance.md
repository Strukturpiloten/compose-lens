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

All 48 previously collected combined and feature-specific provider observations are reviewed and
retained. Six provider-config runs each for authored service `init`, service hostname, service
`cap_add`, service `cap_drop`, service `devices`, service `dns`, service `dns_opt`, service
`dns_search`, service `expose`, service `security_opt`, service annotations, the service
stop-lifecycle probe, service pull policy, service PID limit, service shared-memory size, service memory limit,
service-level `tmpfs`, service `sysctls`, and service `ulimits` are
`planned` and make no support claim.
The pull-policy probe includes schema-only `refresh` and `pull_refresh_after` solely as an explicit
evidence question. It also asks about composite integer intervals and schema-valid `every_0s`,
whose prose semantics remain ambiguous; no provider behavior or cross-format equivalence is
inferred. See the
[result table](research/provider-config-conformance-2026-07-31.md).
The PID-limit probe asks only how provider config handles omission, numeric/string `-1`, positive
integral values including arbitrary precision, ambiguous zero, fractions, and arbitrary strings.
It does not inspect cgroups, infer a default, or compare service `pids_limit` with
`deploy.resources.limits.pids`.
The service-devices probe asks only how provider config handles omission, explicit empty state,
ordered duplicate path forms, CDI-like selectors, and mixed short/long mappings. It does not
inspect host devices, validate colon triples, permissions, CDI or GPU semantics, start a
container, infer privileges, or claim runtime access or cross-format equivalence.
The service-tmpfs probe asks only how provider config handles omission, scalar/list form, explicit
empty lists, exact duplicates, colon-delimited documented assignments, and raw provider options.
It does not inspect runtime mounts, infer default flags, or claim rootless, pod, volume-type, or
cross-format equivalence.
The service-sysctls probe asks only how provider config handles omission, mapping/list form,
explicit empty collections, scalar-valued mappings, ordered list strings, and exact duplicate list
items. It does not apply the configuration, validate namespaces or privileges, inspect a host
kernel, infer provider coercion, or claim runtime or cross-format equivalence.
The service-ulimits probe asks only how provider config handles omission, explicit empty mappings,
integer/string single values, `-1`, and soft/hard ranges. It does not start a container, inspect or
enforce host resource limits, infer a default, normalize a provider spelling, compare Podman
behavior, or claim runtime or cross-format equivalence.
The service-annotations probe covers omission, empty and populated mapping/list forms, duplicate
names, and key-only ambiguity. The service-security-options probe covers representative exact,
duplicate, conflicting, and near-miss AppArmor, no-new-privileges, seccomp, SELinux-label, Mask, and
Unmask shapes. These are provider-config questions only; the probes do not validate profiles or
paths, inspect host/runtime state, or claim cross-format equivalence.

The shared-memory-size probe asks only how provider config handles omission, documented lowercase
units, explicit bytes, leading-zero amounts, numeric and fractional scalars, bare strings, zero,
uppercase units, and IEC spelling. It does not infer Podman's 64 MiB default, inspect runtime
`/dev/shm` allocation, normalize units, or claim IPC, pod-grouping, or cross-format equivalence.
The service-memory-limit probe asks only how provider config handles omission, documented
lowercase units, explicit bytes, leading-zero, numeric, fractional, bare-string, zero, uppercase,
and IEC spellings. Its six planned rows make no default, normalization, enforcement, host/cgroup,
runtime, deploy-consistency, or cross-provider equivalence claim.
The hostname probe asks only whether provider config retains omission, uppercase and digit-leading
labels, and multi-label spelling. It does not claim container-runtime hostname acceptance, DNS or
hosts-file behavior, UTS isolation, or equivalence with service keys and `container_name`.
The capability-drop probe asks only whether provider config retains omission, explicit empty
state, order, and exact case. It does not claim that a runtime recognizes or applies any named
capability, nor infer privilege, namespace, seccomp, SELinux, or cross-format behavior.
The capability-add probe asks the same provider-config-only questions plus independent coexistence
with `cap_drop`. It does not claim runtime privilege effects, reconcile the two fields, or infer
namespace, seccomp, SELinux, runtime capability-set, or cross-format behavior.
Runtime-effect probes remain
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
