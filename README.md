# ComposeLens

ComposeLens is a Rust library for reading, understanding, transforming, validating, and rendering real-world Compose documents.

It is designed for tools that need more than strict deserialization: source-aware diagnostics, implementation extensions, optional interpolation, multi-file projects, profiles, and round-trip-safe transformations.

## Goals

- Parse Compose YAML without forcing immediate normalization or interpolation.
- Represent syntax, typed Compose concepts, extensions, and unknown fields explicitly.
- Preserve field-specific short and long syntax when their defaults or runtime behavior can differ.
- Preserve enough source information for actionable diagnostics and safe editing.
- Support multi-file project loading, merging, profile selection, and configurable interpolation.
- Model behavior found in real Docker Compose and Podman Compose projects.
- Render deterministic Compose documents.
- Construct new deterministic Compose documents through Compose-owned, parse-back-validated values.
- Preserve, merge, inspect, and generate service metadata labels without exposing sensitive values
  through debug output.
- Preserve, merge, inspect, and generate explicit runtime container names with field provenance.
- Preserve, merge, inspect, and generate service-level restart policies without confusing them
  with dependency-update or deploy restart settings.
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
  → optional profile selection and native project view
  → references, paths, defaults, and compatibility
  → validated semantic view
  → rendered Compose document
```

Callers may stop at any appropriate level. Parsing a document must not implicitly read environment variables or contact a runtime.

## Documentation

- [Published Rust API documentation](https://docs.rs/compose-lens/latest/compose_lens/)
- [Documentation index](docs/README.md)
- [Software architecture](docs/architecture.md)
- [Target project structure](docs/project-structure.md)
- [Processing model](docs/processing-model.md)
- [Preservation-oriented editing](docs/preservation-editing.md)
- [Render formatting](docs/render-formatting.md)
- [Generated documents](docs/generated-rendering.md)
- [Phase 2 typed model](docs/typed-model.md)
- [Native coverage](docs/coverage.md)
- [Testing strategy](docs/testing.md)
- [Development environment](docs/development-environment.md)
- [Compose implementation conformance](docs/conformance.md)
- [Real-world fixture corpus](docs/real-world-corpus.md)
- [Cross-repository implementation plan](docs/implementation-plan.md)
- [API stability policy](docs/api-stability.md)
- [0.1.11 release notes](docs/releases/0.1.11.md)
- [Release process](docs/releasing.md)
- [Changelog](CHANGELOG.md)
- [Architecture decisions](docs/decisions/README.md)

Repository-specific guidance for coding agents is in [AGENTS.md](AGENTS.md).

## Origin

ComposeLens is implemented from scratch. It is not a fork of `compose_spec_rs` and does not copy or mechanically translate its source code.

## Stewardship

ComposeLens is created and maintained by [Martin “Becks” Beckert](https://github.com/TheRealBecks) through [Strukturpiloten OHG](https://www.strukturpiloten.de/). The project is part of Strukturpiloten's work on open, maintainable, and portable container infrastructure.

## License

ComposeLens is licensed under the [Mozilla Public License 2.0](LICENSE).
