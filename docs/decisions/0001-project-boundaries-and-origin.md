# ADR 0001: Project boundaries and from-scratch origin

- Status: accepted
- Date: 2026-07-31

## Context

BoxFerry needs a Compose implementation that handles real-world files, implementation extensions, interpolation, profiles, multi-file projects, precise diagnostics, and preservation. Existing Rust models do not provide the required architecture and behavior.

The library may also be useful to editors, validators, migration tools, and other container tooling independent of BoxFerry.

## Decision

ComposeLens is an independent repository and public Rust library. It owns native Compose processing and has no dependency on BoxFerry.

ComposeLens is implemented from scratch. It is not a fork of `compose_spec_rs`, and source code will not be copied or mechanically translated from that project or other Compose implementations.

Specifications, public documentation, and observable behavior from identified implementation versions may be used as research and differential-test evidence.

## Consequences

- The representation can preserve unresolved and implementation-specific data from the beginning.
- BoxFerry consumes a stable native boundary instead of embedding Compose parsing.
- The project must build its own comprehensive conformance and real-world fixture suite.
- Behavioral compatibility claims require versioned evidence.
- Development begins more slowly than wrapping an existing typed model.

## Alternatives considered

### Fork `compose_spec_rs`

Rejected because restructuring its model and compatibility assumptions would be comparable to a new implementation while retaining unwanted constraints.

### Keep Compose parsing inside BoxFerry

Rejected because it would couple a reusable, independently complex format implementation to one conversion application.
