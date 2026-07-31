# ADR 0006: provenance-preserving Compose merge view

- Status: accepted
- Date: 2026-07-31

## Context

Compose merge is not a generic YAML overlay. Mappings recurse, most sequences append, shell-command
fields replace, resource sequences have field-specific uniqueness keys, and Compose tags can reset
or fully override a value. Environment and label values can use mapping or list syntax across
different files. YAML aliases and merge keys also act within each source document before the
Compose files are combined.

A normalized deserialized object would produce a final value but lose which file supplied it, why
an earlier value disappeared, and whether a short or long form carried behavior important to a
conversion. Exposing the private YAML concrete-syntax-tree dependency would instead make the public
API unstable.

## Decision

1. ComposeLens exposes a parser-independent `MergedValue` tree with ordered mappings, sequences,
   scalars, null forms, unresolved aliases, and retained non-Compose tags.
2. Every value has `MergeProvenance`: contributing source spans in processing order and an explicit
   operation of authored, added, replaced, merged, appended, reset, or override.
3. `merge_project` accepts a `LoadedProject` and an optional matching `ProjectInterpolation`.
   Supplying no overlay deliberately merges uninterpolated source values; a mismatched overlay is an
   error.
4. Mappings merge recursively and ordinary sequences append. `command`, `entrypoint`, and
   `healthcheck.test` replace.
5. Service volumes, devices, configs, secrets, and ports merge as unique sequences using their
   documented semantic keys. Matching long mappings recurse; incompatible short and long items
   retain the later authored form and all source evidence.
6. Environment and label map/list forms become a semantic keyed mapping during merge. Every entry
   retains whether its effective source used mapping, `KEY=VALUE` list, or key-only list syntax.
7. `!reset` creates the appropriate empty/default form and records a reset. `!override` bypasses
   ordinary append and uniqueness rules and records an override. Other YAML tags remain explicit.
8. YAML aliases and `<<` merge keys resolve only within their source document. Unresolved aliases
   remain visible and produce a warning rather than disappearing.
9. Merged scalar debug output redacts values marked sensitive by interpolation. Authorized callers
   can still access the semantic value explicitly.

## Consequences

- BoxFerry can explain why a conversion result differs from either individual source file.
- Later canonical rendering can choose a syntax form using retained entry and value evidence.
- The semantic merge tree does not replace the immutable syntax or native typed document layers.
- Environment and label container syntax is normalized for key-based lookup, but each effective
  entry retains its authored form.
- Compatibility profiles must still decide whether a tag or behavior is supported by a selected
  Docker Compose or Podman Compose version.
- Runtime conformance matrices remain necessary even though the specification rules have executable
  authored coverage.

## Alternatives considered

### Recursively merge YAML nodes and render the result

Rejected because it applies the wrong behavior to command and unique-resource sequences and loses
the reason for each replacement.

### Merge only the Phase 2 typed subset

Rejected because unknown and implementation-specific fields would disappear even though they can
be valid and important to real projects.

### Normalize every short form to its long form before merge

Rejected because authored syntax forms can differ in defaults and implementation behavior, notably
for volume and SELinux options.
