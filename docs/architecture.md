# Software architecture

## Purpose

ComposeLens provides a source-aware, tolerant, and typed representation of Compose documents while keeping project processing operations explicit. It supports both analysis and deterministic rewriting without forcing callers into a fully resolved configuration.

## Layers

```text
bytes or text
     │
     ▼
syntax document ──▶ typed document ──▶ loaded project ──▶ merged project ──▶ native project view
     │                    │                  │                  │                    │
     ├──▶ source map      ├──▶ extensions    ├──▶ interpolation├──▶ profiles          ├──▶ adapters
     │                    │                  └──▶ origins       ├──▶ validation        └──▶ diagnostics
     │                    │                                     ├──▶ resolution
     │                    │                                     │
     └────────────────────┴─────────────────────────────────────┴──────────────────────▶ renderer
```

### Source and syntax layer

The syntax layer owns YAML representation, comments where supported, scalar spelling, anchors and aliases, mappings, sequences, duplicate-key diagnostics, and byte/span locations.

It must represent syntactically valid input even when the typed Compose model does not recognize every field.

The initial implementation stores an immutable loss-aware concrete syntax tree plus the original source. [`yaml-edit`](https://docs.rs/yaml-edit/0.2.3/yaml_edit/) is private behind ComposeLens-owned APIs as decided in [ADR 0002](decisions/0002-loss-aware-yaml-syntax.md). Recoverable malformed input produces both a renderable syntax document and structured diagnostics. A constrained same-byte-length private parser adapter accepts valid comma-containing block plain values while all public spelling, decoding, ranges, editing, and rendering continue to use the authored source. The complete-root guard remains the fail-safe for any backend omission. [ADR 0015](decisions/0015-byte-preserving-yaml-backend-compatibility.md) defines that boundary.

### Typed document model

The typed layer exposes Compose concepts such as services, networks, volumes, configs, secrets, build configuration, dependencies, and deployment settings.

Typed nodes retain source references and unknown fields. Parsing into typed data must not destroy the syntax document required for a later loss-aware render.

Short and long syntax remain distinct field-specific variants. They are not normalized merely because they describe similar concepts: defaults, available options, and runtime behavior can differ. [ADR 0003](decisions/0003-preserve-compose-syntax-forms.md) defines the representation-fidelity rule.

The completed native boundary types images, build field identities, commands, environment,
environment-file declarations, service hostnames, explicit container names, independent service lifecycle booleans, labels, extra hosts, service capability additions and drops, raw user and
user-namespace values, ordered mixed service devices, raw service DNS servers and resolver options,
ordered service exposed ports, raw service security options, PID limits, shared-memory sizes, service-level temporary filesystems,
service sysctls, service logging drivers and options, image pull policies, independent stop signals and stop grace periods, ulimits, health checks,
dependencies, deploy field identities, ports, volumes, service networks, profiles, config and secret grants, and the
corresponding top-level network, volume, config, and secret definitions. Deferred interpolation
expressions, empty values, extensions, and unknown fields remain distinguishable. Container paths
are classified independently of host paths. The exact boundary and parse contract are documented
in the [typed model](typed-model.md) and
[ADR 0014](decisions/0014-issue-derived-native-model-expansion.md).

### Project loader

The implemented core loader receives ordered source text and explicit origins from the caller. It
retains every origin, establishes the first document's directory as the multi-file project base,
and aggregates recoverable parse diagnostics. It never reads files or environment variables.
Application adapters own discovery and I/O. ADR 0005 defines this [loading boundary](decisions/0005-explicit-ordered-project-loading.md).

`IncludeResolution` is an additive traversal layer above ordered loading. It merges each reached
node without interpolation, derives its effective typed declarations, and asks an
`IncludeLoader` to authorize and supply every child. It preserves the partial depth-first graph,
origins, requests, cycles, duplicate source IDs, and diagnostics, but never joins paths or reads
environment files. Its separate, opt-in `compose` pass recursively imports absent child services,
networks, volumes, configs, secrets, and model definitions after each parent merge. Same-name
collisions remain source-aware warnings and explicit records rather than invoking multi-file merge
rules. [ADR 0020](decisions/0020-caller-authorized-include-traversal.md) and
[ADR 0021](decisions/0021-include-composition-with-explicit-conflicts.md) define these boundaries.

The independent `plan_project_directories` pass supplies caller-owned effective directory planning
without changing loading or composition. It defaults root and undeclared children from their
retained first-document directories, while explicit declarations reach a resolver with source and
occurrence context. The resolver alone decides path/URI/opaque semantics; deferred and unresolved
results remain inspectable without exposing paths in diagnostics or debug output. [ADR 0022](decisions/0022-caller-owned-include-project-directory-plans.md)
defines that boundary.

### Processing pipeline

Merging, profile selection, interpolation, default application, and normalization are separate operations. Each operation consumes an explicit context and returns diagnostics plus a new view or transformation result.

The first Phase 3 components are the pure interpolation kernel and ordered loaded-project model.
Interpolation uses caller-supplied environment providers and retains original and resolved values,
substitution provenance, source spans, sensitivity, and redacted diagnostics. A project operation
creates one overlay per file before any merge. ADR 0004 defines the shared
[non-destructive processing-overlay policy](decisions/0004-explicit-processing-overlays.md).

The merge stage produces a ComposeLens-owned semantic tree rather than exposing concrete YAML
nodes. It recursively merges mappings, applies field-specific sequence rules and Compose merge
tags, resolves same-document YAML aliases and merge keys, and records every contributing span plus
the effective merge operation. The original syntax and per-document typed models remain available.
ADR 0006 records the [merge-view decision](decisions/0006-provenance-preserving-compose-merge.md).

Service `cap_add` and `cap_drop` are independent exact-scalar unique sequences in ordinary
multi-file merging. Exact case-sensitive duplicates collapse only after append and retain combined
item provenance and sensitivity. Case variants remain distinct. Reset produces an explicit empty
sequence, while override replaces the complete sequence without silently repairing duplicates in
its replacement. Neither field rewrites the other.

Service `devices` retains the repository's established Compose-Go-compatible unique-by-target
merge. Matching path targets replace or recursively merge in place with full contributor
provenance; reset and override remain explicit. Raw CDI, deferred, and opaque short strings are not
normalized. The current Compose merge prose's ordinary append exclusion does not list `devices`,
while Compose-Go's `extends` metadata does; this discrepancy is documented as evidence rather than
silently changing existing behavior.

Service `dns` and `dns_search` retain scalar/list form and use duplicate-preserving list
append; cross-form updates replace. `dns_opt` retains an ordered sequence and uses whole-sequence
replacement. None of these rules interprets resolver data.

Service `expose` uses exact text plus YAML scalar kind as its merge identity. Documented decimal
forms are classified without fixed-width parsing; unsupported and malformed forms remain raw.

Service `security_opt` uses raw sequence append. Exact AppArmor, no-new-privileges, seccomp,
SELinux-label, Mask, and Unmask shapes are exposed as independent lexical candidates. Near misses
and conflicts remain evidence, and no profile, path, provider, runtime, or cross-format policy is
selected.

Service `annotations` retains mapping/list syntax and uses keyed effective merging. Mapping keys do
not interpolate, contributors remain visible, and key-only list items stay diagnosed rather than
receiving label semantics.

Service-level `tmpfs` is an ordinary sequence when both files use list form: entries append without
deduplication so ordering, exact duplicates, item provenance, `!reset`, and `!override` remain
observable. Scalar/list shape mismatches use the normal replacement rule.

Service `sysctls` uses generic Compose collection semantics: mappings recursively merge by exact
key, lists append without deduplication, and map/list mismatches replace. Mapping keys remain
uninterpolated while mapping values and list items use each file's explicit interpolation overlay.
Exact duplicate list strings remain source-aware and diagnosed rather than silently removed.

Service `ulimits` is an ordered mapping whose outer names and nested `soft`/`hard` mappings use
generic recursive merge. Keys remain uninterpolated, values retain each file's explicit
interpolation result, scalar/range mismatches replace, reset produces an explicit empty mapping,
and override replaces the whole field. The project view exposes this behavior without applying
runtime defaults or resource semantics.

Service `logging` uses ordinary recursive mapping merge. `driver` remains an uninterpreted string;
colliding drivers and option values replace with complete provenance, while non-colliding ordered
options remain. Mapping keys never interpolate. Reset and override use the generic merge state.

The `project` module builds a native consumer view directly from a merged project and an optional
matching profile selection. It exposes the first conversion fields through native Compose types,
wraps effective values and collection items in complete `MergeProvenance`, retains all authored key
locations, and reports unmodeled fields without exposing parser nodes. Sensitive values redact
their contents from `Debug`. Canonical render-and-reparse is deliberately absent because generated
spans cannot replace original multi-file evidence. [ADR 0016](decisions/0016-native-merged-project-view.md)
defines the boundary.

Service labels are normalized by semantic key only in this effective view. Each label retains its
mapping, `KEY=VALUE`, or key-only list syntax and all contributing spans. A key-only list label has
an explicit empty-string semantic value; this differs from a key-only environment variable, which
requests host-environment resolution.
Service annotations are likewise keyed in the effective view, but retain authored/effective
scalar evidence, complete raw list items, contributor provenance, and sensitivity. Unlike labels,
a key-only annotation has no invented value and remains diagnosed.

Service environment-file declarations remain ordered and do not trigger I/O. The document model
retains scalar/list and long syntax; the project view preserves append order, item provenance, and
the provenance of long `path`, `required`, and `format` values. Reading or resolving those files is
an application concern, not a parsing or merge side effect.

Post-merge processing remains a set of independent views. Profile selection records active and
inactive services without deleting either. Path resolution uses retained origins plus explicit
caller context and performs no file access. Reference validation reports found, missing, and
profile-inactive targets plus healthy-dependency evidence gaps and contradictions. Default
resolution asks a caller-owned policy and records decisions
without rewriting omissions. A selection carries an exact merged-project snapshot so it cannot be
accidentally applied to another project that reuses source identifiers. ADR 0007 records these
[post-merge boundaries](decisions/0007-explicit-post-merge-views.md).

### Validation profiles

Validation is parameterized by an implemented profile:

- Compose Specification-oriented
- exact-version Docker Compose compatibility
- exact-version `containers/podman-compose` compatibility
- tolerant preservation

A profile classifies a construct as supported, extension, implementation-specific, deprecated,
unsupported, or unknown. Syntax validity and implementation support are separate questions. An
exact Compose provider and optional exact Docker or Podman backend runtime form the compatibility
context. `podman compose` is a delegating wrapper, not a provider identity.

Rules carry value-free explanations and version-scoped evidence. Findings retain feature identity,
semantic path, source span, sensitivity, and classification without storing the feature's raw
value. Supported findings remain queryable even when they produce no diagnostic. Unknown evidence
is never promoted to support merely because a tolerant profile was selected. ADR 0008 defines the
[compatibility model](decisions/0008-versioned-provider-runtime-compatibility.md).

### Renderer

The renderer has two deliberately separate paths:

- deterministic canonical output from a merged project, which is implemented; and
- deterministic generated output from caller-constructed Compose-native values, which is
  implemented for the first runtime-migration subset; and
- preservation-oriented editing from a syntax document, which is implemented for exact value
  scalars.

The implemented canonical-v1 renderer retains merged mapping and sequence order plus effective
Compose short/long forms. It uses fixed two-space indentation and double-quoted keys and string
scalars. An optional matching profile selection removes inactive services without removing
top-level resources. Rendering never performs another processing stage, and it does not claim
byte parity with `docker compose config`. Unresolved aliases and invalid retained tags produce
structured recovery diagnostics. Sensitive output is available explicitly but redacted from
`Debug`. ADR 0009 records the [canonical-rendering contract](decisions/0009-deterministic-canonical-rendering.md).

Preservation editing accepts typed string, boolean, number, and null replacements at exact
value-scalar spans. It validates a complete batch before applying descending byte-range patches to
the original source, so every unrelated byte stays unchanged and failures are atomic. Keys,
collections, aliases, empty values, block scalars, and multiline scalars remain outside the initial
boundary. Sensitive replacement values are redacted from debug output and diagnostics. ADR 0010
records the [preservation-edit contract](decisions/0010-atomic-span-based-preservation-edits.md).

`CanonicalFormatting` changes only indentation width, explicit LF/CRLF line endings, document-marker
emission, and final-line-ending emission. Its default remains byte-identical canonical-v1. It
cannot reorder data, normalize Compose forms, or activate any processing stage. ADR 0011 records
the [presentation-only formatting boundary](decisions/0011-presentation-only-render-formatting.md).

Generated rendering owns typed construction for documents without authored source provenance. It
uses the same private quoted-string encoder, retains insertion order, selects short or long syntax
per field, and reparses every successful result through the syntax and typed-document layers.
Sensitive generated values redact the complete result from `Debug`. It does not construct a fake
merged project or run compatibility validation. ADR 0017 records the
[generated-document boundary](decisions/0017-parse-back-validated-compose-generation.md).

Generated service labels use ordered mapping syntax with explicit quoted string values. This keeps
empty values and values containing `=` unambiguous, rejects duplicate names, and propagates
caller-marked value sensitivity to the complete generated document.
Generated service annotations use ordered mapping syntax and distinguish omission from an explicit
empty mapping. Only unique resolved non-empty names with explicit resolved quoted string values
are accepted; deferred, multiline, key-only, null, and malformed construction cannot succeed.
Generated service security options distinguish omission from one complete configured sequence,
including empty. Resolved non-empty single-line strings are quoted in order, exact duplicates are
retained, including exact duplicate seccomp, `label:disable`, `label:filetype:<type>`,
`label:level:<level>`, `label:nested`, `label:type:<type>`, `mask=<paths>`, and valid `unmask=<paths>` strings. No option, profile, JSON,
SELinux type, provider, runtime, or target-format normalization is applied.
Generated environment-file declarations preserve caller-selected short or long syntax and ordered
`required`/`format: raw` options. Paths use the same sensitivity boundary and are never opened,
resolved, or parsed during generation.
Generated capability-add and capability-drop declarations independently distinguish omission from
a configured vector, including empty. They preserve exact case and order, reject unsafe or
exact-duplicate items, quote every non-empty item, and apply no capability whitelist or target
normalization.
Generated service devices distinguish omission from an explicit empty vector and retain mixed
short/long order and duplicates. Every value is a quoted resolved single-line string; long source
is required. Generation does not inspect host devices, parse short colon triples, validate CDI or
permissions, or claim runtime access.
Generated service DNS values retain caller-selected scalar/list form, ordered duplicates, and
explicit empty lists. Non-empty resolved single-line strings are quoted and parse-back validated
without enforcing an IP/DNS grammar or performing resolver or network access.
Generated stop signals remain unconstrained raw strings because Compose defines no normative
signal-token grammar; quoted empty strings are retained as distinct from null. Generated stop
grace periods retain caller spelling under a `ComposeLens` policy based on the documented `us`,
`ms`, `s`, `m`, and `h` units; no target-runtime normalization occurs.
Generated PID limits use a non-exhaustive Compose-owned enum. Unlimited emits `-1`; finite values
retain arbitrary-precision positive ASCII-decimal spelling. Zero, signs, fractions, exponents,
expressions, and arbitrary strings cannot enter successful generated output.
Generated shared-memory sizes use a non-exhaustive Compose-owned enum plus an explicit documented
lowercase unit. Only canonical positive ASCII-integer amounts can be emitted, and the complete
value is always quoted. Omission, zero, leading zeros, signs, fractions, exponents, whitespace,
expressions, bare numbers, uppercase units, and IEC units cannot enter successful generated output.
No provider default, normalization, or runtime allocation is inferred.
Generated service `tmpfs` retains caller-selected scalar or list form, including explicit empty
lists, exact duplicates, and well-shaped raw target options. Values use `<path>[:<options>]`; the
documented option assignments are `mode`, `uid`, and `gid`, while other raw options remain
provider-dependent evidence rather than being normalized or rejected.
Generated service `sysctls` retains caller-selected mapping or list form, including explicit empty
collections. Mapping names are unique and every map value/list item is emitted as a quoted resolved
string; multiline, NUL-bearing, deferred, and exact-duplicate forms are rejected without applying
namespace, privilege, kernel, or target-runtime rules.
Generated service `ulimits` retains ordered single and required soft/hard forms plus an explicit
empty map. Lowercase names are unique and uninterpolated. Values are quoted resolved `-1` or
non-negative ASCII decimals; missing members, multiline, NUL-bearing, deferred, and arbitrary
schema strings are rejected without injecting provider defaults or claiming runtime enforcement.
Generated service `logging` requires one explicit string driver and an ordered unique non-empty-key
options mapping. Values retain explicit YAML string, number, or null kind; numeric spelling is
validated, output is deterministic, and no defaults or provider semantics are applied.
Generated service-network attachments retain aliases plus optional raw IPv4 and IPv6 addresses in
deterministic long syntax. Address omission, spelling, named-network scope, and sensitivity remain
explicit without IP grammar or IPAM-pool validation.
Generated top-level network definitions are distinct from the shared basic `GeneratedResource`
boundary and are application-owned; external networks remain on its compatible external path.
Optional opaque drivers and ordered unique options preserve string-versus-number YAML scalar
identity, sensitivity, and deterministic parse-back validation without plugin, provider, or runtime
semantics.
Generated top-level volume definitions likewise retain ordered unique `GeneratedLabel` mappings,
including explicit empty maps, deterministic parse-back output, and sensitivity without changing
the shared external-volume lifecycle API. Literal external volumes that also retain labels receive
a distinct source-aware diagnostic; driver and label violations remain independent.
Generated hostnames use a separate non-exhaustive Compose-owned API and accept only resolved ASCII
RFC-1123 values. They remain independent from explicit runtime container names; generation does
not derive either value from the other or synthesize a hostname when omitted.
Generated pull policies use a non-exhaustive Compose-owned enum for documented forms. Custom
`every_<duration>` values retain exact caller spelling and sensitivity while applying the schema
grammar `every_([0-9]+[wdhms])+`. Schema-valid `every_0s` remains representable even though its
prose semantics are ambiguous. Schema-only `refresh` and `pull_refresh_after` are not generated,
and no provider behavior or cross-format equivalence is inferred.

## Dependency direction

- Syntax knows nothing about the typed model.
- The typed model may refer to syntax locations but not parser internals.
- Project processing depends on typed documents and caller-provided inputs, not ambient I/O.
- Native project views depend on merged values and profile selections, not on rendering or BoxFerry.
- Validation depends on models and profiles, not on BoxFerry.
- Rendering depends on syntax, merged projects, or explicit generated Compose values according to
  the selected mode.

## Side-effect boundaries

- Parsing text is pure.
- Interpolation is pure when supplied an immutable environment provider.
- Core project loading receives caller-supplied text and performs no file access.
- Host-path resolution is lexical and uses only caller-supplied path context.
- Default resolution is pure when supplied an immutable default provider.
- Compatibility validation is pure and never invokes a Compose provider or container runtime.
- Canonical rendering is pure and consumes only a merged project plus an optional matching profile selection.
- Generated rendering is pure and consumes only caller-constructed Compose-native values.
- Custom rendering additionally consumes an explicit presentation-only `CanonicalFormatting` value.
- Preservation editing is pure and consumes only a syntax document plus caller-supplied exact-span edits.
- ComposeLens never contacts Docker, Podman, or Kubernetes.
- ComposeLens never starts services or builds images.

The repository's ignored conformance harness is intentionally outside the library boundary. It
invokes an absolute caller-selected provider only when explicitly requested, clears ambient
environment variables, and writes an unreviewed result to a caller-selected new directory. ADR
0012 defines this [evidence-collection boundary](decisions/0012-repository-conformance-harness.md).

## Public release boundary

ComposeLens publishes one library crate. The supported 0.2.x surface follows the layer boundaries
above and exposes only ComposeLens-owned types; parser dependencies remain private. Patch releases
preserve the documented consumer path, diagnostic code strings, side-effect boundaries, and
canonical-v1 defaults. ADR 0019 defines the
[versioned public API and release contract](decisions/0019-consolidated-0.2-public-api.md).
