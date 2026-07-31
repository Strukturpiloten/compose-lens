# ComposeLens documentation

This directory defines ComposeLens's public intent and internal architecture.

## Start here

- [Architecture](architecture.md) — layers, dependency direction, and invariants
- [Project structure](project-structure.md) — intended crate and module organization
- [Processing model](processing-model.md) — explicit document and project stages
- [Testing strategy](testing.md) — parser, resolver, conformance, and round-trip tests
- [Fixture format](fixture-format.md) — shared metadata, provenance, and secrets contract
- [YAML representation evaluation](research/yaml-representation.md) — versioned parser prototype evidence
- [Dependency and license policy](dependency-policy.md) — dependency selection, allowed sources, and license checks
- [API stability policy](api-stability.md) — pre-1.0 compatibility and public dependency boundaries
- [Implementation plan](implementation-plan.md) — synchronized cross-repository tasks T1–T7
- [Roadmap](roadmap.md) — implementation order
- [Architecture decisions](decisions/README.md) — durable design choices

## Documentation rules

- Clearly distinguish Compose Specification requirements from observed implementation behavior.
- Attach authoritative links, implementation versions, and test evidence to compatibility claims.
- Document whether an operation is pure, reads files, consumes environment values, or invokes another process.
- Update processing-stage documentation whenever ordering or default behavior changes.
- Use ADRs for decisions affecting representation, public APIs, loss preservation, or compatibility policy.

Coding agents must also follow the repository-root `AGENTS.md`.
