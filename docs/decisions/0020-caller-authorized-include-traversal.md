# ADR 0020: caller-authorized include traversal

- Status: accepted
- Date: 2026-08-14

## Context

Compose `include` declarations are typed at document and effective-project boundaries, but parsing
them must not turn raw paths into ambient filesystem, environment, or provider behavior. Consumers
need a graph for diagnostics and orchestration before any decision to compose child resources.

## Decision

1. `IncludeResolution` traverses a caller-created root depth-first in effective include order.
   Every reached node uses the existing ordered load, authored no-interpolation merge, and native
   project view before its declarations are considered.
2. `IncludeLoader` is the sole I/O and authorization boundary. It receives an `IncludeRequest`
   retaining declaration span/source, parent identity/base directory, complete typed item, and raw
   path, `env_file`, and `project_directory` order. It returns caller-created document inputs and
   an opaque caller-canonical identity.
3. Active-stack identities detect cycles; source IDs are unique across the whole traversal.
   Loader denial/failure, empty results, malformed/unmodeled declarations, and project-load errors
   produce stable `compose.include.*` diagnostics while preserving partial nodes, edges, requests,
   and origins. Diamonds are not cached.
4. Traversal does not open, canonicalize, or join paths; read environment or `.env` files;
   interpolate include values; infer project names; merge/import child resources; or assert
   provider behavior.

## Consequences

Consumers can impose workspace, URI, archive, editor-buffer, and authorization policies without
losing source-aware traversal evidence. The result is intentionally not a composed Compose
project: resource composition, path policy, environment precedence, project-name behavior, and
provider evidence remain explicit future work.

## Alternatives considered

### Resolve paths and files in the parser or ordinary project loader

Rejected because it would make parsing depend on ambient machine state and violate ADR 0005's
caller-supplied loading boundary.

### Merge included resources during traversal

Rejected because it conflates graph discovery with unproven composition semantics and would hide
the distinct origin and project-directory contexts of child projects.

### Use path strings as identities

Rejected because identity equivalence is policy-dependent for files, URIs, overlays, and editor
buffers; callers must provide canonical identities explicitly.
