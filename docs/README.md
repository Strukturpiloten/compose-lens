# ComposeLens documentation

This directory defines ComposeLens's public intent and internal architecture.

## Start here

- [Architecture](architecture.md) — layers, dependency direction, and invariants
- [Project structure](project-structure.md) — intended crate and module organization
- [Processing model](processing-model.md) — explicit document and project stages
- [Preservation-oriented editing](preservation-editing.md) — atomic scalar edits, style behavior, diagnostics, and limits
- [Render formatting](render-formatting.md) — indentation, line endings, document markers, and the semantic boundary
- [Generated documents](generated-rendering.md) — typed construction, syntax-form selection, parse-back validation, and redaction
- [Phase 2 typed model](typed-model.md) — implemented field boundary, fidelity rules, and diagnostics
- [Native coverage](coverage.md) — syntax, document-model, and merged-project field coverage
- [Testing strategy](testing.md) — parser, resolver, conformance, and round-trip tests
- [Development environment](development-environment.md) — reproducible VS Code tooling and update policy
- [Compose implementation conformance](conformance.md) — exact matrices, evidence lifecycle, and open runtime work
- [Real-world fixture corpus](real-world-corpus.md) — admission, licensing, sanitization, and covered projects
- [Fixture format](fixture-format.md) — shared metadata, provenance, and secrets contract
- [YAML representation evaluation](research/yaml-representation.md) — versioned parser prototype evidence
- [Compose syntax-fidelity evidence](research/compose-syntax-fidelity.md) — why short and long forms remain distinct
- [Compose interpolation evidence](research/compose-interpolation.md) — operators, ordering, providers, and redaction
- [Compose merge evidence](research/compose-merge.md) — ordering, field-specific rules, tags, and path bases
- [Post-merge processing evidence](research/compose-post-merge-processing.md) — profiles, paths, references, and defaults
- [Compatibility-profile evidence](research/compose-compatibility-profiles.md) — providers, runtimes, versions, and initial rules
- [Provider-config conformance results](research/provider-config-conformance-2026-07-31.md) — 48 exact reviewed observations
- [Podlet and compose_spec_rs regression review](research/podlet-compose-spec-rs-regressions-2026-08-01.md) — upstream user cases, existing coverage, and post-0.1 candidates
- [Canonical-rendering evidence](research/compose-canonical-rendering.md) — fixed output, explicit processing boundary, recovery, and redaction
- [Dependency and license policy](dependency-policy.md) — dependency selection, allowed sources, and license checks
- [API stability policy](api-stability.md) — pre-1.0 compatibility and public dependency boundaries
- [0.1.13 release notes](releases/0.1.13.md) — generated service environment files
- [0.1.12 release notes](releases/0.1.12.md) — source-aware service environment files
- [Release process](releasing.md) — Cargo versioning, crates.io authentication, and GitHub releases
- [Changelog](../CHANGELOG.md) — release-facing behavior changes
- [Implementation plan](implementation-plan.md) — synchronized cross-repository tasks T1–T8
- [Architecture decisions](decisions/README.md) — durable design choices

## Documentation rules

- Clearly distinguish Compose Specification requirements from observed implementation behavior.
- Attach authoritative links, implementation versions, and test evidence to compatibility claims.
- Document whether an operation is pure, reads files, consumes environment values, or invokes another process.
- Update processing-stage documentation whenever ordering or default behavior changes.
- Use ADRs for decisions affecting representation, public APIs, loss preservation, or compatibility policy.

Coding agents must also follow the repository-root `AGENTS.md`.
