# Compose syntax-fidelity evidence

Reviewed: 2026-07-31.

## Question

May ComposeLens normalize short and long field syntax into a single typed value during
parsing?

## Service-volume evidence

The current Compose Specification describes both:

- short mount syntax with `z` and `Z` access-mode options; and
- long mount syntax with `bind.selinux`, whose value is `z` or `Z`.

The same specification documents another difference: short bind syntax creates a missing
host directory for backward compatibility, while long syntax exposes `create_host_path`.

Sources:

- [Compose Specification service volumes](https://github.com/compose-spec/compose-spec/blob/main/spec.md#volumes)
- [Docker Compose service volumes](https://docs.docker.com/reference/compose-file/services/#volumes)

Docker's bind-mount documentation distinguishes the legacy volume-string path from the
Mount API. It lists `z` and `Z` for `--volume`, but states that SELinux relabeling cannot be
requested through `--mount`.

Source: [Docker bind mounts](https://docs.docker.com/engine/storage/bind-mounts/#configure-the-selinux-label).

TheRealBecks reproduced the resulting difference with Docker Compose 2.40.3 and Podman
5.6.2 in November 2025. The long form was accepted and retained an SELinux option in the
generated model, but did not relabel the host path. Docker Compose maintainers identified
the missing Mount API capability as the root cause; the proposed Compose change was closed
without merging.

Evidence:

- [docker/compose issue 13396](https://github.com/docker/compose/issues/13396)
- [docker/compose pull request 13397](https://github.com/docker/compose/pull/13397)
- [podman-container-tools/podman issue 27600](https://github.com/podman-container-tools/podman/issues/27600)

## Interpretation

This evidence does not establish that every later implementation and runtime has the same
behavior. It establishes that:

1. specification shape alone cannot prove runtime equivalence;
2. accepted configuration can still have implementation-dependent effects;
3. compatibility claims need implementation and version boundaries; and
4. preserving the authored syntax form is necessary evidence for later decisions.

ComposeLens therefore parses the two forms into distinct variants. Runtime reliability is now a
validation-profile decision and is not hard-coded into the loss-aware parser. The initial profile
rule is deliberately limited to the reported Docker Compose 2.40.3 and Podman 5.6.2 pair; other
version combinations remain unknown until evidence covers them.

## Initial regression requirement

The typed-model corpus contains one document with short `:z`, short `:Z,ro`, and long
`bind.selinux: Z` mounts. Tests require all three to remain distinguishable and retain their
source spans, options, and extension fields.
