# Compose compatibility-profile evidence

- Evaluated: 2026-07-31
- Scope: initial provider/runtime identity and compatibility rules
- Provider conformance: 48 reviewed config observations
- Runtime-effect execution: planned in an isolated 36-run matrix; no local SELinux-capable runtime evidence

## Current release context

The owners' release channels identified these current stable candidates on the evaluation date:

- [Docker Compose 5.3.1](https://github.com/docker/compose/releases/tag/v5.3.1)
- [`containers/podman-compose` 1.5.0](https://github.com/containers/podman-compose/releases/tag/v1.5.0)
- [Docker Engine 29.6.2](https://docs.docker.com/engine/release-notes/29/#2962)
- [Podman 6.0.2](https://github.com/containers/podman/releases/tag/v6.0.2)

These values document research context only. ComposeLens has no “latest” profile and makes no
blanket support claim for these releases. A caller supplies the exact versions it wants assessed.

## Provider and runtime identity

[Podman's compose documentation](https://docs.podman.io/en/latest/markdown/podman-compose.1.html)
states that `podman compose` is a thin wrapper around an external provider. Its defaults include
Docker Compose and `podman-compose`, with Docker Compose taking precedence when both are installed.
Options and commands are passed to that provider.

Consequently, compatibility context has two independent axes:

| Axis | Initial values |
| --- | --- |
| Compose provider | specification, Docker Compose version, `podman-compose` version, tolerant |
| Backend runtime | Docker Engine version, Podman version, or unspecified |

The wrapper version is operational context but does not replace the provider identity.

## Initial rules

### Merge tags

The [Docker merge reference](https://docs.docker.com/reference/compose-file/merge/) explicitly
requires Docker Compose 2.24.4 or later for `!override`. The built-in Docker profile therefore
classifies earlier exact versions as unsupported and later versions as supported.

The same reference documents `!reset` without an introduction version. Reviewed provider
conformance shows it working in the four selected Docker Compose releases. Exact observed Docker
versions are supported; unobserved versions remain implementation-specific. `podman-compose`
1.3.0 and 1.5.0 reject `!reset`. Version 1.3.0 also rejects `!override`, while 1.5.0 applies it.
The full evidence table is in [provider-config conformance](provider-config-conformance-2026-07-31.md).

### Short and long bind SELinux behavior

The current Compose Specification defines both short `z`/`Z` options and long `bind.selinux`.
However, [docker/compose issue 13396](https://github.com/docker/compose/issues/13396) reports that:

- Docker Compose 2.40.3 was used through `podman compose`;
- the backend was Podman 5.6.2;
- short syntax relabeled the host directory; and
- long syntax was accepted but did not relabel it.

The linked [attempted fix](https://github.com/docker/compose/pull/13397) was closed after a Docker
Compose maintainer explained that the Mount API has no `SELinux` option. The built-in rule is scoped
to the exact reported provider/runtime pair. It must not be generalized to later releases without
new evidence.

### Combined image tag and digest

ComposeLens retains `name:tag@digest` because real tools accept this useful form. The current
specification grammar presents tag and digest as alternatives, so the specification and Docker
profiles classify the combination as implementation-specific rather than rejecting it during
parsing. The four observed Docker Compose versions and both observed `podman-compose` versions
retain the combined value, so their exact profiles classify it as supported. Other provider
versions remain implementation-specific or unknown.

### Extensions and tolerant behavior

`x-` fields use the Compose extension namespace and are classified as extensions without warnings.
The tolerant profile records other uncovered constructs as unknown notes. “Tolerant” means preserve
and report; it does not mean supported everywhere.

## Implemented evidence contract

- `ImplementationVersion` parses exact `major.minor.patch` or `vmajor.minor.patch` releases.
- `VersionRange` has inclusive optional minimum and maximum values.
- `CompatibilityEvidence` separates provider-version and runtime-version scopes.
- Evidence kinds distinguish specification, official documentation, issue reproduction, and future
  ComposeLens provider and runtime conformance.
- Findings never store raw compatibility-sensitive values and diagnostics use value-free text.

The authored `fixtures/processing/compatibility-profiles` project exercises all initial features.
`tests/compatibility.rs` verifies exact boundaries, source preservation, selected-service filtering,
evidence scopes, provider/runtime separation, conservative unknowns, and redaction.

## Runtime conformance boundary

Provider-only evidence is complete for the selected Phase 5 matrix. The separate runtime-effect
matrix defines 18 exact provider/runtime/privilege contexts and two SELinux probes. Its 36 entries
remain planned because this development host has no SELinux filesystem and cannot establish
relabel effects. No runtime rule is promoted from provider output.
