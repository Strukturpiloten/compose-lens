# Software architecture

## Purpose

ComposeLens provides a source-aware, tolerant, and typed representation of Compose documents while keeping project processing operations explicit. It supports both analysis and deterministic rewriting without forcing callers into a fully resolved configuration.

## Layers

```text
bytes or text
     │
     ▼
syntax document ──▶ typed document ──▶ loaded project ──▶ semantic view
     │                    │                  │                  │
     ├──▶ source map      ├──▶ extensions    ├──▶ overlays      ├──▶ validation
     │                    │                  ├──▶ profiles      ├──▶ normalization
     │                    │                  ├──▶ interpolation │
     │                    │                  │                  │
     └────────────────────┴──────────────────┴──────────────────┴──▶ renderer
```

### Source and syntax layer

The syntax layer owns YAML representation, comments where supported, scalar spelling, anchors and aliases, mappings, sequences, duplicate-key diagnostics, and byte/span locations.

It must represent syntactically valid input even when the typed Compose model does not recognize every field.

The initial implementation stores an immutable loss-aware concrete syntax tree plus the original source. [`yaml-edit`](https://docs.rs/yaml-edit/0.2.3/yaml_edit/) is private behind ComposeLens-owned APIs as decided in [ADR 0002](decisions/0002-loss-aware-yaml-syntax.md). Recoverable malformed input produces both a renderable syntax document and structured diagnostics.

### Typed document model

The typed layer exposes Compose concepts such as services, networks, volumes, configs, secrets, build configuration, dependencies, and deployment settings.

Typed nodes retain source references and unknown fields. Parsing into typed data must not destroy the syntax document required for a later loss-aware render.

Short and long syntax remain distinct field-specific variants. They are not normalized merely because they describe similar concepts: defaults, available options, and runtime behavior can differ. [ADR 0003](decisions/0003-preserve-compose-syntax-forms.md) defines the representation-fidelity rule.

The completed Phase 2 boundary types images, commands, environment, ports, volumes, service networks, profiles, config and secret grants, and the corresponding top-level network, volume, config, and secret definitions. Deferred interpolation expressions, empty values, extensions, and unknown fields remain distinguishable. The exact boundary and parse contract are documented in the [Phase 2 typed model](typed-model.md).

### Project loader

The implemented core loader receives ordered source text and explicit origins from the caller. It
retains every origin, establishes the first document's directory as the multi-file project base,
and aggregates recoverable parse diagnostics. It never reads files or environment variables.
Application adapters own discovery and I/O; include handling and multi-file composition remain
separate processing behavior. ADR 0005 defines this [loading boundary](decisions/0005-explicit-ordered-project-loading.md).

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

Post-merge processing remains a set of independent views. Profile selection records active and
inactive services without deleting either. Path resolution uses retained origins plus explicit
caller context and performs no file access. Reference validation reports found, missing, and
profile-inactive targets. Default resolution asks a caller-owned policy and records decisions
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

## Dependency direction

- Syntax knows nothing about the typed model.
- The typed model may refer to syntax locations but not parser internals.
- Project processing depends on typed documents and caller-provided I/O abstractions.
- Validation depends on models and profiles, not on BoxFerry.
- Rendering depends on syntax or typed models according to the selected mode.

## Side-effect boundaries

- Parsing text is pure.
- Interpolation is pure when supplied an immutable environment provider.
- Core project loading receives caller-supplied text and performs no file access.
- Host-path resolution is lexical and uses only caller-supplied path context.
- Default resolution is pure when supplied an immutable default provider.
- Compatibility validation is pure and never invokes a Compose provider or container runtime.
- Canonical rendering is pure and consumes only a merged project plus an optional matching profile selection.
- Custom rendering additionally consumes an explicit presentation-only `CanonicalFormatting` value.
- Preservation editing is pure and consumes only a syntax document plus caller-supplied exact-span edits.
- ComposeLens never contacts Docker, Podman, or Kubernetes.
- ComposeLens never starts services or builds images.
