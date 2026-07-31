# Compose compatibility-profile evidence

- Evaluated: 2026-07-31
- Scope: initial provider/runtime identity and compatibility rules
- Runtime automation: not yet implemented

## Current release context

The official release pages identified these latest stable releases on the evaluation date:

- [Docker Compose 5.1.4](https://github.com/docker/compose/releases/tag/v5.1.4)
- [`containers/podman-compose` 1.5.0](https://github.com/containers/podman-compose/releases/tag/v1.5.0)
- [Podman 5.8.2](https://github.com/containers/podman/releases/tag/v5.8.2)

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

The same reference documents `!reset` without an introduction version. Docker profiles classify it
as implementation-specific until a release boundary or broader conformance matrix is recorded.
The specification-oriented profile accepts both tags; the `podman-compose` profile remains unknown.

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
parsing. `podman-compose` remains unknown until its exact versions are exercised.

### Extensions and tolerant behavior

`x-` fields use the Compose extension namespace and are classified as extensions without warnings.
The tolerant profile records other uncovered constructs as unknown notes. “Tolerant” means preserve
and report; it does not mean supported everywhere.

## Implemented evidence contract

- `ImplementationVersion` parses exact `major.minor.patch` or `vmajor.minor.patch` releases.
- `VersionRange` has inclusive optional minimum and maximum values.
- `CompatibilityEvidence` separates provider-version and runtime-version scopes.
- Evidence kinds distinguish specification, official documentation, issue reproduction, and future
  ComposeLens runtime conformance.
- Findings never store raw compatibility-sensitive values and diagnostics use value-free text.

The authored `fixtures/processing/compatibility-profiles` project exercises all initial features.
`tests/compatibility.rs` verifies exact boundaries, source preservation, selected-service filtering,
evidence scopes, provider/runtime separation, conservative unknowns, and redaction.

## Remaining conformance work

- Run the fixture with pinned Docker Compose providers against pinned Docker Engine and Podman
  backends.
- Run it directly with pinned `containers/podman-compose` and Podman combinations.
- Record commands, environment, platform/SELinux state, exit status, normalized configuration, and
  runtime effects.
- Promote a classification only for the measured range. Preserve old-version rules while those
  versions remain supported.
