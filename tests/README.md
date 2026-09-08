# ComposeLens test suites

Cargo-discovered integration-test entry points live directly in this directory. Shared test-only
helpers live in `support/` and never become public library API.

| Suite                       | Responsibility                                                                     |
| --------------------------- | ---------------------------------------------------------------------------------- |
| `repository_policy`         | Fixture, documentation, schema, workflow, and supply-chain invariants              |
| `syntax`                    | YAML spelling, spans, malformed input, and recovery                                |
| `typed_model`               | Native Compose forms and source-aware partial results                              |
| `processing`                | Loading, interpolation, merge, includes, profiles, paths, references, and defaults |
| `compatibility`             | Exact versions, evidence scope, and findings                                       |
| `roundtrip` and `rendering` | Canonical bytes, profile filtering, recovery, and stability                        |
| `generated_rendering`       | Typed construction, exact bytes, parse-back, and redaction                         |
| `preservation_editing`      | Atomic scalar changes and unrelated-byte preservation                              |
| `conformance`               | Provider matrix policy and the explicit ignored capture runner                     |
| `runtime_conformance`       | Planned runtime contexts and fail-closed isolation policy                          |
| `real_world`                | Licensed project regressions                                                       |
| `public_api`                | External-consumer contract for the current 0.3.x release line                      |
| `environment_resolution`    | Caller-authorized environment/secret values, precedence, states, redaction         |
| `application_conformance`   | Independent real-application semantics and retained evidence                       |

`application_conformance` owns independent Nextcloud and Forgejo application contracts across
loading, interpolation, merging, profiles, references, environment files, generation, retained
provider/runtime evidence, and privacy boundaries. Run it with `cargo ci-application`; it remains
offline and does not invoke BoxFerry, a provider, or a runtime.

Introduce a suite only with implemented behavior, fixtures when needed, and meaningful assertions.
Do not add an empty test binary to reserve a name. The test strategy is in
[`docs/testing.md`](../docs/testing.md).
