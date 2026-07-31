# API stability policy

ComposeLens is pre-1.0. Public APIs are available for integration experiments but are not yet covered by a stable semantic-versioning guarantee.

## Current guarantees

- Public APIs use ComposeLens-owned types. Parser-dependency types are private implementation details.
- Every intentional breaking change is documented in release notes once releases begin.
- Diagnostic codes are treated as machine-readable contracts. Renaming or changing their meaning requires fixture and documentation updates.
- Parsing does not read process environment variables or perform runtime I/O.
- Supported public APIs compile on the documented MSRV.

## Before 1.0

Types and methods may change while syntax, typed models, processing stages, and editing semantics are validated. Prefer an exact ComposeLens dependency version until a stable release policy exists.

A public API becomes a stability candidate only after it has:

- at least one independent consumer
- public documentation and examples
- regression coverage for success and failure behavior
- explicit ownership in the architecture
- no leaked dependency representation that prevents replacement

## After 1.0

The project will define normal semantic-versioning commitments, supported release lines, deprecation periods, and diagnostic-code compatibility before publishing 1.0. That policy will supersede this document through an ADR.
