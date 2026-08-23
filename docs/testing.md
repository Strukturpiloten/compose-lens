# Testing strategy

ComposeLens tests protect observable behavior: source fidelity, Compose processing, diagnostics,
redaction, deterministic output, and evidence boundaries. Coverage percentages are regression
alarms, not substitutes for meaningful assertions.

## Test layers

| Layer                 | Protects                                                                           |
| --------------------- | ---------------------------------------------------------------------------------- |
| Unit tests            | Local parsing, classification, validation, and rendering rules                     |
| Syntax and round-trip | Authored bytes, spans, recovery, and deterministic reparsing                       |
| Typed model           | Source-aware Compose forms and malformed partial results                           |
| Processing            | Loading, interpolation, merge, includes, profiles, paths, references, and defaults |
| Generated output      | Exact bytes, invalid input rejection, sensitivity, and parse-back                  |
| Preservation editing  | Atomic span edits and byte-identical unrelated source                              |
| Compatibility         | Exact implementation versions, evidence scope, and findings                        |
| Real-world corpus     | Interactions from licensed deployment examples                                     |
| Public API            | External-consumer compilation and supported behavior                               |
| Repository policy     | Supply chain, fixtures, documentation, schema, and workflow invariants             |

Executable integration-test entry points and their focused responsibilities are indexed in
[`tests/README.md`](../tests/README.md). Do not copy every case into narrative documentation; test
names, fixtures, and assertions are the detailed source of truth.

## Expectations for changes

A behavior change normally adds one successful case and one relevant rejection or recovery case.
Also cover the boundary affected by the change:

- merge-sensitive behavior includes replacement, reset, override, and provenance;
- public API changes include an external-consumer test;
- generated output includes exact bytes and typed parse-back;
- sensitive values include redacted `Debug` and diagnostic assertions;
- parser changes include malformed recovery and byte preservation; and
- compatibility claims include version-scoped evidence or remain explicitly unknown.

Use the smallest fixture that isolates a rule. Add a real-world fixture only when it protects a
distinct interaction or known regression. Empty test binaries and coverage-only assertions are not
useful evidence.

## Deterministic pull-request checks

Run the complete repository gate from the Dev Container:

```console
./scripts/check-all.sh
```

It formats owned files and runs repository policy, all Rust targets, Clippy, tests, doctests,
Rustdoc, conformance contracts, real-world cases, public API checks, package verification, coverage
ratchets, MSRV checks, dependency policy, offline links, and SemVer analysis. Any edit after a
successful complete run requires another run.

Useful focused commands are:

```console
cargo ci-policy
cargo ci-test
cargo test --locked --test typed_model
cargo test --locked --test processing
cargo test --locked --test generated_rendering
cargo test --locked --test public_api
cargo test --locked --test real_world
./scripts/check-files.sh --check
```

The full gate is the release and pull-request authority even when focused checks pass.

## Fixtures

Every fixture directory has a `fixture.toml` manifest describing its suite, provenance, files,
license, redistribution, modifications, secret review, environment assumptions, and expected
behavior. Relative paths must remain inside the fixture directory. External material needs an
immutable revision and redistribution permission; sensitive or identifying data must not be
committed.

The complete contract and real-world admission policy are in
[`fixtures/README.md`](../fixtures/README.md). Tests never read the process environment, access
referenced paths, contact registries, or start a runtime merely because a fixture mentions them.

## Provider and runtime evidence

Ordinary Cargo tests do not execute external Compose providers or runtimes. Matrices, capture
commands, isolation requirements, reviewed records, and the distinction between planned and
observed runs live in [`conformance/README.md`](../conformance/README.md).

External evidence capture is explicit and ignored by default. A generated result becomes evidence
only after its versions, commands, checksums, environment, output, fixture identity, and absence of
sensitive data are reviewed and committed.

## Documentation and repository policy

Repository tests bound maintained narrative pages, keep the public website inventory stable, reject
obsolete duplicate ledgers, and verify the current release-line wording. Markdown formatting and
links are checked locally; scheduled automation performs networked external-link validation.

Authored YAML parser fixtures are excluded from generic formatting because malformed and byte-exact
input is part of their contract. Metadata and repository-owned complete YAML remain syntax-checked.
