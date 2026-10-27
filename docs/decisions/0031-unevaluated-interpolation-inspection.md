# ADR 0031: Unevaluated native interpolation inspection

- Status: accepted
- Date: 2026-10-27
- Amends: ADR 0004
- Preserves: ADR 0028 document-interpolation presentation exclusions

## Context

An application that preserves deferred intent must distinguish literal dollars, native references,
and unsupported syntax without substituting environment values. Evaluating with an empty provider
would apply missing-variable policy, choose defaults, and erase the very evidence the application
needs. Parsing native syntax in every adapter would duplicate grammar and escaping decisions.

Authored scalar presentation also matters: native document interpolation deliberately excludes
single quotes and block scalars. A consumer must not silently treat excluded dollar syntax as an
eligible reference or claim that unchanged excluded bytes have undergone native escape processing.

## Decision

1. Add an explicit bounded inspector under `interpolation`, accepting the existing protected
   `InterpolationInput`. It never receives an environment provider, reads ambient state, evaluates
   a branch, substitutes a value, or emits missing-variable warnings because no provider exists.
2. Return ordered literal/reference/invalid segments with exact original evidence. Reuse existing
   native variable-name, braced-expression, closing-brace helpers, and `InterpolationOperator`.
   Preserve the evaluator's current scanner semantics and leave ordinary evaluation unchanged.
3. Collapse `$$` once inside literal segments. The resulting text remains typed literal data even
   when it resembles a reference. Braced and unbraced references retain distinct forms. Known
   operators retain exact operands and recursively inspected operand segments without selecting a
   branch. Malformed/unsupported braced expressions remain typed invalid evidence and emit the
   existing invalid-expression diagnostic without echoing names or operands.
4. Keep all ranges in original decoded-root UTF-8 byte coordinates, including nested operands.
   Retain the containing authored source span separately. Never infer raw YAML subspans from
   decoded offsets: quotes, escapes, Unicode, and block decoding can change byte relationships.
5. Bound decoded input to 1,048,576 bytes, total segments across all operands to 65,536, and nested
   operand depth to 32 (the root scalar has depth zero). Return typed content-free errors with
   source evidence when a limit is exceeded; never return an apparently successful partial tree.
   These fixed construction bounds do not change evaluator options or provider compatibility.
6. Provide a separate presentation-aware wrapper. Plain and double-quoted values are inspected;
   single-quoted, literal-block, and folded-block values return an explicit exclusion containing
   unchanged decoded text and a dollar-presence flag. Excluded content does not collapse escapes.
   Preserve ADR 0028; do not change document interpolation or make mapping keys eligible.
7. Propagate caller sensitivity into every nested payload and redact all related `Debug` output.
   Exact original text, names, operands, and literal values remain available only through explicit
   accessors. Bound errors store no scalar content; diagnostics contain static messages and spans.
8. Retain native merged/list evidence without reparsing YAML. The complete raw-list semantic item,
   original presentation, latest item source span, and sensitivity remain accessible even when
   field-specific normalization synthesizes a plain value suffix. Authored inspection requires a
   merge without an interpolation overlay; a current merged value may already be resolved. The
   new helper does not reverse evaluation or pretend its decoded range is a raw-source range.
9. Inspection operates only on explicitly supplied scalars. It does not traverse documents,
   automatically interpret metadata, define cross-format binding semantics, or own systemd syntax.

## Consequences

Applications can preserve native intent while using one native grammar. Typed operators and
invalid evidence support actionable decisions without relying on diagnostic prose. Existing
provider-backed conversion and native block/single-quote exclusions retain their contracts.

The generic inspector can inspect arbitrary explicit decoded text; this is deliberately different
from document eligibility. Consumers needing native style policy must use the presentation-aware
wrapper and handle exclusion explicitly. A decoded range identifies expression text within a
scalar, not an independently addressable raw YAML token.

## Alternatives considered

- Evaluating with an empty provider would choose branches, drop references, and create misleading
  missing-variable findings.
- An adapter-owned scanner would duplicate native grammar and allow escaping behavior to drift.
- Automatically inspecting every native field would conflate native evidence with application
  policy and could activate inert metadata.
- Changing block/single-quote evaluation now would contradict ADR 0028 and alter ordinary conversion.
