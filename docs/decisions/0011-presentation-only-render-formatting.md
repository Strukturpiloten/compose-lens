# ADR 0011: presentation-only render formatting

- Status: accepted
- Date: 2026-07-31

## Context

Canonical-v1 output needs fixed bytes for golden tests and reproducible tooling, while applications
may need a YAML document marker, a project-specific indentation width, CRLF output, or no final line
ending. Those are presentation choices, not Compose processing or normalization.

An unconstrained options object could accumulate semantic behavior such as interpolation, profile
selection, mapping reordering, short/long form conversion, or default application. That would hide
processing stages inside rendering and contradict the explicit pipeline established by earlier
ADRs.

Formatting must also preserve the existing default output. Adding options must not invalidate
canonical-v1 fixtures or make an unconfigured render depend on the host platform.

## Decision

1. `CanonicalFormatting` contains only indentation width, line-ending convention, document-marker
   emission, and final-line-ending emission.
2. `CanonicalFormatting::default()` is exactly canonical-v1: two spaces, LF, no document marker,
   and one final LF. `render_canonical` remains a stable shorthand for that default.
3. `render_canonical_with_formatting` accepts the formatting value separately from the merged
   project and optional `ProfileSelection`. Formatting cannot create or alter a selection.
4. `IndentWidth` rejects zero and accepts every positive `u8` width. The type prevents an invalid
   nesting width from entering `CanonicalFormatting`.
5. `LineEnding` supports explicit LF and CRLF. It never reads operating-system defaults.
6. Key and string quoting, escaping, retained mapping/sequence order, YAML tag recovery, and
   effective Compose short/long forms remain part of the renderer's correctness contract, not
   caller formatting choices.
7. Formatting is applied only after the renderer has produced its deterministic LF-based semantic
   output. It does not affect diagnostics or sensitivity classification.
8. Custom-formatted valid output must parse and merge to the same canonical-v1 bytes as the
   corresponding default render.
9. Preservation-oriented editing does not use these options. Its purpose is to retain authored
   formatting outside explicit edit spans.

## Consequences

- Existing callers and golden files retain byte-identical default output.
- Applications can choose common repository or transport conventions without forking the renderer.
- The same options produce the same bytes on every supported host platform.
- There is intentionally no “normalize,” “sort,” “expand short syntax,” or “resolve variables”
  formatting flag. Those would be semantic operations with separate inputs and diagnostics.
- Future presentation choices must demonstrate semantic neutrality and deterministic output before
  being added to `CanonicalFormatting`.

## Alternatives considered

### Use host-native line endings

Rejected because output would differ across machines without an input change.

### Put processing flags in one render-options structure

Rejected because callers could accidentally combine loading, interpolation, merging, defaults,
normalization, and output in an opaque operation.

### Make custom formatting the new default

Rejected because canonical-v1 is already documented and protected by exact golden fixtures.

### Apply formatting options to preservation editing

Rejected because global indentation or line-ending changes contradict byte preservation outside
the requested source spans.
