# Testing strategy

ComposeLens must be built around tests because YAML syntax, Compose processing, and implementation behavior contain many interacting edge cases.

## Test layers

### Syntax tests

Cover scalars, mappings, sequences, anchors, aliases, comments where supported, duplicate keys, malformed YAML, Unicode, line endings, spans, and error recovery.

### Typed-model tests

Cover every field in the documented [typed boundary](typed-model.md), all supported syntax variants,
unknown fields, extensions, image references, ports, volumes, host/container path separation,
environment values, environment-file short/long forms and options, service-label forms, commands,
extra hosts, raw identities, service-level restart policies,
ulimits, health checks, dependency conditions, field-level build/deploy identities, networks,
profiles, configs, secrets, top-level resources, and discriminated unions.

### Processing tests

Cover file ordering, merge rules, reset/override behavior, interpolation operators, `.env` handling, profile selection, include behavior, path origins, defaults, and reference resolution.

### Round-trip and property tests

Verify that parsing never panics, preservation edits retain unrelated syntax, canonical output is deterministic, and supported typed values survive parse-render-parse cycles.

The implemented canonical-rendering tier compares exact golden bytes, repeats rendering to prove
determinism, and verifies parse-merge-render stability. It also covers profile filtering, retained
tags, unresolved-alias recovery, and sensitive-output redaction. A regression fixture places an
empty environment value immediately before later service fields so parser recovery cannot silently
reparent ports, volumes, or extensions into the environment mapping.

Generated-rendering tests construct the runtime-migration subset through public Compose-owned
types, compare exact deterministic bytes, and inspect the parse-back native model. They protect
ordered duplicate-capable environment syntax, ordered environment-file short/long forms and
options, ordered unique service-label mappings, spec-shaped
long TCP/UDP ports, short SCTP ports,
ordinary mounts, deliberate short `SELinux` bind syntax, network aliases,
all service-level restart-policy forms and optional maximum retries,
application/external resource lifecycle, duplicate
rejection, label duplicate rejection, empty and embedded-equals label values, ambiguous short-form
failures, and sensitive debug redaction.

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

The first repository-side harness and exact provider-config matrix are implemented. The harness is
an ignored integration test: normal tests validate its complete matrix and authored fixture, but
external execution requires an absolute launcher, exact expected version, caller-verified artifact
URL and SHA-256 metadata, a full fixture revision, explicit platform and path inputs, and a new
result directory. It clears inherited environment variables and retains raw outputs for review.
See the [conformance guide](conformance.md) and
[ADR 0012](decisions/0012-repository-conformance-harness.md).

### Real-world fixtures

Use licensed projects and minimal reproductions of reported behavior. Every fixture needs provenance, redistribution permission, secret review, and a statement of what it protects.

The first real-world fixture is a generated PostgreSQL variant of
`Strukturpiloten/typo3-container`. It exercises five interacting services, typed Podman-specific
user namespace values, short-form SELinux mounts, dependencies, external networks, tag-plus-digest
images, caller-owned interpolation, sensitive-value redaction, reference validation, and stable
canonical rendering. An independent, byte-identical Docker Awesome Compose fixture adds build
definitions, health checks, dependency conditions, a top-level secret and grant, a named volume,
and a long read-only bind mount. The generation, licensing, sanitization, and update rules are
documented in the [real-world corpus guide](real-world-corpus.md).

## Test organization

Cargo-discovered integration tests live in [`../tests/`](../tests/README.md), with private helpers in `tests/support/`. Fixtures live in [`../fixtures/`](../fixtures/README.md) and are validated against the versioned [fixture manifest contract](fixture-format.md). Product suites are added only with implemented behavior and meaningful assertions.

