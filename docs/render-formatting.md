# Render formatting

ComposeLens separates merged-project semantics from YAML presentation. `render_canonical` always
uses the fixed canonical-v1 format. `render_canonical_with_formatting` accepts explicit
presentation-only choices while producing the same YAML data model.

## Available choices

| Option               | Canonical-v1 default | Supported values                   |
| -------------------- | -------------------- | ---------------------------------- |
| Indentation width    | two spaces           | any positive `u8` number of spaces |
| Line ending          | LF                   | LF or CRLF                         |
| YAML document marker | omitted              | emit or omit `---`                 |
| Final line ending    | emitted              | emit or omit                       |

```rust
use compose_lens::render::{CanonicalFormatting, IndentWidth, LineEnding};

let formatting = CanonicalFormatting::default()
    .with_indent_width(IndentWidth::new(4).expect("four is nonzero"))
    .with_line_ending(LineEnding::CrLf)
    .with_document_marker(true)
    .with_final_newline(false);

assert_eq!(formatting.indent_width().spaces(), 4);
assert_eq!(formatting.line_ending(), LineEnding::CrLf);
assert!(formatting.document_marker());
assert!(!formatting.final_newline());
```

Pass this value to `render_canonical_with_formatting` together with the same merged project and
optional profile selection accepted by `render_canonical`.

## Options that deliberately do not exist

Formatting cannot:

- interpolate environment variables;
- merge Compose documents;
- select profiles;
- resolve paths or apply defaults;
- sort mappings or services;
- normalize or interchange short and long Compose forms;
- validate provider/runtime compatibility; or
- invoke Docker, Podman, or another process.

String and key quoting stays deterministic because changing scalar representation requires YAML
type-safety rules, not aesthetic preference alone. Mapping and sequence order remains the order of
the merged semantic model.

## Canonical versus customized output

Only the default is named `compose-lens-canonical-v1` for exact fixture comparison. Customized
output remains deterministic for the same project, selection, and formatting value, and it must
parse to the same semantic model. Use canonical-v1 when stable shared bytes are more important than
repository-specific presentation.

Formatting options do not apply to preservation-oriented edits. Those edits retain all authored
bytes outside their exact target spans. ADR 0011 defines the [formatting boundary](decisions/0011-presentation-only-render-formatting.md).
