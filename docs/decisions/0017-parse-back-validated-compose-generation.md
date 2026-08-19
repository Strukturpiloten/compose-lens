# ADR 0017: parse-back-validated Compose generation

- Status: accepted
- Date: 2026-08-04
- Additive amendments: 2026-08-05, 2026-08-06, 2026-08-07, 2026-08-15, 2026-08-19

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
5. Strings are emitted through ComposeLens's private minimal-safe YAML encoder. Plain spelling is
   used only when parser validation and YAML 1.1 ambiguity checks prove that string identity is
   retained. The parser dependency remains private.
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
10. Generated service labels use ordered mapping syntax and explicit YAML string values. Empty
    values and embedded `=` characters remain unambiguous, duplicate names are rejected, and
    caller-marked value sensitivity propagates to the generated document.
11. Generated service environment files retain ordered short or long syntax. Long entries emit
    only caller-selected `required` and `format: raw` options. Paths use the generated-string
    sensitivity boundary, and generation never resolves, opens, or parses the referenced file.
12. Generated service devices distinguish omission from an explicit empty vector and retain
    ordered mixed short/long forms plus exact duplicates. Long source is required; all strings must
    be safe resolved single-line values. Generation quotes and parse-back validates them without
    inspecting host devices, parsing colon triples, validating CDI or permissions, or claiming
    runtime access.
13. Generated DNS fields preserve caller-selected form, ordering, and explicit empty state while
    accepting only resolved physical-line-safe values.
14. Generated `expose` accepts unique documented decimal port/range forms and does not infer a
    protocol or runtime publication.
15. Generated `annotations` accepts unique resolved names with explicit string values.
16. Generated `security_opt` preserves one ordered raw sequence, including duplicates, without
    profile, SELinux, path, filesystem, provider, runtime, or cross-format normalization.
17. Every successful generated document is quoted where required and parsed back through the
    native model.
18. Generated service `logging` requires an explicit uninterpreted string driver and ordered
    unique non-empty option keys. Options select string, validated YAML number, or null kind;
    empty maps remain explicit, sensitivity propagates, and no defaults or provider semantics are
    inferred.
19. Generated top-level network driver configuration uses a distinct API rather than extending the
    shared basic `GeneratedResource`. The driver-configured API is application-owned; external
    networks stay on `GeneratedResource::external` because Compose permits only `name` alongside
    `external`. Optional opaque `driver` and ordered unique `driver_opts` retain explicit string
    or validated-number scalar identity, empty-map state, and sensitivity; plugin availability and
    provider-specific option semantics remain unvalidated.
20. Generated top-level volume driver configuration uses distinct volume-specific option types
    rather than network option types or the shared basic/external `GeneratedResource`. The
    driver-configured API is application-owned; external volumes remain
    `GeneratedResource::external`. Optional opaque `driver` and ordered unique `driver_opts`
    retain explicit string or validated-number scalar identity, empty-map state, sensitivity, and
    parse-back fidelity; plugin, provider, runtime, default, and image semantics remain
    unvalidated.
21. Generated application-owned volume definitions accept ordered unique `GeneratedLabel` mappings
    exactly once, including an explicit empty map. Labels use explicit YAML string values,
    preserve sensitivity, and parse back through the existing volume label model. The shared
    `GeneratedResource::external` API remains the sole generated external-volume path. At authored
    and project-view boundaries, literal `external: true` plus any volume `labels` attribute emits
    a distinct diagnostic while retaining labels; a simultaneous driver configuration continues to
    emit its existing independent diagnostic.
22. Generated top-level configs and secrets begin with one file-backed definition each. Names and
    `file` values are required resolved single-line strings, names are unique within their native
    namespaces, values use deterministic minimal safe quoting and parse-back validation, and caller-marked
    file sensitivity redacts debug output. Content, environment, external lifecycle, drivers,
    labels, template drivers, file access, and provider/runtime semantics remain ungenerated.

## Consequences

- BoxFerry and other consumers can generate Compose without maintaining their own YAML writer or
  depending on ComposeLens internals.
- Canonical rendering remains a separate path for authored merged projects; both paths share the
  marker-first minimal-safe YAML presentation defined by ADR 0024.
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
