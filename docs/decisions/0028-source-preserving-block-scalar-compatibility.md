# ADR 0028: Source-preserving block scalar compatibility

- Status: accepted
- Date: 2026-10-02
- Supersedes: [ADR 0015](0015-byte-preserving-yaml-backend-compatibility.md)

## Context

An independently authored regression for ComposeLens issue 173 puts a lone quote in MongoDB's
literal healthcheck before a quoted PostgreSQL image. The private `yaml-edit` 0.3.2 lexer scans the
quote as a quoted scalar and can consume or reassociate the following service. The document root
can still cover every input byte, so the complete-root guard alone cannot detect the wrong ownership.
The same minimal input also loses a service with `yaml-edit` 0.2.3; rolling back the dependency is
insufficient.

Explicit indentation indicators expose a second boundary: the backend interprets the indicator as
an absolute column instead of an offset from the containing block. Its scalar decoder also treats
header comment characters as indicators, drops meaningful spaces from whitespace-only content,
and invents a final line break at EOF. These observations are isolated by authored tests; no
external oracle source was copied or mechanically translated.

The semantic contract is [YAML 1.2.2 section 8.1](https://yaml.org/spec/1.2.2/#81-block-scalar-styles):
quotes are ordinary block content; literal/folded style, relative indentation, line break
normalization, and chomping determine the decoded value independently of sibling mappings.

A historical read-only corroboration used the installed
[PyYAML 6.0.3](https://pypi.org/project/PyYAML/6.0.3/) public `safe_load` API on independently
authored whitespace-only examples. Installed distribution metadata identified version 6.0.3,
the MIT license (redistribution allowed), and `https://pyyaml.org/` as its origin. No upstream
source or retained output was copied. The command was:

```console
python3 -c 'import yaml; print("PyYAML", yaml.__version__); sources = ["x: |+\n  \n", "x: |\n  \n", "x: |-\n  \n", "x: |+\n    \n  \n", "x: |2+\n    \n"]; print([(repr(source), repr(yaml.safe_load(source)["x"])) for source in sources])'
```

This records an explicit local Python API observation, not a dependency pin, automatic test
oracle, provider/runtime result, or authority to install or invoke PyYAML in the library. The
Rust regressions assert independently expected YAML values without invoking an oracle.

## Decision

Retain ADR 0015's private, same-length adapter and its existing plain-scalar, anchor, blank-line,
alias, and complete-root rules. Replace its prohibition on block-scalar adaptation with this narrow
exception:

1. Track block headers and indented bodies outside quoted/flow syntax. Mask only ASCII quote bytes
   in block bodies, one byte for one byte, so the backend cannot quote-scan into dedented siblings.
2. For a valid explicit indentation header, use a same-length implicit header in private parse
   input. Replace excess indentation spaces in explicit-block content with private placeholder
   bytes so the backend infers the authored minimum even when the first line is more indented.
   Retain the authored indentation requirement separately; malformed headers or insufficient
   content indentation produce structured `compose.yaml.syntax` diagnostics.
3. Check every recognized block scalar against its authored boundary. A missing scalar, omitted
   non-whitespace content, or a span crossing a dedented sibling produces
   `compose.yaml.unparsed-input`, even when the complete document root covers all bytes.
4. Recover authored block text by its original span. Independently decode all literal/folded blocks
   using YAML 1.2.2's relative indentation, folding, line-break normalization, and chomping rules.
   Interpret header indicators only before any comment; preserve meaningful content spaces and
   actual final line breaks. The backend owns the concrete tree, not block-scalar meaning.
   Neither the stored tree nor the caller's source is mutated or re-lexed.
5. Keep original text and spans authoritative for raw values, preservation rendering, diagnostics,
   typed models, merge provenance, and project views. Expose no backend types or adapted text.
6. Preserve the existing interpolation policy: literal/folded blocks remain ineligible. Adjacent
   eligible scalars retain explicit caller-owned interpolation and sensitive-value redaction.

Focused authored tests cover service/environment ownership, literal and folded semantics, both
quote kinds, chomping, explicit indentation in mappings and sequences, comments, more-indented
content, whitespace-only/empty blocks, EOF, Unicode, CRLF, malformed input, source spans, merge,
native project views,
and interpolation/redaction boundaries. These are native syntax claims, not provider/runtime
conformance claims; no runtime is invoked and no dependency pin changes.

## Consequences

Valid block quotes cannot silently consume a sibling service. The compatibility adapter owns a
bounded structural safety check and a block-only semantic decoder. Any future backend replacement
must requalify these independently expected semantics. Existing plain-scalar compatibility and
the complete-root guard remain mandatory.

## Alternatives considered

- Downgrading the parser does not fix the minimal service-loss regression.
- Re-parsing original block text repeats the faulty quote scan.
- Decoding restored source on a scratch backend scalar avoids quote lexing but retains the
  backend's independent whitespace, comment, and final-line-break defects.
- Implementing a second complete YAML parser adds unnecessary grammar and recovery risk while
  the block-only semantic and structural compatibility boundary remains independently testable.
- Rewriting caller input would discard spelling and source evidence.
