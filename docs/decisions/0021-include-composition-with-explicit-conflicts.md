# ADR 0021: include composition with explicit conflicts

- Status: accepted
- Date: 2026-08-14
- Extends: [ADR 0020](0020-caller-authorized-include-traversal.md)

## Context

ADR 0020 deliberately established recursive, caller-authorized include traversal without deciding
how loaded child definitions join their parent. Consumers now need an inspectable composition
result, while preserving the loader's no-I/O boundary and every local multi-file merge decision.

The Compose Specification states that included resources are copied into the including project and
that a local resource with the same name wins; it does not authorize applying ordinary multi-file
merge rules to that collision. Docker Compose-Go v2.14.0 has been observed to apply override-like
behavior instead. That provider observation contradicts the specification boundary and is retained
only as provider evidence, not as ComposeLens composition semantics.

## Decision

1. `IncludeResolution::compose` is a separate, opt-in, I/O-free pass over the already loaded,
   no-interpolation `ProjectView` occurrences. It neither changes `IncludeLoader` nor opens paths,
   reads environments, invokes providers, or discovers additional input.
2. Each child is composed recursively first. Its selected services, networks, volumes, configs,
   secrets, and individual model definitions are then imported into the parent after the parent's
   own ordinary multi-file merge.
3. Local or previously selected parent definitions win by exact namespace and name. Colliding
   included candidates are not passed to the normal Compose merge implementation. They remain
   explicit `IncludeResourceConflict` records with both occurrence indices and identities, source
   spans and labels, and the include edge through which the import was attempted.
4. Every conflict emits the stable warning `compose.include.resource-conflict` with the incoming
   candidate as its primary label and the incumbent as its secondary label. Composition diagnostics
   begin with the unchanged traversal diagnostics and append these warnings in deterministic
   depth-first include order.
5. Include nodes and successful edges expose retained occurrence indices. A cycle edge targets its
   existing active node; every successful non-cycle edge targets the retained child occurrence.
   `IncludeCompositionResult::is_complete` is false for traversal errors and for conflicts.

## Consequences

Consumers can inspect typed selected definitions and local cross-references after composition
without losing the original per-occurrence projects, graph, diagnostics, or provenance. They must
make an explicit policy decision for conflicts rather than receiving a silently merged definition.

Composition does not settle include path resolution, environment precedence, project naming,
profile reconciliation across projects, provider-specific overrides, rendering a composed document,
or a runtime/provider compatibility claim.

## Alternatives considered

### Apply the ordinary multi-file merge to same-name included definitions

Rejected because included-project collision handling is distinct from files in one project and
would silently select a definition contrary to the Compose Specification's local-wins rule.

### Compose while traversing or inside `IncludeLoader`

Rejected because it would hide the graph-processing stage, couple resource selection to I/O, and
weaken ADR 0020's authorization boundary.

### Adopt Compose-Go v2.14.0 override behavior

Rejected because it conflicts with specification evidence and would turn one provider observation
into a library-wide policy without explicit compatibility selection.
