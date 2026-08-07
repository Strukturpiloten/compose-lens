# Real-world fixture corpus

The real-world suite checks ComposeLens against licensed projects that were created for actual deployment needs. It complements minimal authored fixtures: small cases isolate one rule, while real projects expose interactions among syntax, typed fields, implementation extensions, processing, diagnostics, and rendering.

The licensed Strukturpiloten TYPO3 regression receives an in-memory overlay with annotations and
representative exact, duplicate, conflicting, and near-miss security options. It protects merge,
interpolation, classification, and project-view access without modifying the upstream fixture or
invoking a provider, filesystem, or runtime.

## Admission policy

A real-world fixture must:

- identify an immutable upstream URL and revision;
- name the upstream license and include required license or notice files;
- permit redistribution in the repository;
- record every generation or sanitization change;
- contain no deployment credentials, personal paths, or unreviewed runtime data;
- state the behavior protected by executable tests; and
- remain small enough to review and diagnose when it fails.

The fixture license applies to the imported material; it does not replace ComposeLens's MPL-2.0 project license. Generated or modified external files retain their upstream licensing obligations. A fixture with unclear rights is represented by a new minimal MPL-2.0 reproduction or a retrieval procedure, not copied into the repository.

## Generation and secrets

When an upstream repository contains a template rather than final Compose YAML, test the generated document and record the generator, options, source revision, original output hash, stored-file hash, and modifications in `fixture.toml`. Never commit a generated `.env` file unchanged. Create deterministic test-only inputs, replace machine paths, and classify credential-shaped interpolation values as sensitive even when their contents are obviously synthetic.

Tests must not read the process environment, contact registries, or start containers. External provider and runtime execution belongs to the separate [conformance harness](conformance.md).

## Current corpus

### `strukturpiloten-typo3-postgresql`

The first fixture is generated from [`Strukturpiloten/typo3-container`](https://github.com/Strukturpiloten/typo3-container/tree/21c00ee39aab42b8c232c3d6020aeb8e9569a13e) at revision `21c00ee39aab42b8c232c3d6020aeb8e9569a13e`. The imported material is `AGPL-3.0-only`; its complete license text is stored beside the fixture.

It protects:

- generation from a Go template into a five-service PostgreSQL deployment;
- typed Podman-oriented `userns_mode` values plus retained `pull_policy` fields;
- 15 short-form bind mounts whose `z` and `Z` semantics must not be normalized into long syntax;
- service dependencies and internal/external networks;
- image references combining a tag and SHA-256 digest;
- deterministic caller-owned interpolation with sensitive-value redaction; and
- byte-preserving source rendering plus stable canonical parse-render-parse behavior.

The manifest contains the normalized generator command and both Compose-file hashes. The stored file differs from the generator output only by one added final newline. `environment.env` contains deterministic relative paths and fixture-only credentials; it is not a deployable environment file.

### `docker-awesome-compose-nginx-golang-mysql`

The independent fixture is an unmodified copy of Docker's
[`nginx-golang-mysql`](https://github.com/docker/awesome-compose/tree/30f4b7f6a6c3b0c0ecf4d4efb0de203c48d11562/nginx-golang-mysql)
sample at revision `30f4b7f6a6c3b0c0ecf4d4efb0de203c48d11562`. The imported Compose file
and upstream license are distributed under `CC0-1.0`; their exact SHA-256 hashes are recorded in
`fixture.toml`. The source bytes are not modified.

It protects a distinct ecosystem shape:

- three services combining an image and two build definitions;
- health checks and dependency conditions;
- a top-level secret with a service secret grant;
- a named database volume;
- a long-form read-only bind mount;
- independently identified build subfields with nested values retained losslessly; and
- valid reference resolution plus deterministic canonical reparsing.

Tests never read the referenced secret, enter either build context, contact a registry, or start a
provider or runtime.

## Updating an imported fixture

1. Generate from a clean checkout at a full commit revision.
2. Review the source license and redistribution terms again.
3. Inspect every candidate file for secrets, personal paths, and generated identifiers.
4. Reapply only documented deterministic sanitization.
5. Update revision, command, modifications, and hashes in the manifest.
6. Run repository policy, the real-world suite, all tests, Clippy, rustdoc, and MSRV checks.
7. Explain behavioral expectation changes in the pull request; never refresh golden behavior blindly.

Corpus growth remains open-ended. Prefer projects that add a distinct behavior or ecosystem pattern instead of many near-duplicate examples.
