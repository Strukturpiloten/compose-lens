# Testing strategy

ComposeLens must be built around tests because YAML syntax, Compose processing, and implementation behavior contain many interacting edge cases.

## Test layers

### Syntax tests

Cover scalars, mappings, sequences, anchors, aliases, comments where supported, duplicate keys, malformed YAML, Unicode, line endings, spans, and error recovery.

### Typed-model tests

Cover every field in the documented [Phase 2 boundary](typed-model.md), all supported syntax variants, unknown fields, extensions, image references, ports, volumes, environment values, commands, networks, profiles, configs, secrets, top-level resources, and discriminated unions.

### Processing tests

Cover file ordering, merge rules, reset/override behavior, interpolation operators, `.env` handling, profile selection, include behavior, path origins, defaults, and reference resolution.

### Round-trip and property tests

Verify that parsing never panics, preservation edits retain unrelated syntax, canonical output is deterministic, and supported typed values survive parse-render-parse cycles.

The implemented canonical-rendering tier compares exact golden bytes, repeats rendering to prove
determinism, and verifies parse-merge-render stability. It also covers profile filtering, retained
tags, unresolved-alias recovery, and sensitive-output redaction. A regression fixture places an
empty environment value immediately before later service fields so parser recovery cannot silently
reparent ports, volumes, or extensions into the environment mapping.

Preservation-editing tests compare exact authored and expected files after changing typed scalar
spans. They prove that comments, whitespace, ordering, unknown fields, extensions, flow syntax, and
untouched quoting stay byte-identical. Failure tests cover foreign sources, key and non-scalar
targets, overlaps, block scalars, invalid numbers, atomic rollback, successful reparsing, and
sensitive replacement redaction.

Formatting tests prove that default options remain byte-identical canonical-v1, custom indentation
and line-ending output is exact, zero-width indentation is unrepresentable, document/final markers
are explicit, and customized output reparses and merges to the same canonical semantic bytes.

### Implementation conformance tests

Run selected fixtures through known Docker Compose and Podman Compose versions. Record:

- exact implementation and version
- command and arguments
- input files and environment map
- working directory assumptions
- stdout, stderr, exit status, and normalized result

Observed behavior becomes evidence, not an unquestionable specification. Conflicts between implementations are represented through compatibility profiles.

### Real-world fixtures

Use licensed projects and minimal reproductions of reported behavior. Every fixture needs provenance, redistribution permission, secret review, and a statement of what it protects.

## Test organization

Cargo-discovered integration tests live in [`../tests/`](../tests/README.md), with private helpers in `tests/support/`. Fixtures live in [`../fixtures/`](../fixtures/README.md) and are validated against the versioned [fixture manifest contract](fixture-format.md). Product suites are added only with implemented behavior and meaningful assertions.

The initial syntax corpus exercises comments, anchors, aliases, duplicate keys, extension fields, scalar spelling, interpolation-shaped text, tag-plus-digest image references, Unicode, CRLF input, malformed flow syntax, source spans, and exact parse/render/parse stability. The Phase 2 typed-model corpus covers its complete field boundary, deferred expressions, empty and null values, extensions and unknowns, partial invalid-input recovery, and stable source-spanned diagnostics. It also keeps short `:z`, short `:Z,ro`, and long `bind.selinux: Z` volume mounts distinct. The first processing corpus protects every interpolation operator, nested expressions, escaped dollars, missing-variable policies, required-value redaction, sensitivity propagation, and nesting recovery. It also covers ordered multi-file loading, explicit origin retention, first-file base-directory selection, duplicate source IDs, recoverable diagnostics, and one per-file interpolation overlay before merge. Field-aware merge fixtures cover mapping recursion, ordinary append, command replacement, mixed environment and label forms, unique ports, volumes, devices, configs, and secrets, YAML merge keys, unknown fields, reset/override tags, provenance, and sensitive-value debug redaction. Post-merge fixtures cover explicit and all-profile selection, profile reset behavior, inactive-service exclusion, relative and caller-supplied home path origins, named-resource references, inactive and missing service edges, documented defaults, no-default policy behavior, and rejection of selections from another project. Compatibility fixtures cover exact version parsing and ranges, selected-service feature discovery, Docker's documented `!override` boundary, distinct provider/runtime identities, conservative `podman-compose` unknowns, tolerant notes, source preservation, evidence scope, stable diagnostics, and sensitive-value redaction. Canonical-rendering fixtures cover exact presentation, multi-file output, retained Compose forms and tags, profile filtering, parse-render stability, recoverable aliases, trailing empty values, redaction, default formatting compatibility, and customized semantic stability. Preservation-edit fixtures cover typed exact-span changes, scalar-style retention and fallback, atomic failures, byte-identical unrelated syntax, reparsing, and redaction.

The built-in compatibility rules are unit/integration evidence, not a substitute for the runtime
conformance tier. Phase 5 expands the exact Docker Compose, `podman-compose`, Docker Engine, and
Podman matrices. A runtime observation may be promoted into a built-in rule only with an exact
command, provider version, runtime version, platform assumptions, and retained result.

## Regression rule

Every bug fix adds the smallest fixture that failed before the fix. When an external implementation changes, preserve old-version expectations if ComposeLens still claims compatibility with that version.

## Canonical commands

The crate uses Rust 2024 with an MSRV of 1.85.0. `rust-toolchain.toml` pins the normal development toolchain; the explicit MSRV command prevents that pin from hiding accidental use of newer language or library features.

```shell
cargo fmt --all -- --check
cargo ci-check
cargo ci-policy
cargo ci-clippy
cargo ci-test
cargo ci-doctest
RUSTDOCFLAGS="-D warnings" cargo ci-doc
cargo +1.85.0 ci-check
cargo +1.85.0 ci-policy
cargo deny check
```

The `ci-*` aliases use `--locked`, all workspace features, and all targets where the Cargo command supports them. CI also runs markdownlint and lychee over the documentation. Add exact conformance, property, and fuzz commands here before those harnesses become required checks.
