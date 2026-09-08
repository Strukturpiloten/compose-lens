# ADR 0026: Independent real-application conformance contracts

- Status: accepted
- Date: 2026-09-08

## Context

Small syntax fixtures establish individual Compose rules, but they do not show that loading,
interpolation, merge tags, environment-file resolution, profiles, references, provenance, and
rendering continue to work together for an application-sized project. Using only a downstream
consumer's generated result as the expected result would create a circular test: ComposeLens and
the consumer could make the same mistake while still agreeing.

Provider configuration and runtime execution are separate claims. The library must remain pure,
must not depend on BoxFerry, and must not invoke a Compose provider, container runtime, network, or
generated command during an ordinary test or release gate. Retained external observations still
need enough identity, isolation, cleanup, and privacy evidence to be reviewable.

## Decision

ComposeLens maintains a dedicated `application_conformance` integration suite for a small set of
reviewed real applications. Each application contract:

- imports selected native Compose definitions at an immutable upstream revision, records exact
  hashes and redistribution terms, and independently states the semantic expectations exercised;
- uses only ComposeLens public native APIs and test-only helpers, with no BoxFerry dependency;
- supplies interpolation and environment-file values explicitly and asserts sensitive values stay
  redacted;
- exercises application-relevant combinations across loading, interpolation, merge, profile and
  reference processing rather than treating successful parsing as conformance; and
- validates generated YAML with exact bytes, typed parse-back, and deliberate mutation failures
  when generated output is part of the contract.

The initial contracts use bounded Nextcloud and Forgejo definitions. The Forgejo provider
application and its peer remain separate Compose projects joined by an explicitly named external
network; tests must not merge them into one project merely for convenience.

`conformance/application-evidence.toml` is the reviewed evidence index for these contracts. It
records an exact provider artifact and distinguishes `observed` runtime rows from `planned` rows.
Every observed row links a retained record; every planned row explains why no observation is
claimed. Provider records state that no runtime was invoked. Runtime records identify the external
workflow and source revision and require successful cleanup, resource audit, and privacy review.
Unobserved host properties remain explicit rather than inferred.

`cargo ci-application` runs the deterministic offline suite. The local complete gate, pull-request
CI, and release workflow invoke that alias explicitly. They validate retained records but never
capture new provider or runtime evidence.

## Consequences

- Cross-stage regressions have independently reviewable application semantics and are required in
  local, pull-request, and release validation.
- Fixture updates require an immutable revision, file and license review, refreshed hashes, and a
  review of the semantic assertions; golden output cannot be refreshed without explanation.
- External provider output can support only a provider-configuration claim. Runtime behavior can
  support only the exact recorded provider/runtime/root-mode context.
- The corpus stays intentionally small. New applications must add a distinct interaction or
  regression and remain practical for deterministic offline review.
- BoxFerry continues to consume ComposeLens through its public API; ComposeLens does not absorb
  BoxFerry's neutral model or migration policy.

## Alternatives

- **Use only unit-sized fixtures.** Rejected because they do not protect interactions across the
  complete processing pipeline.
- **Compare ComposeLens output directly with BoxFerry output.** Rejected because agreement is not
  an independent semantic oracle and would invert repository ownership.
- **Run Docker Compose or Podman in ordinary tests.** Rejected because it introduces environment,
  network, privilege, and cleanup side effects into a pure-library gate.
- **Treat a successful provider `config` command as runtime proof.** Rejected because parsing and
  normalized provider output do not establish applied runtime effects.
