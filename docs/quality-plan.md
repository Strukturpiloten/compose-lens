# Quality plan

ComposeLens has native document and merged-project types for every closed key in the current
Compose schema. That completes key recognition, but not every value semantic, generation path, or
provider-compatibility claim.

This plan aims for dependable software that a small project can maintain. The detailed field
ledger remains in the [roadmap](roadmap.md); this document sets priorities and the quality bar.

## Investment boundary

ComposeLens will use:

- deterministic pull-request checks that finish in a practical amount of time;
- focused positive and negative tests for each changed behavior;
- malformed-input, merge, reset, provenance, redaction, and public-API tests where relevant;
- representative end-to-end and licensed real-world fixtures; and
- opt-in or scheduled provider checks when behavior cannot be proven by pure Rust tests.

ComposeLens will not require:

- a fuzzing program;
- 100% code coverage;
- every provider, version, operating system, or distribution combination;
- a large performance or benchmark farm; or
- enterprise-scale release governance.

Coverage floors remain regression alarms, not a goal to execute every line. A useful negative test
is more valuable than extra coverage without a behavioral assertion.

## Planned work

### 1. Prevent specification drift

Maintain a machine-readable inventory of closed Compose keys and fail policy checks when the
official schema changes without a classification. New keys must be marked typed, preserved-only,
or intentionally unsupported before coverage claims are updated.

The committed snapshot is pinned to an upstream commit and verified offline by repository policy
tests. A scheduled/manual-only drift workflow compares it with upstream `main`; ordinary pull
request and MSRV policy checks never fetch the schema.

To update the snapshot deliberately:

1. Run `bash scripts/check-specification-drift.sh` and review its digest plus added/removed key
   output.
2. Replace `schema/compose-spec.json` with the reviewed official upstream file, then update the
   commit, blob, SHA-256, and exact root/service classifications in
   `schema/compose-key-inventory.json`.
3. Add or update typed-model, preservation, or intentional-unsupported evidence and focused
   policy tests. Keep nested-schema drift outside this phase unless separately scoped.
4. Run the offline repository policy test and normal file checks before submitting the update.

### 2. Complete high-value semantics and diagnostics

Prioritize behavior that affects real consumers:

- keep the implemented caller-authorized `include` traversal and opt-in local-wins composition
  and caller-owned project-directory planning slices covered while leaving field-specific resource
  path resolution/rebasing, non-local context policy, environment-file/.env precedence,
  project-name rules, composed rendering, and provider evidence as separate future work;
- the remaining bounded deploy reservation-device semantics when evidence justifies them;
- provider-specific spelling differences without enforcing one provider globally; and
- stable source-aware diagnostics for invalid, deferred, deprecated, or unsupported values.

Parsing must remain free of implicit file, environment, provider, and runtime access.

### 3. Expand generated Compose output

Add construction APIs in BoxFerry demand order. Each generated field needs deterministic output,
invalid-value rejection, sensitivity handling, and parse-render-parse coverage. Do not add a field
only to increase a generated-key count.

### 4. Add focused provider conformance

Use Docker Compose and Podman Compose only for behavior where the specification and pure tests are
insufficient. Pull requests should validate repository-owned deterministic fixtures. Networked or
provider-executing evidence capture remains explicit, pinned, and opt-in or scheduled.

### 5. Grow the real-world corpus selectively

Add a licensed, immutable fixture when it exposes a missing behavior or prevents a known
regression. Do not collect projects merely to increase corpus size.

### 6. Exercise the public API through BoxFerry

Use BoxFerry as the main downstream contract test. Promote source-aware APIs when conversion needs
them, remove redundant pre-1.0 APIs instead of maintaining compatibility aliases, and keep input
format compatibility separate from Rust API compatibility.

### 7. Stabilize for 1.0

Consider 1.0 when:

- schema drift is automatically detected;
- supported parse, merge, interpolation, validation, and generation boundaries are documented;
- diagnostics and redaction are stable enough for downstream use;
- representative provider and real-world evidence covers the supported claims; and
- BoxFerry no longer needs internal workarounds for normal Compose input and output.

## Test requirement for changes

Every behavior change should normally add one successful case and one relevant rejection or
recovery case. Merge-sensitive changes also cover override and reset behavior. Public API changes
add an external-consumer test, and generation changes add exact output plus parse-back coverage.
Exceptions should be explained in the change rather than hidden behind a coverage number.
