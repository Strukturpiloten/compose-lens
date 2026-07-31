# Repository guidance for coding agents

This file applies to the entire ComposeLens repository.

## Read before changing code

Read these documents in order:

1. `README.md`
2. `docs/architecture.md`
3. `docs/project-structure.md`
4. `docs/processing-model.md`
5. `docs/testing.md`
6. `docs/decisions/README.md` and all accepted ADRs

Architectural changes require documentation and an ADR update in the same change.

## Scope

ComposeLens owns native Compose syntax, native Compose models, project loading, merging, profile selection, interpolation, validation profiles, rendering, source locations, and diagnostics.

ComposeLens does not own cross-format conversion, runtime inspection, Quadlet, Kubernetes deployment policy, or BoxFerry's application model. It must not depend on BoxFerry.

## Origin policy

ComposeLens is implemented from scratch. Do not copy or mechanically translate source code from `compose_spec_rs`, Docker Compose, Podman Compose, Podlet, or another parser. Specifications, public documentation, and observable behavior may inform independent implementation. Differential fixtures must record the implementation, version, command, environment inputs, and expected result.

Third-party parsing dependencies require deliberate review. Record choices that constrain the document model or round-trip behavior in an ADR.

## Non-negotiable behavior

- Parsing never reads process environment variables unless a caller explicitly requests interpolation through an environment provider.
- Unknown fields and `x-*` extensions are not silently discarded.
- Preserve original scalar text when it carries meaning that a normalized value would lose.
- Image references accepted by supported real implementations must not be rejected solely by unrelated OCI normalization rules.
- Duplicate, merged, and overridden values retain enough provenance for diagnostics.
- Invalid user input returns structured errors and never panics.
- Rendering is deterministic for the same document and options.
- Secrets and interpolated values are redacted in diagnostics by default.

## Development rules

- Keep syntax representation, typed model, project processing, and validation as distinct modules.
- Do not make the typed model depend on BoxFerry types.
- Make processing stages explicit; avoid a single convenience function that hides file access, merging, interpolation, validation, and normalization.
- Add conformance and regression fixtures with every parsing or resolution behavior change.
- Store source, license, implementation version, and environment metadata for external fixtures.
- Update documentation and compatibility claims with behavior changes.

No Rust crate exists yet. Once scaffolded, add the canonical formatting, linting, testing, documentation, and compatibility commands here and in `docs/testing.md`.
