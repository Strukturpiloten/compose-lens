# Compose compatibility

Parsing a Compose key and proving that a Compose implementation behaves a certain way are different claims. ComposeLens keeps them separate.

Use `validation::validate_compatibility` with an explicit `CompatibilityProfile` and `ImplementationVersion` to request evidence-backed findings. No host installation or development-machine version is selected automatically.

## Evidence boundary

Compatibility rules may be based on specification text, exact provider output, or reviewed runtime observations. Each claim stays scoped to the implementation, version, command, fixture, and environment that produced the evidence. Accepting or rendering syntax does not prove runtime behavior.

The maintained matrices and their review state live in the repository's [conformance documentation](https://github.com/Strukturpiloten/compose-lens/blob/main/docs/conformance.md). Ordinary tests use retained fixtures and do not contact Docker, Podman, or a network service.

## Choose a validation mode

- Use syntax and model diagnostics to answer whether ComposeLens understood a document.
- Use reference validation to check links between selected project resources.
- Use compatibility validation only after choosing the implementation and version you target.
- Treat a missing rule as unknown, not as proof of support.

ComposeLens reports evidence; the calling application owns release policy and whether a finding blocks work.
