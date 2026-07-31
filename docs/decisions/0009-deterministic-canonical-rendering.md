# ADR 0009: deterministic canonical rendering

- Status: accepted
- Date: 2026-07-31

## Context

ComposeLens needs a stable machine-generated representation for golden tests, downstream tooling,
and explicit export of a processed project. That output has a different purpose from
preservation-oriented editing: comments and authored scalar style are not expected to survive,
but the semantic value and retained Compose syntax form must survive.

Docker Compose also calls the output of `docker compose config` canonical. That command parses,
resolves, merges, and renders the model, and normally expands short notation. ComposeLens keeps
those processing stages explicit and preserves short and long forms because they are not always
behaviorally interchangeable. Reusing the same word must not imply byte or normalization parity.

Canonical output can contain values supplied by an interpolation provider. It therefore needs the
same sensitivity boundary as the merge model and diagnostics.

## Decision

1. `render_canonical` consumes a `MergedProject` and an optional matching `ProfileSelection`. It
   performs no loading, interpolation, merging, path resolution, defaulting, compatibility
   validation, provider invocation, or runtime invocation.
2. The canonical-v1 presentation is UTF-8 YAML with LF line endings, a final newline, two-space
   indentation, no document marker, insertion order retained for mappings and sequences, and
   JSON-compatible double quotes for every mapping key and string scalar.
3. YAML booleans render as lowercase `true` or `false`; null-like values render as `null`; numeric
   semantic spelling remains unchanged; empty mappings and sequences render inline as `{}` and
   `[]`.
4. Canonical rendering retains the effective short or long Compose form present in the merged
   model. It does not claim parity with `docker compose config`, which may normalize forms and
   resolve more context.
5. A matching profile selection omits inactive services but does not remove top-level resources.
   A selection from another merged project is rejected with the shared project-mismatch
   diagnostic and no output.
6. Retained non-Compose YAML tags are emitted when they are safe YAML tag tokens. An unsafe token
   produces an error and is dropped so the remaining value can still be rendered. An unresolved
   alias produces an error and a `null` recovery value because a standalone alias has no canonical
   target.
7. The result exposes rendered text only through explicit accessors. When any rendered value is
   sensitive, its `Debug` representation redacts the entire output. Rendering diagnostics never
   contain semantic values.
8. Canonical output must parse successfully after recoverable rendering errors, and a
   parse-merge-render cycle over valid canonical output must reproduce the same bytes.

## Consequences

- Golden files can compare exact, platform-independent bytes without depending on a Docker or
  Podman installation.
- Callers choose the processing stages whose results they want rendered; there are no hidden
  environment or filesystem reads.
- Canonical output is intentionally noisier than hand-written Compose because all keys and strings
  are quoted.
- Comments, anchors, aliases, authored quoting, and original whitespace require the later
  preservation-editing path rather than canonical rendering.
- A future configurable presentation API must preserve canonical-v1 behavior and cannot silently
  change its semantic processing boundary.

## Alternatives considered

### Match `docker compose config` output

Rejected because that command performs processing and normalization that ComposeLens exposes as
separate choices. Its output also changes with the selected Docker Compose release.

### Serialize the typed model with a generic YAML serializer

Rejected because a generic serializer would not enforce ComposeLens ordering, sensitivity,
profile-selection, tag-recovery, and retained-form contracts.

### Preserve authored formatting in every render

Rejected as the only rendering mode because merged projects combine multiple sources and have no
single authored concrete syntax tree. Preservation-oriented editing remains a separate Phase 4
operation.
