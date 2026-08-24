# Native Compose coverage

ComposeLens separates recognizing source from understanding project meaning and proving provider
behavior. A field is not “supported everywhere” merely because one layer can parse it.

## Coverage dimensions

| Dimension           | Claim                                                                                       |
| ------------------- | ------------------------------------------------------------------------------------------- |
| Syntax preservation | Valid or recoverable YAML remains source-addressable; unknown fields and extensions survive |
| Document model      | One authored document exposes a native type and its syntax alternatives                     |
| Project view        | Effective multi-file values expose merge and profile provenance                             |
| Generated output    | Caller-constructed values have deterministic, parse-back-validated YAML                     |
| Compatibility       | A finding is backed by version-scoped specification or implementation evidence              |

These dimensions are independent. A document-only type does not imply effective merge behavior;
project-view support does not imply generated construction; generated output does not prove that a
provider applies the value successfully.

## Current boundary

The machine-readable [`compose-key-inventory.json`](../schema/compose-key-inventory.json) is the
source of truth for the current closed top-level and immediate service-key audit. It records the
reviewed Compose Specification commit and classifies all nine root keys and 93 immediate service
keys. Repository-policy tests fail if that snapshot or classification drifts.

All keys in that bounded inventory have source-aware document-model coverage. Project-view and
generated-document coverage are intentionally narrower and consumer-driven. Nested values are
promoted only after their syntax, merge, recovery, provenance, and security behavior is defined.
Unknown or future nested content remains available through source-aware unmodeled fields.

The exact public types are documented in Rustdoc. Executable behavior is covered by
`tests/typed_model.rs`, `tests/processing.rs`, `tests/generated_rendering.rs`, and
`tests/public_api.rs`; this document does not duplicate their field lists.

## Compatibility evidence

Compose Specification acceptance, provider configuration output, and runtime effects are separate
claims. Compatibility rules cite only reviewed evidence for an explicit provider/runtime version
range. Missing evidence stays unknown.

Retained observations and planned probes live under [`conformance/`](../conformance/README.md).
Ordinary pull-request tests validate the matrices and records without invoking Docker, Podman, or a
network service.

## Promoting coverage

A change that promotes a field should normally provide:

1. authored positive and malformed/recovery cases;
2. field-specific short, long, empty, reset, override, and merge behavior where applicable;
3. source and sensitivity propagation into the effective project view;
4. deterministic output and parse-back tests when generation is needed; and
5. explicit unknown or version-scoped findings when provider behavior matters.

Generated construction remains demand-driven. New work is tracked in GitHub issues rather than a
second static field roadmap.
