# Parse and render Compose

ComposeLens does not bundle parsing, interpolation, merging, profile selection, validation, and rendering into one opaque call. Each stage returns its own diagnostics and can be omitted.

```text
source text -> syntax -> typed document -> loaded inputs
            -> optional interpolation -> merge -> optional profiles
            -> validation or project view -> rendering
```

## Canonical output

Use `render::render_canonical` when you want deterministic Compose output from a merged project. Canonical rendering normalizes presentation, not hidden processing: it does not interpolate variables, choose profiles, resolve paths, read environment files, or contact a runtime.

Use the generated-document API when creating Compose from typed ComposeLens values. Output is parsed back before success is returned. Use `render::apply_preservation_edits` when a small source edit must retain surrounding comments and style.

## Processing that needs caller input

- Pass an `interpolation::EnvironmentProvider` implementation to interpolate. `MapEnvironment` is useful when the application has already authorized values.
- Supply ordered `loader::DocumentInput` values and origins for multi-file loading.
- Implement the include loader and path-resolution traits if includes or referenced files may be accessed.
- Pass a `profiles::ProfileRequest` when profile selection is wanted.

ComposeLens never reads `.env`, environment files, the process environment, or referenced paths merely because they appear in a document.