The initial syntax corpus exercises comments, anchors, aliases, duplicate keys, extension fields, scalar spelling, interpolation-shaped text, tag-plus-digest image references, Unicode, CRLF input, malformed flow syntax, complete comma-containing block plain scalars, hyphenated anchor names and direct aliased block values, unquoted option-like sequence items, blank lines before indented mapping values, incomplete syntax-tree fail-safe detection, source spans, and exact parse/render/parse stability. The Phase 2 typed-model corpus covers its complete field boundary, deferred expressions, empty and null values, extensions and unknowns, partial invalid-input recovery, and stable source-spanned diagnostics. It also keeps short `:z`, short `:Z,ro`, and long `bind.selinux: Z` volume mounts distinct. The first processing corpus protects every interpolation operator, nested expressions, escaped dollars, missing-variable policies, required-value redaction, sensitivity propagation, and nesting recovery. It also covers ordered multi-file loading, explicit origin retention, first-file base-directory selection, duplicate source IDs, recoverable diagnostics, and one per-file interpolation overlay before merge. Field-aware merge fixtures cover mapping recursion, ordinary append, command replacement, mixed environment and label forms, ordered environment files, unique ports, volumes, devices, configs, and secrets, YAML merge keys, unknown fields, reset/override tags, provenance, and sensitive-value debug redaction. Native project-view fixtures cover profile filtering, direct native images, commands, environments, environment-file short/long forms, sequence and mapping extra hosts, `host-gateway`, bracketed IPv6, ports, volumes, service config/secret grants, networks, top-level resources, unmodeled-field references, mismatched selections, recoverable invalid forms, sensitive-value redaction, unique-by-target nested-field retention, and field/item/collection provenance across two source files. Post-merge fixtures cover explicit and all-profile selection, profile reset behavior, inactive-service exclusion, relative and caller-supplied home path origins, named-resource references, inactive and missing service edges, documented defaults, no-default policy behavior, and rejection of selections from another project. Compatibility fixtures cover exact version parsing and ranges, selected-service feature discovery, Docker's documented `!override` boundary, distinct provider/runtime identities, conservative `podman-compose` unknowns, tolerant notes, source preservation, evidence scope, stable diagnostics, and sensitive-value redaction. Canonical-rendering fixtures cover exact presentation, multi-file output, retained Compose forms and tags, profile filtering, parse-render stability, recoverable aliases, trailing empty values, redaction, default formatting compatibility, and customized semantic stability. Preservation-edit fixtures cover typed exact-span changes, scalar-style retention and fallback, atomic failures, byte-identical unrelated syntax, reparsing, and redaction.

The built-in compatibility rules are unit/integration evidence, not a substitute for the runtime
conformance tier. Phase 5 expands the exact Docker Compose, `podman-compose`, Docker Engine, and
Podman matrices. Planned matrix entries make no support claim. A runtime observation may be
promoted into a built-in rule only with an exact command, provider version, runtime version,
platform assumptions, and reviewed retained result.

The issue-derived regression tier adds authored fixtures for both valid and malformed
`extra_hosts`, user/group interpolation, unlimited ulimits, service-level restart policies and
maximum retries, health checks, dependency conditions,
anonymous Linux container paths, and independently identified build/deploy fields. The licensed
TYPO3 fixture demonstrates the Podman `keep-id` consumer. Compatibility tests detect
`host-gateway` and Podman user-namespace modes and require official Podman 5.4 evidence while
keeping unobserved provider pass-through classified as unknown.

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
cargo test --locked --test conformance
cargo test --locked --test runtime_conformance
cargo test --locked --test real_world
cargo test --locked --test public_api
cargo test --locked --test generated_rendering
cargo package --locked
```

The `ci-*` aliases use `--locked`, all workspace features, and all targets where the Cargo command
supports them. CI also runs markdownlint and lychee over the documentation. The ordinary
conformance command validates matrix policy and leaves its external runner ignored. The explicit
collection command is documented in [`../conformance/README.md`](../conformance/README.md). Add
property and fuzz commands here before those harnesses become required checks.
