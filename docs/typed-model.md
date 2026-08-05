# Phase 2 typed Compose model

Phase 2 is complete for the first BoxFerry Compose-to-Quadlet conversion boundary. It is a
source-aware native Compose model, not a claim that every field in the Compose Specification
is already typed.

Fields outside this boundary remain available through the loss-aware syntax document and as
source-spanned unknown-field references. Adding another field must not require redesigning the
Phase 2 representation rules.

## Parse contract

`ComposeDocument::parse` extracts typed data from a `SyntaxDocument` and returns recoverable
diagnostics plus partial data. It does not:

- interpolate environment expressions;
- read the process environment;
- select profiles;
- apply Compose defaults;
- merge multiple files;
- normalize short syntax into long syntax; or
- validate runtime support in Docker Compose or Podman Compose.

Those operations belong to Phase 3 processing and compatibility profiles.

Multi-file consumers use `build_project_view` after merge and optional profile selection. That
operation types effective values directly and wraps them in `ProjectValue<T>` with complete
multi-file provenance. It never renders canonical YAML and reparses it as a single document. See
[ADR 0016](decisions/0016-native-merged-project-view.md).

The merged-project view exposes `extra_hosts` in both sequence and mapping form. Each effective
entry retains its hostname key sources, address provenance, authored collection form, raw IPv4 or
IPv6 spelling, and a distinct `host-gateway` classification. This is additive to the
single-document `ExtraHosts` model and lets adapters preserve explicit runtime mappings.

The merged-project view also exposes `healthcheck` without collapsing scalar/list command forms.
Every effective timing, retry, disable, and command field has its own merge provenance. Compose
`start_interval` remains distinct from provider-specific startup-healthcheck features.

Execution identity and context are native at both document and merged-project layers. Effective
`user`, `userns_mode`, `group_add`, `working_dir`, and `read_only` values retain raw spelling,
field provenance, and, for supplementary groups, per-item provenance. No account, group, path, or
namespace lookup is performed.

An explicit `container_name` is a source-aware scalar at both layers. The effective project view
retains ordinary Compose scalar replacement provenance across files. The parser does not confuse
the custom runtime name with the service key or infer one when the field is absent.

Service config and secret grants are native at both layers. The merged-project view retains short
versus long syntax, collection/item provenance, and separate provenance for long-form `source`,
`target`, `uid`, `gid`, and `mode`. This preserves Compose's unique-by-target merge behavior,
including nested values inherited from an earlier file.

Compose processing tags `!reset` and `!override` remain intact in the syntax document. The typed
parser reads their inner value when it matches a typed field, allowing valid override documents to
participate in loading without incorrectly reporting the wrapper as a field-type error. The merge
stage, not the typed parser, applies the tag's semantics.

## Typed boundary

| Location | Phase 2 fields |
| --- | --- |
| Document | `name`, `services`, `networks`, `volumes`, `configs`, `secrets` |
| Service | `container_name`, `image`, `build`, `command`, `environment`, `labels`, `extra_hosts`, `user`, `userns_mode`, `group_add`, `working_dir`, `read_only`, `ulimits`, `depends_on`, `healthcheck`, `deploy`, `ports`, `volumes`, `networks`, `profiles`, `configs`, `secrets` |
| Network definition | `driver`, `driver_opts`, `attachable`, `enable_ipv4`, `enable_ipv6`, `external`, `internal`, `ipam`, `labels`, `name` |
| Volume definition | `driver`, `driver_opts`, `external`, `labels`, `name` |
| Config definition | `file`, `environment`, `content`, `external`, `name` |
| Secret definition | `file`, `environment`, `external`, `name` |

Every implemented mapping retains `x-` extensions and unrecognized fields with source spans.
Collections retain authored order.

## Representation fidelity

Field-specific variants retain forms whose behavior or meaning can differ:

- command: explicit null, scalar, or list, including empty scalar and empty list;
- environment: list or mapping, including `NAME`, `NAME=`, empty strings, and null values;
- ports: scalar short syntax or mapping long syntax;
- service volumes: scalar short syntax or mapping long syntax;
- service networks: name sequence or options mapping;
- service config and secret grants: name short syntax or mapping long syntax;
- extra hosts: hostname/address sequence or mapping, retaining delimiters and IPv6 brackets;
- dependencies: service-name sequence or condition/options mapping;
- health-check tests: shell-command scalar or tokenized list;
- ulimits: one scalar or separate soft/hard values;
- build: scalar context or a mapping of independently identified specification fields;
- service and resource labels: list or mapping. Service-label list entries retain the complete
  scalar so values containing additional `=` characters are not truncated.

