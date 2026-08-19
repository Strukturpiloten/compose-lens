# ComposeLens diagnostics

Every processing stage exposes diagnostics instead of printing or terminating the process. A diagnostic contains a stable machine-readable code, severity, human message, source labels, and optional notes.

## Handle partial results

Call `is_valid()` when a stage must be free of errors before continuing, and inspect `diagnostics()` regardless of success. Recovery-oriented parsing may return useful typed or syntax evidence beside errors. Applications decide whether warnings are displayed, promoted, or collected.

Source labels use `source::SourceId` and byte spans. Keep the matching source text or origin metadata so a UI can turn those spans into filenames, lines, and columns.

## Stable integration boundary

Diagnostic code strings are the automation key. Messages and source excerpts are for people and may improve over time. Do not parse display text to make decisions.

Sensitive model values redact their `Debug` output by default. Applications should preserve that boundary when attaching source excerpts or serializing their own reports.

ComposeLens does not write to stdout or stderr and does not choose an exit code. BoxFerry's CLI is one example of translating library findings into grouped human and JSON output.
