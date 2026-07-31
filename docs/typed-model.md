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

Compose processing tags `!reset` and `!override` remain intact in the syntax document. The typed
parser reads their inner value when it matches a typed field, allowing valid override documents to
participate in loading without incorrectly reporting the wrapper as a field-type error. The merge
stage, not the typed parser, applies the tag's semantics.

## Typed boundary

| Location | Phase 2 fields |
| --- | --- |
| Document | `name`, `services`, `networks`, `volumes`, `configs`, `secrets` |
| Service | `image`, `command`, `environment`, `ports`, `volumes`, `networks`, `profiles`, `configs`, `secrets` |
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
- service config and secret grants: name short syntax or mapping long syntax; and
- labels: list or mapping.

The raw short volume and port strings remain authoritative. Conservative helper parsing must not
turn platform-dependent path or address grammars into false certainty. ADR 0003 defines the full
[syntax-form policy](decisions/0003-preserve-compose-syntax-forms.md).

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
source indentation to recover same-level sibling entries when the private YAML dependency nests
them under that empty value. Regression fixtures require the empty entry and all following siblings
to remain independently visible.

## Diagnostics and evidence

Invalid collection shapes, invalid short/long alternatives, incomplete long forms, duplicate
fields, and invalid typed scalars produce stable diagnostic codes and source labels. Extraction
continues wherever partial data remains useful.

The authored `typed-model` fixtures cover valid Phase 2 forms, invalid recoverable forms, combined
image tags and digests, deferred expressions, empty values, extension fields, unknown fields, and
the short/long service-volume SELinux asymmetry.

Primary format references:

- [Compose Specification](https://github.com/compose-spec/compose-spec/blob/main/spec.md)
- [Docker Compose services reference](https://docs.docker.com/reference/compose-file/services/)
- [Docker Compose networks reference](https://docs.docker.com/reference/compose-file/networks/)
- [Docker Compose volumes reference](https://docs.docker.com/reference/compose-file/volumes/)
- [Docker Compose configs reference](https://docs.docker.com/reference/compose-file/configs/)
- [Docker Compose secrets reference](https://docs.docker.com/reference/compose-file/secrets/)
