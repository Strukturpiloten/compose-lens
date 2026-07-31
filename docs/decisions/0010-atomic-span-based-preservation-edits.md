# ADR 0010: atomic span-based preservation edits

- Status: accepted
- Date: 2026-07-31

## Context

ComposeLens must support focused changes to an authored document without reserializing unrelated
syntax. Rebuilding a document from the typed or merged model would discard comments, whitespace,
anchors, duplicate-key evidence, unknown fields, and authored scalar styles. Exposing mutable
`yaml-edit` nodes would make the parser dependency part of the public API and undermine ADR 0002.

The typed model already exposes exact `SourceSpan` values for supported scalar fields. Those spans
provide an unambiguous bridge back to the original syntax document, including when mapping keys are
duplicated or the same semantic value occurs more than once.

Edits can contain secrets, and multiple edits can conflict. A partial batch would be difficult for
callers to reason about and could leave a document in an unintended intermediate state.

## Decision

1. The initial preservation-edit API targets exact YAML value-scalar spans. Mapping keys,
   collections, aliases, empty values, block scalars, and multiline scalars are not accepted by
   this operation.
2. Public edit types are owned by ComposeLens. `ScalarEdit` combines a `SourceSpan` with a typed
   `ReplacementScalar`; no `yaml-edit` type is exposed.
3. Replacements support strings, sensitive strings, booleans, numeric spelling, and explicit null.
   Numeric text must parse as exactly one complete YAML integer or floating-point scalar.
4. String replacements retain double-quoted style, or single-quoted style when the value is safe
   for it. Plain style is retained only when the private parser confirms that the complete
   replacement is one YAML string scalar. Otherwise a deterministic double-quoted scalar is used.
5. The operation validates every source identity, exact scalar span, style, replacement, and range
   overlap before changing text. Any error rejects the entire batch and returns the original source
   bytes.
6. A valid batch is applied to the original source in descending byte-offset order. Every byte
   outside the selected scalar spans remains unchanged, and the `SyntaxDocument` itself remains
   immutable.
7. Parsing, interpolation, typed extraction, merging, validation, file access, environment access,
   and runtime invocation are never implicit parts of editing.
8. Successfully edited sensitive output is available only through explicit accessors. Replacement
   values are redacted from `ReplacementScalar`, `ScalarEdit`, and successful result `Debug` output.
   Diagnostics never include replacement content.

## Consequences

- A caller can use spans from the typed model to update images, commands, environment values, and
  other scalar fields while preserving unrelated authored text byte-for-byte.
- Exact spans avoid path ambiguity and make source-document mismatches diagnosable.
- The first API deliberately cannot insert or remove fields, edit mapping keys, transform
  collections, or safely rewrite block scalars. Those operations need syntax-specific contracts
  and tests before they are added.
- String style can change from plain or single-quoted to double-quoted when retaining the authored
  style would change YAML type or structure.
- Byte-range application avoids depending on mutable parser wrappers, but the private parser still
  identifies eligible value scalars and validates ambiguous replacement spelling.

## Alternatives considered

### Expose mutable parser nodes

Rejected because it would leak a pre-1.0 dependency into the public API and make safe replacement
or supplementation of the YAML parser much harder.

### Accept arbitrary YAML fragments

Rejected because a scalar edit could inject mappings, sequences, comments, tags, or aliases and
silently change document structure.

### Address fields only by semantic paths

Rejected as the foundational API because duplicate keys, sequences, and repeated values make paths
ambiguous. Higher-level helpers may resolve a path to an exact span before creating an edit.

### Apply every valid edit from a partially invalid batch

Rejected because callers could mistake a partial document for the requested final state. Atomic
failure is deterministic and retryable.
