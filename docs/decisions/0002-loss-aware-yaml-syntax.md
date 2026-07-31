# ADR 0002: Private loss-aware YAML concrete syntax tree

- Status: accepted
- Date: 2026-07-31

## Context

ComposeLens must retain comments, scalar spelling, anchors, aliases, duplicate keys, extensions, unknown fields, and source locations. It must also return useful structure and diagnostics for malformed input. A conventional YAML-to-object deserializer irreversibly discards several of those properties before Compose processing begins.

The parser is a high-risk dependency because its representation constrains diagnostics, preservation edits, concurrency, and the eventual typed model. The evaluated versions and probe results are recorded in the [YAML representation evaluation](../research/yaml-representation.md).

## Decision

Use `yaml-edit` 0.2.3, with default features disabled, as the initial private YAML concrete-syntax implementation.

ComposeLens stores the parser's immutable green-tree parse result together with the original source. Public APIs expose only ComposeLens-owned source identifiers, spans, diagnostics, and syntax documents. No dependency type appears in the public interface.

Malformed YAML produces a syntax document plus structured diagnostics when the parser can recover. Diagnostic codes and safe generic messages are owned by ComposeLens; dependency error strings are not exposed as stable API or echoed when they could contain user data.

The dependency version is exact and updates require the complete fixture suite, strict Clippy, dependency/license checks, and MSRV validation.

## Consequences

- Initial preservation rendering retains source bytes, comments, ordering, duplicate keys, anchors, aliases, and scalar spelling.
- ComposeLens can replace or supplement the parser without a public type migration.
- The stored parse representation is `Send + Sync`, verified by a compile-time integration in the unit tests, even though editable high-level `yaml-edit` wrappers use interior mutability.
- ComposeLens must build its own stable navigation and editing APIs instead of re-exporting convenient dependency wrappers.
- The parser remains syntax infrastructure, not the authority for Compose semantics or implementation compatibility.
- A pre-1.0 dependency increases maintenance risk and requires strong regression fixtures.

## Alternatives considered

### Tree-sitter with the YAML grammar

Tree-sitter provides positioned error recovery, comments, and incremental parsing. It was not selected because the grammar is not a semantic YAML validator, exact rendering still requires separately retained source, and it adds a runtime plus generated C grammar before incremental editor parsing is a demonstrated requirement.

### `saphyr-parser`

`saphyr-parser` provides a YAML 1.2-oriented event stream, anchors, aliases, and spans. It was not selected as the primary representation because semantic events omit comments and formatting trivia and cannot support preservation rendering alone. It may later serve as a differential semantic oracle.

### A new ComposeLens Rowan parser

A custom lossless parser would provide maximum control but would duplicate substantial YAML grammar, recovery, and security work. It is not justified while a private dependency adapter and regression corpus keep replacement possible.

### Direct object deserialization

Object-first parsing was rejected because it normalizes scalar spelling, commonly collapses duplicate keys, discards comments, and couples syntax acceptance to an incomplete typed Compose model.
