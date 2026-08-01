# ComposeLens fixtures

Fixtures are stored as `fixtures/<suite>/<id>/`. Every fixture directory contains a `fixture.toml` manifest and all files listed by that manifest.

The common manifest contract is documented in [Fixture format](../docs/fixture-format.md). Executable test entry points live in [`tests/`](../tests/README.md).

Do not add credentials, unreviewed external content, or files with unclear redistribution rights.
External fixture files retain their upstream licenses; adding them does not relicense them under
ComposeLens's MPL-2.0 license. Follow the [real-world corpus policy](../docs/real-world-corpus.md)
and keep required upstream license or notice files beside imported material.

Authored `conformance` fixtures declare questions for exact external providers; they do not encode
support merely because they are valid ComposeLens inputs. Reviewed provider output belongs under
[`../conformance/records/`](../conformance/records/README.md), not in a fixture manifest.
