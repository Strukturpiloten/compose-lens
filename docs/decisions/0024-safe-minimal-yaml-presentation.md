# ADR 0024: Marker-first YAML with safe minimal string quoting

- Status: accepted
- Date: 2026-08-19
- Amends: ADR 0009, ADR 0011, and ADR 0017

## Context

ComposeLens's previous canonical and generated output quoted every mapping key and string. The
bytes were safe but difficult to scan, unlike normal Compose files. Complete generated documents
also omitted the explicit YAML document marker even though it improves standalone copy/paste and
works with YAML directives.

The renderer cannot choose plain scalars only by appearance: a string such as no is a YAML 1.1
boolean in older consumers, while 007, null, a date, or syntax indicators can change type or
structure.

## Decision

1. Canonical and generated complete Compose documents start with ---.
2. CanonicalFormatting::default() emits the marker. with_document_marker(false) remains the
   explicit opt-out for callers that require marker-free output.
3. String keys and values use plain YAML only when the private parser proves that the complete
   candidate is one plain YAML string. Otherwise they use deterministic double quotes.
4. YAML 1.1 boolean and null spellings (y, yes, n, no, on, off, true, false, null, and ~, in their
   defined case variants) remain quoted even when the parser accepts them as strings.
   Sexagesimal-looking numbers, special floating-point spellings, dates, and timestamps remain
   quoted too.
5. Native boolean, number, and null values retain their typed YAML spelling. The renderer does
   not quote them merely to make output uniform.
6. The new exact byte contract is compose-lens-canonical-v2; parse-back and generated-document
   tests cover safe plain and required-quote cases.

## Consequences

- Common Compose output is concise: services, service names, image references, and ordinary
  labels no longer carry visual noise.
- String intent stays portable across YAML 1.1 and newer YAML readers.
- Canonical bytes intentionally change from canonical-v1. Consumers needing a marker-free
  document can select the existing explicit formatting option.
- Preservation editing remains separate and retains authored styles outside the requested edit.

## Alternatives considered

### Keep quoting every string

Rejected because correct output was unnecessarily hard for people to read and copy.

### Emit every string plain

Rejected because YAML scalar inference can silently change a string key or value into another
type or YAML structure.
