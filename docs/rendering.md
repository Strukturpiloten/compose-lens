# Rendering and source editing

ComposeLens has three output paths because “render this project”, “create a new Compose document”,
and “change one authored value” have different fidelity contracts.

| Path                | Input                                          | Preserves                                         | Use it when                                            |
| ------------------- | ---------------------------------------------- | ------------------------------------------------- | ------------------------------------------------------ |
| Canonical rendering | `MergedProject` and optional profile selection | Effective values, order, and Compose syntax forms | Stable whole-project YAML is wanted                    |
| Generated document  | ComposeLens-owned builder values               | Explicit construction choices                     | An application creates new Compose YAML                |
| Preservation edits  | `SyntaxDocument` and exact scalar spans        | Every unrelated source byte                       | A focused source change must retain style and comments |

None of these paths loads files, interpolates variables, merges documents, chooses profiles, applies
defaults, validates a provider, or invokes a runtime. Callers run the processing stages they need
before selecting an output path.

## Canonical rendering

`render::render_canonical` emits the `compose-lens-canonical-v2` representation. The default is:

- explicit `---` document marker;
- two-space indentation;
- LF line endings and one final LF;
- retained mapping and sequence order;
- retained effective short or long Compose forms; and
- parser-validated minimal quoting for string keys and values.

Ambiguous YAML boolean, null, numeric, date, timestamp, and other non-string spellings remain quoted.
Native booleans, numbers, and null values retain their typed YAML form. Safe retained tags remain;
unsafe tags and unresolved aliases produce diagnostics and a parseable recovery value.

`render_canonical_with_formatting` accepts `CanonicalFormatting` for indentation width, LF or CRLF,
document-marker emission, and final-line-ending emission. Formatting cannot reorder data, normalize a
Compose form, or enable a processing stage. The default stays the exact shared byte contract.

An optional `ProfileSelection` must belong to the same merged project. A matching selection removes
inactive services from output but does not remove top-level resources.

## Generated documents

Generated builders accept ComposeLens-owned values for fields with an explicit construction contract.
Coverage is demand-driven; a field is not added merely because the parser recognizes it.

Every generated field must define:

- omission, empty, and syntax-form behavior;
- invalid-value and duplicate rejection;
- sensitivity propagation and redacted debugging;
- deterministic bytes; and
- parse-back coverage through the syntax and native model.

Set-once builder operations prevent accidental conflicting output. Names and strings are validated as
safe YAML values, while provider/runtime defaults and cross-format normalization remain outside the
builder. A successful complete document is parsed back before it is returned.

The current generated surface is documented by Rustdoc and executable tests in
`tests/generated_rendering.rs`. [ADR 0017](decisions/0017-parse-back-validated-compose-generation.md)
defines the construction boundary.

Generated service environment entries render in lexical key order. Sorting is stable for equal
keys, so duplicate list-form entries keep their relative last-value behavior. This generated-output
rule does not change canonical rendering, which retains effective authored order.

## Preservation edits

`render::apply_preservation_edits` atomically replaces existing YAML value scalars at exact
`SourceSpan` values. A typical workflow is:

1. Parse a `SyntaxDocument`.
2. Extract a typed field and its span.
3. Create one or more `ScalarEdit` values.
4. Apply the complete batch to the original syntax document.
5. Inspect diagnostics and `is_valid()` before using explicit output.

```rust
use compose_lens::model::ComposeDocument;
use compose_lens::render::{ReplacementScalar, ScalarEdit, apply_preservation_edits};
use compose_lens::source::SourceId;
use compose_lens::syntax::SyntaxDocument;

let source = "---\nservices:\n  app:\n    image: example.invalid/app:1\n";
let syntax = SyntaxDocument::parse(SourceId::new(1), source).expect("source fits");
let typed = ComposeDocument::parse(syntax.document());
let image = typed
    .document()
    .and_then(|document| document.service("app"))
    .and_then(compose_lens::model::Service::image)
    .expect("fixture has image");
let edit = ScalarEdit::new(image.span(), ReplacementScalar::string("example.invalid/app:2"));
let result = apply_preservation_edits(syntax.document(), &[edit]);

assert!(result.is_valid());
assert!(result.output().contains("example.invalid/app:2"));
```

The complete batch is rejected if a span belongs to another source, does not identify one supported
value scalar, overlaps another edit, targets a block or multiline scalar, or contains an invalid
typed replacement. Failure returns the original bytes.

String replacements retain compatible authored quote style and otherwise use deterministic double
quotes. Boolean, number, and null replacements use their explicit YAML type. This API does not insert
or delete fields, edit keys, rebuild collections, transform syntax forms, or rewrite block scalars.

[ADR 0010](decisions/0010-atomic-span-based-preservation-edits.md) owns the atomic editing contract.

## Sensitive output

Use sensitive construction or replacement values when output contains credentials or other protected
data. Successful text necessarily contains the value and is available through an explicit accessor;
the value is redacted from builder, edit, result, and diagnostic `Debug` output. Callers control where
explicit output is displayed or written.

Canonical and formatting decisions are recorded in ADRs
[0009](decisions/0009-deterministic-canonical-rendering.md),
[0011](decisions/0011-presentation-only-render-formatting.md), and
[0024](decisions/0024-safe-minimal-yaml-presentation.md).
