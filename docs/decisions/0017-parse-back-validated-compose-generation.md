# ADR 0017: parse-back-validated Compose generation

- Status: accepted
- Date: 2026-08-04
- Additive amendments: 2026-08-05, 2026-08-06

## Context

Canonical rendering starts with an authored, loaded, and merged project. It cannot construct a
new Compose definition from another model without fake source spans or a generated-YAML parse
bridge in every downstream converter. BoxFerry needs to generate Compose definitions from
reviewed runtime observations, while Compose-specific syntax selection and YAML escaping belong in
ComposeLens.

Short and long Compose syntax are not universally equivalent. In particular, preserving an
`SELinux` bind relabel request requires deliberate short-form output instead of assuming the long
`bind.selinux` spelling behaves identically in real providers.

## Decision

1. The `render` module owns an additive `ComposeDocumentBuilder` path for documents that have no
   authored source.
2. Public generated types are Compose-specific and contain no BoxFerry model types.
3. The initial builder covers the reviewed runtime-migration service, network, and volume subset.
   Top-level resources keep the Compose model key separate from an optional exact platform name,
   so application ownership does not implicitly rename an observed resource through project
   scoping.
4. Generated mapping collections retain insertion order and reject duplicate names. Singleton
   fields reject replacement rather than silently selecting the last caller value.
5. Strings are emitted through ComposeLens's private double-quoted YAML encoder. The parser
   dependency remains private.
6. Syntax is selected per field rather than normalized globally. TCP/UDP ports use long syntax
   with string-typed `published` values; SCTP uses the platform-protocol-capable short form.
   `GeneratedSelinux` bind mounts use short syntax explicitly. Ambiguous short-form values fail
   before output.
7. Every successful build reparses its exact bytes through both `SyntaxDocument` and
   `ComposeDocument`. The returned output exposes the validated native document.
8. Caller-marked sensitivity propagates to the result. Deployable text requires an explicit
   accessor and sensitive debug output is redacted.
9. Generation is pure. It does not perform interpolation, merging, profile selection, default
   resolution, compatibility validation, file access, or provider execution.
10. Generated service labels use ordered mapping syntax and explicit quoted string values. Empty
    values and embedded `=` characters remain unambiguous, duplicate names are rejected, and
    caller-marked value sensitivity propagates to the generated document.
11. Generated service environment files retain ordered short or long syntax. Long entries emit
    only caller-selected `required` and `format: raw` options. Paths use the generated-string
    sensitivity boundary, and generation never resolves, opens, or parses the referenced file.

## Consequences

- BoxFerry and other consumers can generate Compose without maintaining their own YAML writer or
  depending on ComposeLens internals.
- Canonical rendering remains unchanged for authored merged projects.
- Parse-back validation catches construction/renderer drift, but does not replace provider-version
  compatibility evidence.
- Adding further generated fields is an additive typed API change with exact golden and parse-back
  tests.
- The generator may choose different syntax forms per field when evidence says they are not
  interchangeable.

## Alternatives considered

- **Build generic YAML in BoxFerry and parse it with ComposeLens.** Rejected because every consumer
  would duplicate escaping, field ordering, form selection, and secret-redaction responsibilities.
- **Construct private merged-project values with generated spans.** Rejected because a generated
  document has no merge operations or authored provenance and should not pretend otherwise.
- **Expose the private YAML dependency as a builder API.** Rejected because it would make parser
  types part of the public compatibility contract.
- **Always emit Compose long syntax.** Rejected because field-specific runtime behavior, including
  `SELinux` relabel handling, can make the forms non-equivalent.
