# Compose canonical-rendering evidence

- Evaluated: 2026-07-31
- Scope: deterministic rendering of a ComposeLens merged project
- Implemented format: `compose-lens-canonical-v1`

## Upstream terminology

The [`docker compose config` reference](https://docs.docker.com/reference/cli/docker/compose/config/)
describes a command that parses, resolves, merges, and renders the Compose model in canonical
format. Its normal processing includes environment resolution and expansion of short notation;
options such as `--no-interpolate`, `--no-normalize`, and `--no-path-resolution` selectively
disable parts of that behavior.

The [Compose Specification](https://github.com/compose-spec/compose-spec/blob/main/spec.md) defines
the YAML application model and multi-file merge behavior, but it does not prescribe one exact
byte-level serializer for third-party libraries.

ComposeLens therefore uses “canonical” for its own stable presentation contract, not as a claim
that its bytes equal a particular Docker Compose release. Processing stages remain explicit, and
short and long Compose syntax are not normalized into one another.

## Canonical-v1 format

| Property | Fixed behavior |
| --- | --- |
| Encoding | UTF-8 |
| Line endings | LF, including one final newline |
| Document marker | omitted |
| Indentation | two spaces |
| Mapping and sequence order | retained from the merged semantic model |
| Mapping keys | JSON-compatible double-quoted strings |
| String scalars | JSON-compatible double-quoted strings |
| Booleans | lowercase `true` and `false` |
| Null-like values | `null` |
| Numbers | retained semantic spelling |
| Empty collections | inline `{}` and `[]` |
| Compose short/long forms | retained; never implicitly interchanged |

`CanonicalFormatting::default()` is exactly this table. ADR 0011 permits explicit indentation,
LF/CRLF, document-marker, and final-line-ending changes without changing ordering, quoting safety,
or Compose forms. Customized output is deterministic but is not named canonical-v1.

The [YAML 1.2.2 double-quoted scalar rules](https://yaml.org/spec/1.2.2/#double-quoted-style)
provide an unambiguous scalar style with escape sequences. ComposeLens escapes quotes,
backslashes, common control characters, and Unicode line/format controls so canonical strings do
not depend on plain-scalar inference.

## Explicit boundary

`render_canonical` accepts an already merged project. It does not:

- read files or the process environment;
- interpolate variables;
- resolve paths or apply defaults;
- normalize short and long Compose forms;
- validate compatibility; or
- invoke Docker Compose, `podman-compose`, Docker Engine, or Podman.

An optional matching `ProfileSelection` filters inactive services only. Top-level resources remain
available because an application may need them for diagnostics or another selected service.

## Recovery and sensitivity

A retained safe YAML tag is preserved. An invalid tag token is diagnosed and omitted while its
value is rendered. An unresolved alias is diagnosed and rendered as `null`; this produces a
standalone parseable document without inventing a target value.

Interpolated sensitive values are present in the explicit output because rendering is the caller's
requested operation. The result's `Debug` implementation redacts the whole output, and diagnostic
messages contain no rendered values.

## Test evidence

The authored `fixtures/roundtrip/canonical-merged` fixture covers multi-file merging, scalar
normalization, retained order, short volume syntax with `:Z`, long port syntax, unknown tags,
profile-restricted services, empty collections, and a trailing empty environment value followed by
parent service fields. `tests/rendering.rs` verifies exact golden bytes, repeatability,
parse-merge-render stability, profile filtering, alias recovery, and sensitive-output redaction.
