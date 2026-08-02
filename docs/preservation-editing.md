# Preservation-oriented editing

ComposeLens can atomically replace existing YAML value scalars while retaining every unrelated
source byte. The operation is intended for focused transformations such as changing an image
reference, command argument, environment value, boolean, number, or explicit null.

## Workflow

1. Parse a `SyntaxDocument`.
2. Extract the typed document and select the field to change.
3. Use that field's exact `SourceSpan` to create a `ScalarEdit`.
4. Apply all edits to the original syntax document in one batch.
5. Check `is_valid()` before consuming the explicit output.
6. Parse the output again when the next operation needs a new syntax or typed document; old spans
   refer to the old source layout.

```rust
use compose_lens::model::ComposeDocument;
use compose_lens::render::{ReplacementScalar, ScalarEdit, apply_preservation_edits};
use compose_lens::source::SourceId;
use compose_lens::syntax::SyntaxDocument;

let source = "services:\n  app:\n    image: example/app:1\n";
let syntax = SyntaxDocument::parse(SourceId::new(1), source).expect("source fits in the syntax tree");
let typed = ComposeDocument::parse(syntax.document());
let image = typed
    .document()
    .and_then(|document| document.service("app"))
    .and_then(compose_lens::model::Service::image)
    .expect("fixture has an image");
let edit = ScalarEdit::new(image.span(), ReplacementScalar::string("example/app:2"));
let result = apply_preservation_edits(syntax.document(), &[edit]);

assert!(result.is_valid());
assert_eq!(result.output(), "services:\n  app:\n    image: example/app:2\n");
```

## Atomic validation

The complete batch is rejected when any edit:

- belongs to another source document;
- does not exactly match one YAML value scalar;
- overlaps another requested edit;
- targets a block or multiline scalar; or
- supplies text that is not exactly one YAML number for a numeric replacement.

On rejection, `output()` returns the original bytes and `is_sensitive()` is false because no
replacement was inserted. Diagnostics identify source spans but never contain replacement values.

## Style behavior

| Authored target | String replacement behavior |
| --- | --- |
| Double quoted | remains double quoted with deterministic escaping |
| Single quoted | remains single quoted when safe; apostrophes are doubled |
| Plain | remains plain only when it parses as one complete YAML string |
| Unsafe plain or single-quoted replacement | falls back to double quotes |
| Block or multiline scalar | rejected by this operation |

Boolean replacements use lowercase YAML spelling. Numeric spelling is retained after validation.
Null replacements use `null`. Typed replacement kinds are explicit, so replacing a quoted string
with a boolean is an intentional type change.

## Security boundary

Use `ReplacementScalar::sensitive_string` for secret-bearing content. The edited text necessarily
contains the value and is available through the explicit output accessor. Debug formatting for the
replacement, edit, and successful result is redacted. Callers remain responsible for where they
write or display explicit output.

## Current limits

This API does not insert or delete fields, edit keys, transform short and long Compose forms,
rebuild collections, or rewrite block scalars. It also does not automatically rerun interpolation,
merging, profile selection, compatibility validation, or any runtime command. ADR 0010 defines the
[durable editing contract](decisions/0010-atomic-span-based-preservation-edits.md).
