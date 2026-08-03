# Processing model

## Principle

A Compose file is not the same thing as a fully resolved Compose project. ComposeLens represents the intermediate stages so callers retain control over environment access, merging, profile selection, defaults, and normalization.

## Core representations

### Syntax document

Represents what was written, including source spans and unrecognized constructs. It is the basis for preservation-oriented editing and precise diagnostics.

### Typed document

Represents one Compose document with native Compose types. Values may still contain interpolation expressions, relative paths, implementation extensions, and unresolved references.

### Loaded project

Represents an ordered collection of documents plus their origins. The implemented loader retains
each document's display label and directory, requires unique caller-managed source IDs, and uses the
first document's directory as the multi-file project base. It does not imply that any optional
transformation has run.

### Semantic view

Represents a chosen interpretation of the project for a particular implementation profile and processing context. It records which operations and defaults were applied.

### Native project view

Represents the effective first-conversion fields of a merged and optionally profile-selected
project as native Compose values. `ProjectValue<T>` retains the complete merge operation, every
contributing span, the effective source, and sensitivity state. It is a consumer boundary, not a
replacement for the generic merged tree.

## Explicit operations

### Load

Accepts named source texts and origins supplied by the caller. The core loader performs no file or
environment access. Recoverable syntax and typed-model diagnostics stay attached to their document
and are aggregated without preventing analysis. File discovery and I/O belong in application
adapters; paths retain the origin needed for later resolution.

### Interpolate

Evaluates supported variable expressions using an explicit provider. The provider may expose process environment variables, a supplied map, an `.env` document, or a test fixture. No provider is consulted during parsing.

The result retains a distinction between the original expression and the resolved value. Sensitive values are redacted from diagnostics.

The implemented interpolation kernel supports direct, default, required, alternative, nested, and escaped-dollar expressions. `EmptyEnvironment`, `MapEnvironment`, and the public `EnvironmentProvider` trait keep variable access explicit. A provider value can be marked sensitive; that classification propagates only when its content enters the result. Diagnostics never contain resolved values or the caller-authored message operand of a required expression.

The document overlay applies the kernel to unquoted and double-quoted YAML values while leaving mapping keys and single-quoted values unchanged. A loaded project can create one overlay per file in input order. A future application-level environment loader will construct provider precedence from explicit caller inputs and the selected implementation profile; merge still occurs only after interpolation.

### Merge

Combines loaded documents into a parser-independent semantic tree. The implemented merge retains
source spans and classifies authored, added, replaced, recursively merged, appended, reset, and
overridden values. It handles ordinary mappings and sequences, shell-command replacement,
environment and label keys across map/list forms, unique service resources, YAML merge keys,
`!reset`, and `!override`.

The operation accepts an optional set of matching per-file interpolation overlays. Omitting them is
an explicit request to merge authored expressions. A mismatched project/overlay pair is rejected.
Implementation compatibility classification remains a later stage. The evidence and remaining
runtime test requirements are recorded in [Compose multi-file merge evidence](research/compose-merge.md),
and ADR 0006 defines the [merge representation](decisions/0006-provenance-preserving-compose-merge.md).

### Select profiles

`select_profiles` creates a non-destructive service view from a merged project and an explicit
`ProfileRequest`. Services without an effective `profiles` restriction are active. Restricted
services are active when one of their valid names is requested, or when the caller explicitly
requests all profiles. The operation validates the Compose profile-name grammar and preserves
inactive services in the merged source.

Command-line service targeting can activate a targeted service's profiles in Docker Compose. That
is runtime-command behavior, not an implicit part of this library operation; an application must
model it as an explicit input if it needs that behavior. References do not silently activate a
profile-disabled service.

### Build the native project view

`build_project_view` consumes a `MergedProject` directly and accepts an optional matching
`ProfileSelection`. A matching selection filters inactive services; omitting it includes all
services. A selection created from a different project returns the same stable mismatch diagnostic
used by the other post-merge operations.

The native view covers project name, services, images, commands, merged environment, extra host
mappings, health checks, service dependencies, ports, volume mounts, service networks, profiles,
and top-level network, volume, config, and secret definitions. Environment values are keyed after field-specific Compose merging, while
each entry retains whether its effective spelling came from mapping, `KEY=VALUE`, or key-only list
syntax. Extra-host entries retain sequence or mapping syntax, raw address spelling, lexical address
classification, and the `host-gateway` implementation token. Values and collection items retain
complete multi-file provenance. Dependencies retain effective short or long syntax and nested
condition, restart, required, and unknown-option provenance without applying target lifecycle
defaults. Unmodeled fields retain their semantic path, all key locations, value provenance,
extension classification, and sensitivity.

This operation does not render or parse generated YAML. It also does not read files, consult an
environment provider, apply defaults, validate an implementation, or perform conversion. Native
coverage is documented in [ADR 0016](decisions/0016-native-merged-project-view.md).

### Resolve references and paths

`resolve_paths` classifies the supported host paths and retains their raw text, source span,
purpose, first-file project origin, and optional lexically resolved form. Relative paths use the
merged project's retained base directory. `~` expansion requires a caller-supplied
`PathContext`; ComposeLens does not inspect the process home directory. Resolution does not
canonicalize, follow symlinks, test existence, or otherwise access the file system. Inactive
service mounts are not part of the selected view, while top-level config and secret file sources
remain project resources.

