# Testing strategy

ComposeLens must be built around tests because YAML syntax, Compose processing, and implementation behavior contain many interacting edge cases.

## Test layers

### Syntax tests

Cover scalars, mappings, sequences, anchors, aliases, comments where supported, duplicate keys, malformed YAML, Unicode, line endings, spans, and error recovery.

### Typed-model tests

Cover every supported field in short and long syntax, unknown fields, extensions, image references, ports, volumes, environment values, commands, health checks, build definitions, and discriminated unions.

### Processing tests

Cover file ordering, merge rules, reset/override behavior, interpolation operators, `.env` handling, profile selection, include behavior, path origins, defaults, and reference resolution.

### Round-trip and property tests

Verify that parsing never panics, preservation edits retain unrelated syntax, canonical output is deterministic, and supported typed values survive parse-render-parse cycles.

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

## Regression rule

Every bug fix adds the smallest fixture that failed before the fix. When an external implementation changes, preserve old-version expectations if ComposeLens still claims compatibility with that version.

## Canonical commands

The crate uses Rust 2024 with an MSRV of 1.85.0. `rust-toolchain.toml` pins the normal development toolchain; the explicit MSRV command prevents that pin from hiding accidental use of newer language or library features.

```shell
cargo fmt --all -- --check
cargo ci-check
cargo ci-clippy
cargo ci-test
cargo ci-doctest
RUSTDOCFLAGS="-D warnings" cargo ci-doc
cargo +1.85.0 ci-check
cargo deny check
```

The `ci-*` aliases use `--locked`, all workspace features, and all targets where the Cargo command supports them. CI also runs markdownlint and lychee over the documentation. Add exact conformance, property, and fuzz commands here before those harnesses become required checks.
