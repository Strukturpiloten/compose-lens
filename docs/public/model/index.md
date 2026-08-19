# ComposeLens model

ComposeLens keeps authored syntax separate from processed meaning. Pick the narrowest stage that answers your question so the library never performs work you did not request.

| Stage            | Main type or function         | Use it for                                              |
| ---------------- | ----------------------------- | ------------------------------------------------------- |
| YAML syntax      | `syntax::SyntaxDocument`      | Parse bytes, retain spans, and make preservation edits. |
| Typed document   | `model::ComposeDocument`      | Inspect authored Compose fields and syntax forms.       |
| Loaded inputs    | `loader::LoadedProject`       | Keep ordered files and origins together.                |
| Merged project   | `merge::merge_project`        | Apply Compose merge rules with provenance.              |
| Selected project | `profiles::select_profiles`   | Select services using an explicit profile request.      |
| Native view      | `project::build_project_view` | Read processed, source-aware Compose values.            |

Malformed or unsupported input remains source-addressable where possible. A caller can inspect diagnostics and partial values instead of losing the original evidence.

## Direct use

```rust
use compose_lens::{model::ComposeDocument, source::SourceId, syntax::SyntaxDocument};

let syntax = SyntaxDocument::parse(
    SourceId::new(1),
    "---\nservices:\n  web:\n    image: example.invalid/web:1\n",
).expect("valid Compose input");
let compose = ComposeDocument::parse(syntax.document());
let web = compose.document().and_then(|document| document.service("web"));
assert!(web.is_some());
```

Parsing is in-memory and has no runtime side effects. File discovery and include loading stay with the caller through explicit loader interfaces.