`validate_references` inspects active services and distinguishes found, missing, and
profile-inactive targets. The boundary covers networks, named volumes, configs, secrets,
`depends_on`, service namespace modes, links, and local `extends`. A `service_healthy` dependency
with no Compose health check produces a warning because image metadata is unavailable; an
explicitly disabled health check produces an error. Diagnostics never delete or rewrite the
authored reference. Long dependencies with `required: false` remain visible but downgrade missing,
inactive, or disabled-health failures to warnings.

Every resolver rejects a `ProfileSelection` created from a different merged project, even when the
caller reused the same source identifiers.

### Apply defaults

`resolve_defaults` turns omissions into explicit `DefaultRequest` values and asks a caller-owned
`DefaultProvider` for each decision. `NoDefaults` leaves every omission unresolved.
`ComposeDefaults` supplies the documented network, port, volume, config, secret, and restart
defaults for an explicit Linux or Windows container platform. The result is a decision overlay;
the merged tree is unchanged.

Defaulting remains separate from parsing and implementation compatibility. A future Docker Compose
or Podman Compose profile may deliberately supply different values or decline a specification
default when versioned runtime evidence requires it.

### Validate

`validate_compatibility` discovers compatibility-sensitive constructs in the selected merged view
and applies an explicit `CompatibilityProfile`. The initial detector covers combined image tags and
digests, short and long bind `SELinux` relabeling, `!reset`, `!override`, and `x-` extensions.
Supported occurrences remain in the report without diagnostics; implementation-specific,
deprecated, unsupported, and unknown classifications receive stable diagnostics at the authored
span. Validation never deletes or normalizes the construct.

Profiles identify the exact Compose provider independently from the optional backend runtime.
Docker Compose can target Docker Engine or Podman. The independent `containers/podman-compose`
provider is distinct from Podman's `podman compose` wrapper, which delegates to an external
provider. ComposeLens therefore has no ambiguous “Podman Compose” target that guesses which
provider was executed.

Implementation versions are exact three-component numeric values. Evidence can carry inclusive
minimum and maximum provider and runtime ranges. ComposeLens never substitutes “latest” when the
caller omitted a version. Built-in classifications distinguish specification text, official
documentation, public issue reproductions, and future ComposeLens-controlled runtime conformance.
Unknown remains an honest result when evidence does not cover the selected version pair.

The tolerant profile preserves constructs and emits notes for unknown runtime behavior. It does
not turn missing evidence into a support claim. A selection belonging to another merged project is
rejected before compatibility discovery.

### Render

`render_canonical` emits the fixed `compose-lens-canonical-v1` YAML form from a merged project. A
matching optional profile selection filters inactive services while retaining top-level resources.
The result contains output, structured diagnostics, validity, and sensitivity state. Safe retained
YAML tags survive; invalid tags are diagnosed and dropped; unresolved aliases are diagnosed and
recovered as `null` so the output remains parseable.

Canonical rendering does not implicitly load, interpolate, merge, resolve, default, normalize, or
validate. In particular, it retains effective short and long Compose forms instead of reproducing
the normalizing behavior of `docker compose config`. The exact format and security boundary are
documented in [ADR 0009](decisions/0009-deterministic-canonical-rendering.md). Preservation-oriented
syntax editing remains a separate operation with its own source-level contract.

`apply_preservation_edits` atomically replaces existing YAML value scalars at exact source spans.
The initial replacement types are string, sensitive string, boolean, validated number, and null.
The operation patches the original syntax text rather than serializing the typed document, so
comments, whitespace, ordering, extensions, unknown fields, and untouched scalar spelling remain
byte-identical. Any mismatched source, non-scalar target, unsupported block/multiline style,
invalid number, or overlap rejects the complete batch and returns the original text.

Spans are revision-specific. A caller parses successful output again before creating another edit
from the new document. Structural insertions, removals, collection transformations, and block
scalar editing remain outside the initial boundary. See the [editing guide](preservation-editing.md)
and [ADR 0010](decisions/0010-atomic-span-based-preservation-edits.md).

`render_canonical_with_formatting` uses the same merged project and optional selection as canonical
rendering, plus explicit `CanonicalFormatting`. The only choices are a positive space-indentation
width, LF or CRLF, document-marker emission, and final-line-ending emission. Default formatting is
byte-identical canonical-v1. Formatting does not alter ordering, quoting safety, Compose syntax
forms, diagnostics, or sensitivity. See the [formatting guide](render-formatting.md) and
[ADR 0011](decisions/0011-presentation-only-render-formatting.md).

## Unknown and implementation-specific data

- `x-*` extensions receive first-class preservation.
- Unknown fields are retained with source locations.
- Validation profiles decide whether unknown fields are allowed, warned about, or rejected.
- Podman-specific fields remain native data; BoxFerry decides whether a target can represent them.

## Image references

ComposeLens preserves the written reference and may expose a tolerant parsed view. It must support implementation behavior such as references combining a tag and digest when the selected implementation accepts them. Normalization must never erase the original value.
