# YAML representation evaluation

This evaluation supports [ADR 0002](../decisions/0002-loss-aware-yaml-syntax.md). It is evidence for the initial syntax representation, not a permanent claim that one parser implements every YAML or Compose behavior.

## Evaluation date and versions

Evaluated on 2026-07-31 with the project MSRV of Rust 1.85.0 and the pinned development toolchain.

| Candidate                                                                                                                                             |         Version | License           |      Declared Rust version |
| ----------------------------------------------------------------------------------------------------------------------------------------------------- | --------------: | ----------------- | -------------------------: |
| [`yaml-edit`](https://docs.rs/yaml-edit/0.2.3/yaml_edit/)                                                                                             |           0.2.3 | Apache-2.0        |                       1.70 |
| [`tree-sitter`](https://docs.rs/tree-sitter/0.26.11/tree_sitter/) plus [`tree-sitter-yaml`](https://docs.rs/tree-sitter-yaml/0.7.2/tree_sitter_yaml/) | 0.26.11 / 0.7.2 | MIT               |          1.77 / undeclared |
| [`saphyr-parser`](https://docs.rs/saphyr-parser/0.0.11/saphyr_parser/)                                                                                |          0.0.11 | MIT OR Apache-2.0 | 1.85.0 in package metadata |

All evaluated licenses are already allowed by `deny.toml`.

## Probe corpus

The same Compose-shaped source was supplied to each parser. It contained a comment, an anchor, an alias through a merge key, a quoted scalar with significant spelling, and duplicate mapping keys. A second source contained an unclosed flow sequence.

The temporary probe reported:

```text
yaml-edit exact=true comments=1 malformed_errors=1
tree-sitter valid=true comments=1 malformed_has_error=true
saphyr events=20 aliases=1 malformed_has_error=true comment_events=unsupported
```

The committed `syntax` and `roundtrip` integration tests extend that probe with Compose extensions, interpolation-shaped text, a tag-plus-digest image reference, Unicode, CRLF line endings, source spans, exact reparsing, and fixture provenance.

## Findings

| Requirement                         | `yaml-edit`                         | Tree-sitter YAML                                                      | `saphyr-parser`                                      |
| ----------------------------------- | ----------------------------------- | --------------------------------------------------------------------- | ---------------------------------------------------- |
| Concrete tree retains trivia        | Yes                                 | Comments are nodes; other trivia requires retaining source separately | No; semantic event stream                            |
| Exact tree rendering                | Yes in the probe                    | No standalone emitter from the syntax tree                            | No                                                   |
| Anchors and aliases                 | Retained as syntax                  | Retained as grammar nodes                                             | Retained as semantic events and IDs                  |
| Duplicate mapping keys              | Retained in the concrete tree       | Retained in the parse tree                                            | Events are available before object mapping           |
| Positioned malformed-input recovery | Tree plus positioned errors         | Error and missing nodes                                               | Positioned parse error; event processing stops       |
| Semantic YAML 1.2 role              | Partial and still maturing          | Grammar, not a semantic validator                                     | Strongest evaluated semantic parser                  |
| Build/runtime cost                  | Pure Rust Rowan tree                | Tree-sitter runtime plus generated C grammar                          | Pure Rust event parser                               |
| API coupling risk                   | Manageable behind a private adapter | Manageable but requires a larger adapter                              | Event model cannot satisfy source preservation alone |

## Selection

Use `yaml-edit` as the initial private concrete-syntax implementation with default features disabled. ComposeLens stores its immutable `Parse<YamlFile>` result and original text but exposes only ComposeLens-owned source, diagnostic, and syntax APIs.

This is a deliberately reversible selection:

- The dependency is pinned exactly because it is pre-1.0.
- No `yaml-edit` type appears in the public API.
- Parser updates must pass the authored preservation and malformed-input corpus on Rust 1.85.0.
- ComposeLens remains responsible for Compose typing, duplicate-key policy, alias semantics, validation profiles, and deterministic canonical rendering.
- `saphyr-parser` remains a possible semantic conformance oracle if the concrete parser accepts input whose YAML validity is uncertain.
- Tree-sitter remains a possible editor-oriented fallback if incremental parsing becomes a concrete requirement.

## Known limits and follow-up

- The current syntax document is read-only through the public API. Controlled preservation edits arrive only after their invariants are designed.
- The concrete tree uses 32-bit byte offsets; sources larger than that limit are rejected before parsing.
- Exact rendering is tested for the current corpus, not yet the full YAML test suite.
- `yaml-edit` is a young dependency. Maintenance, advisories, parser regressions, and API changes require review on every update.
- Syntax acceptance is not Compose validity. Typed and implementation-specific rules belong to later tasks.
