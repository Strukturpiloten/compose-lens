# ADR 0004: explicit non-destructive processing overlays

- Status: accepted
- Date: 2026-07-31

## Context

A Compose document is interpolated, merged, profile-selected, and resolved before a runtime uses
it. Hiding those operations behind parsing would make results depend on ambient state and would
erase evidence needed for diagnostics and migration decisions.

Interpolation is the first Phase 3 operation. Compose applies it to eligible YAML values on each
file before merge. Variable values can come from several sources with implementation-specific
precedence, and some values are sensitive. A resolved string alone cannot explain which source was
used or safely report why processing failed.

## Decision

Phase 3 operations produce explicit, non-destructive processing results.

1. Interpolation receives an `EnvironmentProvider`; it never reads the process environment or a
   file implicitly.
2. Provider precedence is constructed by the loader or caller. The interpolation kernel does not
   invent shell, `.env`, or command-line precedence.
3. An interpolation result retains the original scalar, recovered resolved value, source span,
   substitution provenance, sensitivity, and structured diagnostics.
4. Default behavior follows documented Compose direct-substitution behavior: an unset direct
   variable emits a warning and becomes an empty string. Callers may instead preserve it or treat
   it as an error.
5. Required-variable failures and invalid expressions retain the original expression in the
   recovered result. Diagnostics never echo required-expression operands or resolved values.
6. Nested interpolation has an explicit safety limit.
7. Document processing applies interpolation only to eligible YAML values and produces one overlay
   per file before merge. Keys remain uninterpolated; fields that support arbitrary keys use their documented
   equal-sign list form when key interpolation is needed.
8. Later merge, profile, path, reference, default, and validation stages follow the same overlay
   principle: retain inputs and attach decisions instead of destructively rewriting evidence.

## Consequences

- Tests can use deterministic map or empty providers without mutating process state.
- A caller can deliberately expose process variables through its own provider implementation.
- Secret-bearing resolved values remain available to the authorized caller while diagnostics stay
  redacted by construction.
- The loader must own environment-source precedence, file ordering, and the merge of per-file overlays.
- Processing types require more provenance than a normalized object graph, but BoxFerry can explain
  exact and approximate conversions.

## Alternatives considered

### Read the process environment during parsing

Rejected because the same source would parse differently across machines and parsing could expose
ambient secrets.

### Resolve strings in place

Rejected because it destroys the original expression and prevents later diagnostics from
explaining how a value was derived.

### Put environment precedence in the interpolation kernel

Rejected because source discovery and precedence depend on loader inputs and implementation
profiles. The interpolation evaluator only needs a resolved provider contract.
