# Generated Compose documents

ComposeLens has two deterministic output paths with different inputs:

- canonical rendering consumes a provenance-bearing `MergedProject`; and
- generated rendering consumes caller-constructed Compose-native values that have no authored
  YAML source.

`ComposeDocumentBuilder` is the generated-document boundary. It performs no file, environment,
provider, or runtime access. `build` emits deterministic two-space/LF YAML and immediately parses
the bytes through ComposeLens's loss-aware syntax and typed-document model. A successful
`GeneratedComposeDocument` therefore contains both deployable text and a parse-back-validated
`ComposeDocument`.

## Initial generated subset

`GeneratedService` supports the first runtime-migration fields:

- image references, including `name:tag@digest` spellings;
- exec, shell, and explicitly empty commands;
- ordered literal and host-resolved environment entries;
- combined `user[:group]`, user namespace, supplementary groups, working directory, and explicit
  read-only-root state;
- ordered short-form host mappings;
- protocol-aware target/published/host-address ports;
- named, anonymous, and bind mounts;
- ordered network attachments and aliases; and
- application-owned or external top-level network and volume definitions, with optional exact
  platform-level names.

Generated collections retain insertion order and reject duplicate names where Compose mapping
syntax would otherwise overwrite intent. Singleton service fields reject a second assignment.
Strings are always double-quoted by the private renderer, so YAML scalar inference cannot change
their type.

`GeneratedResource::set_custom_name` emits Compose's top-level `name:` field. Runtime migration
uses it when an application-owned observed network or volume must keep its exact platform name;
without that field Compose would derive a new project-scoped name. Lifecycle ownership and exact
runtime naming remain separate choices.

## Short and long syntax

The builder does not treat Compose short and long forms as universally interchangeable. TCP/UDP
ports, ordinary binds, named volumes, and anonymous volumes use explicit long syntax. Long-form
`published` values are quoted strings as required by the Compose specification. SCTP ports use
short syntax because that form permits platform-specific protocols while the long form defines
only TCP and UDP. An SCTP host address therefore requires an explicit published port so its short
form remains unambiguous. A bind carrying
`GeneratedSelinux` uses short syntax because current Compose behavior does not make the long
`bind.selinux` spelling equivalent for this migration requirement. The builder rejects `:` in a
source or target that would make this selected short form ambiguous; it does not discard the
relabel request or silently switch forms.

Environment and `extra_hosts` use quoted sequence short forms. That preserves insertion order and
allows repeated effective environment names without constructing a duplicate-key YAML mapping.

## Sensitivity and explicit output access

`GeneratedString::sensitive` marks values whose contents must not appear in `Debug`. Sensitivity
propagates through the service, builder, and final document. `GeneratedComposeDocument::text` is
the explicit access boundary for deployable bytes. The parse-back model is also hidden from the
final document's debug representation when any generated value is sensitive.

## Validation boundary

Parse-back validation proves that ComposeLens can read the exact bytes it generated and recover
the supported native forms. It does not establish that every Compose provider or container
runtime version implements those forms equivalently. Callers still run the separate compatibility
profile appropriate for Docker Compose, `podman-compose`, and the selected backend.

The builder does not perform interpolation, merge files, select profiles, apply defaults, resolve
paths, or validate source references. Those remain explicit processing stages for authored input.
See [ADR 0017](decisions/0017-parse-back-validated-compose-generation.md).
