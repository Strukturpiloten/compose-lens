# Software architecture

ComposeLens is a pure, source-aware Compose library. It separates authored YAML, native Compose
meaning, project processing, compatibility evidence, and output so a caller never receives hidden
environment or runtime behavior.

## Ownership

ComposeLens owns:

- loss-aware Compose YAML syntax and source locations;
- native Compose document and project types;
- caller-supplied loading, interpolation, merge, include, profile, and resolution operations;
- evidence-backed compatibility findings; and
- deterministic output and preservation-oriented edits.

Applications own file discovery, environment precedence, authorization, user interaction, target
selection, and cross-format conversion. BoxFerry consumes the public native boundary; ComposeLens
does not depend on BoxFerry or contain BoxFerry's neutral model.

## Layers

```text
source bytes
    |
    v
syntax document -> typed document -> loaded project -> merged project
       |                |                 |                 |
       +-- spans        +-- native forms  +-- origins       +-- provenance
       +-- diagnostics  +-- unknown data  +-- overlays      +-- project views
       |                                                    |
       +---------------- rendering and focused editing <----+
```

| Layer               | Responsibility                                                                   | Deliberately absent                             |
| ------------------- | -------------------------------------------------------------------------------- | ----------------------------------------------- |
| Syntax              | YAML structure, spelling, comments, spans, recovery                              | Compose defaults and environment access         |
| Typed document      | Source-aware Compose fields and syntax alternatives                              | Merge, profile selection, runtime support       |
| Loaded project      | Ordered caller-supplied inputs and origins                                       | File discovery and implicit interpolation       |
| Processing          | Interpolation overlays, field-aware merge, includes, profiles, paths, references | Runtime execution and destructive normalization |
| Native project view | Effective Compose values with complete provenance                                | Cross-format types and target policy            |
| Compatibility       | Findings for explicit provider/runtime versions                                  | Host detection and “latest” inference           |
| Output              | Canonical YAML, generated documents, focused source edits                        | Hidden processing stages                        |

The private YAML backend is an implementation detail. Public APIs expose only ComposeLens-owned
syntax, source, diagnostic, and model types. [ADR 0002](decisions/0002-loss-aware-yaml-syntax.md)
defines that boundary, and
[ADR 0015](decisions/0015-byte-preserving-yaml-backend-compatibility.md) records the narrowly
constrained compatibility adapter used to retain valid authored bytes.

Short and long Compose syntax remains field-specific because equivalent-looking forms may carry
different defaults or provider behavior. [ADR 0003](decisions/0003-preserve-compose-syntax-forms.md)
is the durable representation rule.

## Processing is explicit

Parsing, loading, interpolation, merging, include traversal, profile selection, path handling,
default decisions, reference validation, compatibility validation, and rendering are separate
operations. Each receives all external context as an argument and returns diagnostics plus a new
view or result. Inputs remain available for later explanation.

The [processing model](processing-model.md) describes the stages. Durable decisions live in ADRs
0004 through 0008 and 0020 through 0023.

## Module ownership

| Concern                                 | Module          |
| --------------------------------------- | --------------- |
| Text positions and source identity      | `source`        |
| YAML structure and spelling             | `syntax`        |
| Diagnostics                             | `diagnostic`    |
| Native Compose types                    | `model`         |
| Ordered documents and includes          | `loader`        |
| Variable substitution                   | `interpolation` |
| Compose merge                           | `merge`         |
| Profiles                                | `profiles`      |
| Effective native project values         | `project`       |
| Paths, references, and defaults         | `resolution`    |
| Provider/runtime evidence               | `validation`    |
| Canonical, generated, and edited output | `render`        |

Dependencies point from later stages toward earlier representations. Syntax does not import model
semantics, the native model does not import BoxFerry types, and parsing modules do not gain file or
runtime access. New code belongs in the narrowest module that owns its behavior.

## Side-effect and security boundaries

Core library operations are deterministic for the same explicit inputs. They do not:

- read files, `.env`, the process environment, or user home implicitly;
- canonicalize paths, follow symlinks, or test file existence;
- contact Docker, Podman, registries, networks, or model providers;
- start, stop, inspect, or mutate infrastructure; or
- print diagnostics or choose a process exit code.

Caller-provided traits authorize optional data acquisition. Raw values remain available through
explicit accessors, while sensitive types redact `Debug` output and diagnostics. A diagnostic stores
stable codes and source labels, not secret values. Generated or edited text that necessarily
contains a secret is available only through explicit result accessors.

## Public release boundary

The supported surface consists of ComposeLens-owned public modules exercised by
`tests/public_api.rs`. Parser dependencies stay private, side-effect boundaries remain stable, and
intentional pre-1.0 breaking changes require a new minor version and migration notes. The current
contract is in [API stability](api-stability.md); detailed items belong in generated Rustdoc rather
than a copied symbol catalogue.