The raw short volume and port strings remain authoritative. Conservative helper parsing must not
turn platform-dependent path or address grammars into false certainty. ADR 0003 defines the full
[syntax-form policy](decisions/0003-preserve-compose-syntax-forms.md).

Container-side mount targets additionally expose a lexical `ContainerPath` classification for Unix
absolute, Windows drive-letter, Windows UNC, relative, and deferred paths. This classification is
independent of the machine running ComposeLens. Host bind sources continue through the separate
host-path resolver with explicit origins and home context.

## Issue-derived model expansion

Raw `user` values are authoritative. User/group helpers recognize names, numeric IDs, empty
components, and deferred interpolation without resolving accounts. The split ignores colons inside
`${VAR:-default}`, avoiding a common false decomposition. `ulimits` recognizes authored `-1` as an
explicit unlimited value in both scalar and soft/hard forms.

`service_healthy`, `service_started`, and `service_completed_successfully` dependency conditions
are native types. `ComposeDocument::validate_dependencies` checks one document; post-merge
`validate_references` checks the selected project. A missing Compose health check produces a
warning because the image may contain health metadata. An explicitly disabled health check is an
error for a required `service_healthy` edge. `required: false` downgrades unavailable dependency
diagnostics to warnings without hiding the reference.

Build and deploy definitions expose every current top-level subfield as its own stable kind and
source reference. This is intentionally field-level evaluation, not a claim that every nested
value is already semantically typed or supported by every converter.

Podman `keep-id`, `auto`, and `nomap` user-namespace values and the `host-gateway` token have native
classifications. Their compatibility findings cite official implementation documentation and do
not promote untested provider pass-through to supported. See
[ADR 0014](decisions/0014-issue-derived-native-model-expansion.md).

## Deferred values

Interpolation is not parsing. Boolean-capable fields therefore distinguish a YAML boolean literal
from a deferred scalar expression such as `${EXTERNAL:-false}`. Environment and option mappings
retain null, boolean, numeric, and string scalars without applying interpolation or coercion.

Image references are intentionally tolerant. A combined tag and digest such as
`registry.example/app:1.2@sha256:abcdef` is accepted and retains its complete raw value. ComposeLens
does not reject a real implementation-supported reference merely because an unrelated stricter
grammar would reject it.

## Empty YAML values

An omitted mapping value, for example `INHERITED:` in an environment mapping or `cache:` in a
top-level volume collection, is an explicit null-like authored value. The typed extractor uses
source columns to recover same-level sibling entries when the private YAML dependency nests them
under that empty value. Recovery crosses the private tree boundary when an empty value is the last
entry of a child mapping, so later parent fields are not silently reparented. Regression fixtures
require the empty entry and all following siblings to remain independently visible.

## Diagnostics and evidence

Invalid collection shapes, invalid short/long alternatives, incomplete long forms, duplicate
fields, and invalid typed scalars produce stable diagnostic codes and source labels. Extraction
continues wherever partial data remains useful.

The authored `typed-model` fixtures cover valid Phase 2 forms, issue-derived post-0.1 forms,
invalid recoverable forms, combined
image tags and digests, deferred expressions, empty values, extension fields, unknown fields, and
the short/long service-volume SELinux asymmetry.

Primary format references:

- [Compose Specification](https://github.com/compose-spec/compose-spec/blob/main/spec.md)
- [Docker Compose services reference](https://docs.docker.com/reference/compose-file/services/)
- [Docker Compose networks reference](https://docs.docker.com/reference/compose-file/networks/)
- [Docker Compose volumes reference](https://docs.docker.com/reference/compose-file/volumes/)
- [Docker Compose configs reference](https://docs.docker.com/reference/compose-file/configs/)
- [Docker Compose secrets reference](https://docs.docker.com/reference/compose-file/secrets/)
- [Docker Compose build reference](https://docs.docker.com/reference/compose-file/build/)
- [Docker Compose deploy reference](https://docs.docker.com/reference/compose-file/deploy/)
- [Podman 5.4 run reference](https://docs.podman.io/en/v5.4.0/markdown/podman-run.1.html)
