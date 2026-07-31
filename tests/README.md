# ComposeLens test suites

Executable integration-test entry points live directly in this directory so Cargo discovers them. Shared test-only helpers live in `support/` and must not become part of the public library API.

Suites are introduced with the behavior they verify:

- `repository-policy` — fixture metadata and repository security invariants
- `syntax` — YAML-facing syntax, malformed input, spans, and recovery
- `typed-model` — native Compose types and short/long forms
- `processing` — loading, merging, interpolation, profiles, paths, references, and defaults
- `compatibility` — exact implementation versions, provider/runtime profiles, evidence, and findings
- `roundtrip` — preservation and deterministic canonical rendering
- `rendering` — canonical/custom output, formatting boundaries, profile filtering, recovery, stability, and redaction
- `preservation-editing` — atomic exact-span scalar changes and byte-preservation failures
- `conformance` — behavior observed from identified Compose implementations
- `real-world` — licensed external projects and regression cases

Do not add an empty test binary merely to reserve a suite name. Add the entry point, its fixtures, and meaningful assertions together.
