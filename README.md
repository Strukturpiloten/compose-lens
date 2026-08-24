# ComposeLens

ComposeLens is a Rust library for reading, inspecting, processing, editing, and rendering Compose
documents. It keeps authored syntax separate from processed project meaning, so callers choose when
interpolation, merging, profile selection, path handling, or compatibility validation happens.

## Why use it

- Preserve source locations, unknown fields, extensions, and meaningful short or long syntax.
- Inspect one authored document or a provenance-rich multi-file project view.
- Supply environment values, include content, path policy, profiles, and target versions explicitly.
- Return structured diagnostics and partial results instead of printing or terminating the process.
- Produce deterministic Compose YAML or make focused edits without rewriting unrelated source.

ComposeLens never starts containers, contacts a runtime, reads the process environment implicitly,
or converts Compose directly into another format. Cross-format conversion belongs to
[BoxFerry](https://github.com/Strukturpiloten/boxferry).

## Start

Add the crate:

```console
cargo add compose-lens
```

Parse an in-memory document:

```rust
use compose_lens::{model::ComposeDocument, source::SourceId, syntax::SyntaxDocument};

let source = "---\nservices:\n  web:\n    image: example.invalid/web:1\n";
let syntax = SyntaxDocument::parse(SourceId::new(1), source).expect("source fits");
let parsed = ComposeDocument::parse(syntax.document());

assert!(parsed.document().and_then(|document| document.service("web")).is_some());
assert!(parsed.diagnostics().is_empty());
```

Rust 1.85.0 or newer is required.

## Processing model

```text
source text -> syntax -> typed document -> loaded inputs
            -> optional interpolation -> merge -> optional profiles
            -> project view or validation -> rendering
```

Each arrow is an explicit API boundary. A caller may stop at the narrowest stage that answers its
question, and every stage keeps diagnostics and source evidence available.

## Documentation

- [Human guides on boxferry.dev](https://boxferry.dev/docs/libraries/compose-lens/)
- [Rust API on boxferry.dev](https://boxferry.dev/docs/api/compose-lens/)
- [Contributor documentation](docs/README.md)
- [Architecture decisions](docs/decisions/README.md)
- [Changelog](CHANGELOG.md)

Repository-specific instructions for coding agents are in [AGENTS.md](AGENTS.md).

## Stewardship and license

ComposeLens is maintained by [Martin “Becks” Beckert](https://github.com/TheRealBecks) through
[Strukturpiloten OHG](https://www.strukturpiloten.de/). It is licensed under
[MPL-2.0](LICENSE).
