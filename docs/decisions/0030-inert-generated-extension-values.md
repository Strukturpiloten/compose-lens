# ADR 0030: Bounded inert generated extension values

- Status: accepted
- Date: 2026-10-27
- Amends: ADR 0017, ADR 0024, and ADR 0028

## Context

Applications need to retain native `x-*` payloads without maintaining a YAML writer or assigning
cross-format semantics inside ComposeLens. Recursive payloads can contain non-string YAML kinds,
protected keys, and authored folded strings. A typed extension source reference alone cannot
safely generate these values or explain unsupported mapping keys.

## Decision

1. Add ComposeLens-owned generated extensions only at document and direct service scope. Require
   the exact lowercase `x-` prefix and a non-empty single-line suffix; preserve the rest of the
   key exactly. Reject exact duplicates without including key data in errors. Differently cased
   keys are independent. Native fields cannot be injected through the extension namespace.
2. Use private recursive storage with explicit null, boolean, validated number, string, folded
   string, ordered mapping, and ordered sequence construction. All mapping keys are generated
   strings, including empty or protected strings. Empty collections remain explicit.
3. Validate numeric spelling as one complete native YAML numeric scalar and retain those bytes.
   Never route numbers through a floating-point representation. Do not interpret application or
   provider meaning, interpolate, read environment values, or accept raw YAML fragments.
4. Enforce a maximum collection depth of 64 and a maximum of 65,536 nodes per value tree, counting
   nested mapping keys as nodes. Fallible collection constructors check stored subtree totals
   before rendering, preventing recursive rendering of an unchecked tree. These are construction
   resource bounds, not provider compatibility claims.
5. Folded strings use explicit relative indentation and native strip/clip/keep chomping. Insert
   physical blank separators where folding would otherwise change semantic line breaks. Validate
   exact decoded parse-back before accepting the value. Unrepresentable values fail explicitly;
   do not silently switch to literal or quoted style. Native parser limitations, including
   tab-indented block content, remain explicit construction failures.
6. Expose native scalar presentation and strict YAML string classification for merged scalars and
   keys. Preserve unsupported non-scalar key spelling and source spans with false strict-string
   classification instead of dropping the entry. Tags and unresolved aliases retain their existing
   distinct value kinds. Merge identity includes strict string classification, so an unsupported
   raw key and an authored string containing those bytes cannot overwrite each other. Flow maps
   bypass block indentation recovery and retain all direct entries. Quote generated `<<` string
   keys; only authored plain `<<` activates native YAML merge behavior. Expand source groups
   through native extraction rather than the backend's string-deduplicating merged view. Preserve
   inherited key categories, unsupported shapes, duplicate evidence, and source spans. Local keys
   override inherited keys; earlier source mappings override later mappings. Bound alias recursion
   through the native guard. Retain invalid/unresolved merge sources under a non-strict `<<` key
   instead of silently dropping them or applying partial inheritance. Applications own decisions
   about unsupported shapes. Guard anchored mapping declarations consumed as plain-scalar content
   with the existing unparsed-input diagnostic rather than accepting changed source structure.
7. Propagate sensitivity from all nested keys and values. Generated parse-back models retain only
   extension source references, so conservatively protect their document/service extension names
   when generated output is sensitive. Source-aware wrappers redact names independently of the
   containing object's `Debug`; explicit accessors expose values intentionally. Persisted YAML
   carries no sensitivity marker. Merged mapping sensitivity includes key taint.
8. Keep canonical-v2 rendering unchanged. Explicit folded style is an additive generated extension
   choice, not a change to ordinary string quoting or canonical authored output.

## Consequences

Applications can construct inert native metadata without depending on the private YAML backend.
Numeric spelling, ordering, key casing, folded semantics, and source evidence remain available at
native boundaries. Generation stays pure and parse-back validated through syntax and Compose
models. No dependency, provider-version claim, or cross-format model is added.

The extension API deliberately cannot generate tagged values, aliases, non-string keys, arbitrary
native field names, or nesting outside document/direct-service scope. Further construction shapes
require their own reviewed contract. Sensitivity is conservatively broader in sensitive generated
parse-back models and cannot survive serialization without a caller-owned policy.

## Alternatives considered

- Raw YAML fragments would bypass construction bounds, numeric validation, namespace checks, and
  protected debugging.
- Generic vendor semantics inside ComposeLens would cross the native library boundary.
- Quoting every extension scalar would erase null, boolean, numeric, and folded presentation.
- Silently dropping unsupported authored keys would make application diagnostics incomplete.
