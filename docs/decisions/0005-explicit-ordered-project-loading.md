# ADR 0005: explicit ordered project loading

- Status: accepted
- Date: 2026-07-31

## Context

A multi-file Compose project needs more information than a vector of parsed YAML documents. File
order changes merge results, the first file establishes the project directory for relative paths,
and every input still needs its own origin for diagnostics, provenance, and future `include`
handling.

Finding files, reading standard input, choosing a working directory, and discovering environment
sources are application concerns. Performing those operations in the core loader would make tests
and library behavior depend on ambient machine state. Parsing may also recover from malformed YAML,
so a loader that rejects every diagnostic would discard useful analysis evidence.

## Decision

1. `LoadedProject::load` accepts an ordered collection of caller-supplied `DocumentInput` values.
   It does not discover or read files.
2. Every input has a unique caller-managed `SourceId`, source text, and `DocumentOrigin`.
3. A document origin contains an opaque display label and an explicit directory. ComposeLens
   retains both without canonicalizing paths or consulting the file system.
4. The first document's directory becomes the multi-file project base. Later document directories
   remain attached as provenance but do not replace that base.
5. Empty projects, duplicate source identifiers, and syntax-tree capacity failures are fatal load
   errors. Recoverable syntax and typed-model diagnostics remain attached to their document and are
   aggregated by the project.
6. Loading does not interpolate, merge, select profiles, resolve paths, or apply defaults.
7. Project interpolation is an explicit follow-up operation that creates one overlay per document
   in file order. The source documents remain unchanged.

## Consequences

- CLI and editor integrations can implement their own discovery, standard-input, URI, and access
  policies without changing the parser.
- Unit and integration tests need no temporary files or process-environment mutation.
- Merge and path-resolution stages receive stable ordering and origin evidence.
- The API requires callers to supply a directory even for in-memory or standard-input documents;
  that explicit value is the intended resolution context.
- `include` will need a distinct loading operation because included Compose projects have different
  project-directory semantics from a multi-`-f` project.

## Alternatives considered

### Accept paths and read them inside `LoadedProject::load`

Rejected because it combines I/O policy, discovery, parsing, and project processing, and cannot
represent editor buffers or remote sources cleanly.

### Derive every relative path from its containing override file

Rejected because Compose multi-file behavior uses the first file as the path base. Individual
origins are still retained for provenance and future operations with different rules.

### Reject a project when any document has a parse diagnostic

Rejected because ComposeLens is an analysis and migration library. Recoverable source-aware
results are more useful than an all-or-nothing loader.
