# ComposeLens documentation

Start with the [public ComposeLens guide](public/index.md) when using the library. The generated
[Rust API](https://docs.rs/compose-lens) is the source of truth for individual types and methods.

## Design

- [Architecture](architecture.md) — ownership, layers, dependencies, and side-effect boundaries
- [Processing model](processing-model.md) — explicit stages from source text to project views
- [Rendering](rendering.md) — canonical output, generated documents, and preservation edits
- [API stability](api-stability.md) — the supported pre-1.0 release contract
- [Native coverage](coverage.md) — what “typed”, “generated”, and “compatible” mean

## Maintain

- [Testing](testing.md) — test layers, complete checks, and behavioral expectations
- [Development environment](development-environment.md) — supported tools and focused commands
- [Dependency policy](dependency-policy.md) — sources, licenses, and representation dependencies
- [Release process](releasing.md) — release-plz preparation and protected publication
- [Fixture policy](../fixtures/README.md) — manifests, licensing, secrets, and real-world corpus
- [Conformance evidence](../conformance/README.md) — versioned provider observations

## Evidence and history

- [Research index](research/README.md) — versioned technical evidence behind decisions
- [Architecture decisions](decisions/README.md) — durable accepted and superseded decisions
- [Changelog](../CHANGELOG.md) — release-facing behavior changes

Live work belongs in GitHub issues and the BoxFerry product roadmap. This repository does not keep a
second cross-repository implementation plan or a completed-task archive.
