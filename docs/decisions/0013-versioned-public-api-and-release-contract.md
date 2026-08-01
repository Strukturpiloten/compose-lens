# ADR 0013: Versioned public API and release contract

- Status: accepted
- Date: 2026-07-31

## Context

ComposeLens now exposes every stage needed by an early BoxFerry consumer: loss-aware parsing,
native types, explicit loading and interpolation, provenance-preserving merge, post-merge views,
compatibility validation, canonical rendering, and preservation-oriented scalar edits. Calling the
crate merely “unstable” would give consumers no useful boundary, while promising 1.0 compatibility
would freeze interfaces before independent integration feedback exists.

The crate also needs to remain independently maintainable. Exposing parser-library types or adding
a single convenience API that performs implicit I/O would make later parser replacement and safe
application integration significantly harder.

## Decision

ComposeLens will publish one library crate with a supported pre-1.0 `0.1.x` API line.

- Patch releases preserve the supported module paths and source compatibility.
- Intentional public breaks require the next 0.x minor version, an ADR when architectural, and
  release-note migration guidance.
- Public interfaces use ComposeLens-owned types. Parser dependencies remain private.
- Processing stages remain explicit and side-effect boundaries remain part of the contract.
- Diagnostic code strings and canonical-v1 default rendering are versioned behavioral contracts.
- Future-growth compatibility enums are non-exhaustive before the first release.
- A consumer-facing integration test compiles and executes the supported end-to-end library path.
- Release archives include source, tests, fixtures, retained conformance evidence, and project
  documentation so claims can be audited from the published artifact.
- CI builds rustdoc with warnings denied and verifies the crates.io package before publication.

The exact guarantees and exclusions live in the versioned
[API stability policy](../api-stability.md).

## Consequences

BoxFerry can integrate against a named contract without depending on internal representations.
Patch releases have a meaningful compatibility promise, while the project retains room to improve
the design in 0.2 after recording migration costs. Published archives are larger than a minimal
source-only crate because they carry the evidence behind compatibility claims.

New variants can be added to the selected non-exhaustive compatibility enums without forcing an
otherwise unnecessary release break. Existing public enums not marked non-exhaustive still require
a breaking release to change exhaustiveness.

## Alternatives considered

### Keep every public API experimental until 1.0

Rejected because it provides no dependable integration surface and delays the feedback needed to
reach 1.0.

### Promise 1.0-level stability immediately

Rejected because BoxFerry and independent consumers have not yet exercised enough real projects to
justify a permanent interface.

### Publish separate crates for each layer

Rejected for now because the layers have architectural boundaries but not independent release
needs. Additional crates would add version coordination without reducing the current dependency
surface.
