# ComposeLens

ComposeLens is a Rust library for reading, understanding, transforming, validating, and rendering real-world Compose documents.

It is designed for tools that need more than strict deserialization: source-aware diagnostics, implementation extensions, optional interpolation, multi-file projects, profiles, and round-trip-safe transformations.

> [!IMPORTANT]
> ComposeLens is in early development. Its loss-aware YAML syntax API, Phase 2 source-aware typed model, explicit project processing, version-aware compatibility profiles, deterministic merged-project renderer, presentation-only formatting, and atomic value-scalar editing are available. The typed boundary covers the service and resource fields required by the first BoxFerry Compose-to-Quadlet conversion; structural editing, broad runtime conformance matrices, the complete Compose model, and a stable API are not yet available.

## Goals

- Parse Compose YAML without forcing immediate normalization or interpolation.
- Represent syntax, typed Compose concepts, extensions, and unknown fields explicitly.
- Preserve field-specific short and long syntax when their defaults or runtime behavior can differ.
- Preserve enough source information for actionable diagnostics and safe editing.
- Support multi-file project loading, merging, profile selection, and configurable interpolation.
- Model behavior found in real Docker Compose and Podman Compose projects.
- Render deterministic Compose documents.
- Allow callers to choose strict, implementation-specific, or tolerant validation profiles.

## Non-goals

- Running a Compose project
- Reimplementing Docker or Podman
- Converting Compose directly to Quadlet or Kubernetes
- Enforcing OCI rules that real Compose implementations do not enforce
- Treating the Compose Specification as the only source of real-world behavior

Cross-format conversion belongs to [BoxFerry](https://github.com/Strukturpiloten/boxferry). Quadlet handling belongs to [QuadletLens](https://github.com/Strukturpiloten/quadlet-lens).

## Processing levels

```text
source text
  → loss-aware syntax document
  → typed Compose document
  → loaded multi-file project
  → optional per-file interpolation
  → provenance-preserving merged project
  → selected profiles, references, paths, and defaults
  → validated semantic view
  → rendered Compose document
```

Callers may stop at any appropriate level. Parsing a document must not implicitly read environment variables or contact a runtime.

## Documentation

- [Documentation index](docs/README.md)
- [Software architecture](docs/architecture.md)
- [Target project structure](docs/project-structure.md)
- [Processing model](docs/processing-model.md)
- [Preservation-oriented editing](docs/preservation-editing.md)
- [Render formatting](docs/render-formatting.md)
- [Phase 2 typed model](docs/typed-model.md)
- [Testing strategy](docs/testing.md)
- [Cross-repository implementation plan](docs/implementation-plan.md)
- [API stability policy](docs/api-stability.md)
- [Roadmap](docs/roadmap.md)
- [Architecture decisions](docs/decisions/README.md)

Repository-specific guidance for coding agents is in [AGENTS.md](AGENTS.md).

## Origin

ComposeLens is implemented from scratch. It is not a fork of `compose_spec_rs` and does not copy or mechanically translate its source code.

## Stewardship

ComposeLens is created and maintained by [Martin “Becks” Beckert](https://github.com/TheRealBecks) through [Strukturpiloten OHG](https://www.strukturpiloten.de/). The project is part of Strukturpiloten's work on open, maintainable, and portable container infrastructure.

## License

ComposeLens is licensed under the [Mozilla Public License 2.0](LICENSE).
