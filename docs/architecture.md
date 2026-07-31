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

### Typed document model

The typed layer exposes Compose concepts such as services, networks, volumes, configs, secrets, build configuration, dependencies, and deployment settings.

Typed nodes retain source references and unknown fields. Parsing into typed data must not destroy the syntax document required for a later loss-aware render.

### Project loader

The loader finds or receives project files and related environment sources through caller-provided interfaces. File discovery, include handling, path origins, and multi-file composition belong here rather than in the parser.

### Processing pipeline

Merging, profile selection, interpolation, default application, and normalization are separate operations. Each operation consumes an explicit context and returns diagnostics plus a new view or transformation result.

### Validation profiles

Validation is parameterized by a profile, for example:

- Compose Specification-oriented
- Docker Compose compatibility
- Podman Compose compatibility
- tolerant preservation

A profile may classify a construct as supported, extension, implementation-specific, deprecated, or invalid. Syntax validity and implementation support are separate questions.

### Renderer

The renderer supports at least:

- preservation-oriented editing from the syntax document
- canonical deterministic output from a typed or processed model

Canonical rendering does not claim byte-for-byte round trips.

## Dependency direction

- Syntax knows nothing about the typed model.
- The typed model may refer to syntax locations but not parser internals.
- Project processing depends on typed documents and caller-provided I/O abstractions.
- Validation depends on models and profiles, not on BoxFerry.
- Rendering depends on syntax or typed models according to the selected mode.

## Side-effect boundaries

- Parsing text is pure.
- Interpolation is pure when supplied an immutable environment provider.
- Loading files is isolated behind loader interfaces.
- ComposeLens never contacts Docker, Podman, or Kubernetes.
- ComposeLens never starts services or builds images.
