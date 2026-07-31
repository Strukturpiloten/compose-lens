# ComposeLens

ComposeLens is a Rust library for reading, understanding, transforming, validating, and rendering real-world Compose documents.

It is designed for tools that need more than strict deserialization: source-aware diagnostics, implementation extensions, optional interpolation, multi-file projects, profiles, and round-trip-safe transformations.

> [!IMPORTANT]
> ComposeLens is in its initial design phase. It does not yet provide a usable crate or a stable API.

## Goals

- Parse Compose YAML without forcing immediate normalization or interpolation.
- Represent syntax, typed Compose concepts, extensions, and unknown fields explicitly.
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

## Planned processing levels

```text
source text
  → loss-aware syntax document
  → typed Compose document
  → loaded multi-file project
  → selected profiles and optional interpolation
  → validated semantic view
  → rendered Compose document
```

Callers may stop at any appropriate level. Parsing a document must not implicitly read environment variables or contact a runtime.

## Documentation

- [Documentation index](docs/README.md)
- [Software architecture](docs/architecture.md)
- [Target project structure](docs/project-structure.md)
- [Processing model](docs/processing-model.md)
- [Testing strategy](docs/testing.md)
- [Roadmap](docs/roadmap.md)
- [Architecture decisions](docs/decisions/README.md)

Repository-specific guidance for coding agents is in [AGENTS.md](AGENTS.md).

## Origin

ComposeLens is implemented from scratch. It is not a fork of `compose_spec_rs` and does not copy or mechanically translate its source code.

## License

ComposeLens is licensed under the [Mozilla Public License 2.0](LICENSE).
