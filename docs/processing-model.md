# Processing model

## Principle

A Compose file is not the same thing as a fully resolved Compose project. ComposeLens represents the intermediate stages so callers retain control over environment access, merging, profile selection, defaults, and normalization.

## Core representations

### Syntax document

Represents what was written, including source spans and unrecognized constructs. It is the basis for preservation-oriented editing and precise diagnostics.

### Typed document

Represents one Compose document with native Compose types. Values may still contain interpolation expressions, relative paths, implementation extensions, and unresolved references.

### Loaded project

Represents an ordered collection of documents plus their origins and external sources. It does not imply that every optional transformation has run.

### Semantic view

Represents a chosen interpretation of the project for a particular implementation profile and processing context. It records which operations and defaults were applied.

## Explicit operations

### Load

Reads named documents and optional environment sources through caller-provided interfaces. Paths retain the document origin needed for later resolution.

### Interpolate

Evaluates supported variable expressions using an explicit provider. The provider may expose process environment variables, a supplied map, an `.env` document, or a test fixture. No provider is consulted during parsing.

The result retains a distinction between the original expression and the resolved value. Sensitive values are redacted from diagnostics.

### Merge

Combines documents according to an explicit Compose implementation profile. Merge results retain provenance for replaced, appended, reset, or removed values.

### Select profiles

Activates services using an explicit set of profile names. ComposeLens does not invent active profiles. Services without profiles follow the selected implementation profile's normal rules.

### Resolve references and paths

Resolves internal references and optionally path origins without requiring conversion to an absolute host path. Raw, normalized, and resolved forms must not be conflated.

### Validate

Produces structured diagnostics for syntax, native model constraints, cross-references, and implementation compatibility. Validation does not delete unsupported content.

### Render

Renders either a preservation-oriented syntax document or a deterministic canonical document. Rendering does not implicitly load, interpolate, merge, or validate.

## Unknown and implementation-specific data

- `x-*` extensions receive first-class preservation.
- Unknown fields are retained with source locations.
- Validation profiles decide whether unknown fields are allowed, warned about, or rejected.
- Podman-specific fields remain native data; BoxFerry decides whether a target can represent them.

## Image references

ComposeLens preserves the written reference and may expose a tolerant parsed view. It must support implementation behavior such as references combining a tag and digest when the selected implementation accepts them. Normalization must never erase the original value.
