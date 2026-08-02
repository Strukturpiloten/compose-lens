# ADR 0015: Byte-preserving YAML backend compatibility

- Status: accepted
- Date: 2026-08-02

## Context

The first BoxFerry Compose adapter fixture exposed a private `yaml-edit` parser defect. In a block
sequence, a valid unquoted short-volume scalar such as `./data:/data:Z,ro` was terminated at the
comma. The backend reported no parse error and omitted the rest of the document root. Returning a
valid-looking partial document violates ComposeLens's loss-aware parsing contract.

The immediate 0.1.x fix added a complete-root check and the stable
`compose.yaml.unparsed-input` diagnostic. That prevented silent interpretation but still forced a
quoted workaround for valid real-world Compose source. Replacing or forking the complete YAML
backend for a patch release would introduce substantially more parser risk than the isolated
defect warrants.

## Decision

ComposeLens keeps `yaml-edit` private and adds a constrained, same-byte-length compatibility input
for the backend:

1. Commas in single-line block-style plain values are replaced only in the private parse input.
2. Flow collections, quoted scalars, comments, and literal or folded block scalars are not changed.
3. Replacement is one ASCII byte for one ASCII byte, so concrete-tree byte ranges remain aligned
   with the caller's source.
4. The public `SyntaxDocument`, preservation rendering, editing, interpolation, native parsing, and
   merging use the original source text for authored scalar spelling and semantic recovery.
5. The complete-root guard remains mandatory. Any future backend omission that the compatibility
   path does not cover still produces `compose.yaml.unparsed-input` rather than partial success.
6. Authored regressions exercise syntax validity, exact preservation, native short-volume options,
   downstream merge behavior, quotes, flow collections, comments, and block scalars.

The compatibility layer is an internal parser adapter, not a Compose normalization rule. No
public parser-dependency type or transformed text is exposed.

## Consequences

- Valid unquoted comma-separated short-volume options work without changing their spelling.
- Source spans remain byte-accurate and preservation rendering remains byte-identical.
- The 0.1.x parser dependency can remain stable while ComposeLens owns the defect boundary.
- Every future compatibility rule must be constrained, source-preserving, independently tested,
  and backed by the complete-root safety check.
- Repeated parser defects that require structural YAML emulation are evidence to replace the
  backend rather than grow this adapter without bound.

## Alternatives considered

- Keeping only the structured error was rejected because it made valid common Compose source
  unusable and left BoxFerry dependent on a quoting workaround.
- Forking `yaml-edit` for 0.1.1 was rejected because ComposeLens would assume a second parser
  maintenance surface for one localized tokenization defect.
- Switching YAML libraries in a patch release was rejected because comment, duplicate-key,
  recovery, source-span, and editing behavior would all need to be requalified.
- Canonicalizing or quoting the caller's source was rejected because the syntax layer must not
  rewrite input merely to parse it.
