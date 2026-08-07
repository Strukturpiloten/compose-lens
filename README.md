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
- Preserve, merge, inspect, and generate explicit service hostnames with conservative RFC-1123
  validation, deferred-expression retention, and no synthesized default.
- Preserve, merge, inspect, and generate service-level restart policies without confusing them
  with dependency-update or deploy restart settings.
- Preserve, merge, inspect, and generate service image pull policies without inventing provider
  behavior or discarding schema-only refresh evidence.
- Preserve, merge, inspect, and generate independent service stop signals and raw Compose stop
  grace periods without normalizing them into another lifecycle manager's units.
- Preserve, merge, inspect, and generate service PID limits without fixed-width parsing, default
  injection, runtime probing, or conflation with deploy resource limits.
- Preserve, merge, inspect, and safely generate service shared-memory sizes without injecting the
  Podman default, normalizing provider-dependent values, or inspecting `/dev/shm`.
- Preserve, merge, inspect, and safely generate service memory limits without fixed-width parsing,
  provider/runtime enforcement, host/cgroup inspection, or conflation with deploy memory policy.
- Preserve, merge, inspect, and generate service-level `tmpfs` scalar/list forms, colon-delimited
  documented or raw options, duplicates, provenance, and sensitivity without conflating volume mounts.
- Preserve, merge, inspect, and safely generate service `sysctls` mapping/list forms, scalar
  spelling, order, provenance, and sensitivity without namespace or runtime interpretation.
- Preserve, recursively merge, inspect, and safely generate ordered service `ulimits` single and
  soft/hard forms without injecting defaults or claiming runtime enforcement.
- Preserve, merge, inspect, and generate independent ordered service `cap_add` and `cap_drop`
  strings with explicit empty state, exact-case uniqueness, provenance, and no capability
  whitelist or target normalization.
- Preserve, target-key merge, inspect, and safely generate ordered mixed short/long service
  `devices`, including raw CDI/deferred/opaque short forms, explicit empty state, duplicates,
  nested provenance, and no host-device, permissions, or runtime-access validation.
- Preserve, merge, inspect, and safely generate raw service DNS settings and exposed ports without
  resolver, network, or runtime access.
- Preserve and safely generate keyed service annotations without losing mapping/list syntax or
  ambiguous key-only entries.
- Preserve, recursively merge, inspect, and safely generate service logging drivers and ordered
  string/number/null options without interpreting provider semantics.
- Preserve raw service security options and expose conflict-aware lexical candidates without
  selecting policy, inspecting the host, or claiming runtime behavior.
- Preserve, inspect, and generate ordered service `env_file` short/long syntax and options; retain
  interpolation and multi-file provenance without reading environment files.
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
- [Roadmap and exact specification gaps](docs/roadmap.md)
- [Testing strategy](docs/testing.md)
- [Development environment](docs/development-environment.md)
- [Compose implementation conformance](docs/conformance.md)
- [Real-world fixture corpus](docs/real-world-corpus.md)
- [Cross-repository implementation plan](docs/implementation-plan.md)
- [API stability policy](docs/api-stability.md)
- [0.1.16 release notes](docs/releases/0.1.16.md) — logging and generated network and volume configuration
- [0.1.15 release notes](docs/releases/0.1.15.md) — DNS, expose, annotations, and security options
- [0.1.14 release notes](docs/releases/0.1.14.md)
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
