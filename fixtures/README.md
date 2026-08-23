# ComposeLens fixtures

Fixtures live at `fixtures/<suite>/<id>/`. Every directory contains a `fixture.toml` manifest and
lists every file that belongs to the case. Executable integration-test entry points are indexed in
[`tests/README.md`](../tests/README.md).

## Manifest contract

Schema version 1 provides common provenance and expectation fields while allowing suite-specific
data under `extensions`:

```toml
schema = 1
id = "minimal-service"
suite = "syntax"
description = "Protects parsing of a minimal Compose service."
secrets_reviewed = true
files = ["compose.yaml"]

[provenance]
source = "authored"
license = "MPL-2.0"
redistribution = "allowed"
modifications = "none"

[environment]
description = "No interpolation environment is provided."

[expectations]
summary = "The service and image remain present with source locations."
```

IDs and suite names use lowercase ASCII letters, digits, and hyphens. The ID matches its directory;
the suite matches its parent. Allowed suites are `syntax`, `typed-model`, `processing`, `roundtrip`,
`conformance`, and `real-world`.

`provenance.source` is `authored`, `external`, or `generated`. External fixtures require immutable
`url` and `revision` values. Generated fixtures require an `oracle` with `implementation`, exact
`version`, and `command`. License, redistribution, and modifications are always explicit.

Every file path is relative to the fixture directory. Absolute paths, parent traversal, missing
files, and duplicates are rejected. Set `secrets_reviewed = true` only after checking every listed
file for credentials and identifying runtime data.

`environment.description` records interpolation, working-directory, implementation, and runtime
assumptions. `expectations.summary` says which behavior the executable test protects. Detailed
expected output or diagnostics belong in suite-specific fields or `extensions`.

## External and real-world material

External material must have clear redistribution permission and retain its upstream license or
notice. Its license does not replace ComposeLens's MPL-2.0 license. If redistribution is forbidden or
unclear, store a minimal MPL-2.0 reproduction or a retrieval/generation procedure instead.

A real-world fixture must:

- identify an immutable upstream URL and revision;
- document licensing, redistribution, generation, and sanitization;
- contain no deployment credentials, personal paths, or unreviewed runtime output;
- protect a distinct behavior through executable assertions; and
- remain small enough to review when it fails.

Generated examples record the generator, options, source revision, original and stored hashes, and
every modification. Never copy a deployment `.env` file unchanged. Use deterministic test-only
values, and keep credential-shaped values sensitive even when synthetic.

Tests do not read the process environment, referenced secrets or build contexts, contact registries,
or start providers and runtimes. External execution belongs under
[`conformance/`](../conformance/README.md).

## Current real-world corpus

### `strukturpiloten-typo3-postgresql`

Generated from
[`Strukturpiloten/typo3-container`](https://github.com/Strukturpiloten/typo3-container/tree/21c00ee39aab42b8c232c3d6020aeb8e9569a13e)
at revision `21c00ee39aab42b8c232c3d6020aeb8e9569a13e`. Imported material is
`AGPL-3.0-only`; its license is stored with the fixture.

The five-service project protects Podman-oriented values, short SELinux mounts, dependencies,
internal and external networks, tag-plus-digest images, caller-owned interpolation, sensitive-value
redaction, source preservation, and canonical reparsing. The manifest records generator inputs,
hashes, and the one final-newline modification.

### `docker-awesome-compose-nginx-golang-mysql`

An unmodified copy of Docker Awesome Compose's
[`nginx-golang-mysql`](https://github.com/docker/awesome-compose/tree/30f4b7f6a6c3b0c0ecf4d4efb0de203c48d11562/nginx-golang-mysql)
sample at revision `30f4b7f6a6c3b0c0ecf4d4efb0de203c48d11562`. The imported Compose file
and license are `CC0-1.0`; exact hashes are in the manifest.

It protects a distinct project shape with images, builds, health dependencies, a secret grant, a
named volume, a long read-only bind mount, reference validation, and canonical reparsing.

## Updating an imported fixture

1. Generate from a clean checkout at a full commit revision.
2. Recheck license and redistribution terms.
3. Inspect candidate files for secrets, personal paths, and generated identifiers.
4. Reapply only documented deterministic sanitization.
5. Update revision, command, modifications, and hashes.
6. Run repository policy, the owning suite, and the complete repository gate.
7. Explain expectation changes; never refresh a golden file without review.

Prefer a fixture that adds a distinct interaction over many near-duplicate projects.
