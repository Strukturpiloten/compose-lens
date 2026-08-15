# ADR 0022: caller-owned include project-directory plans

- Status: accepted
- Date: 2026-08-14
- Extends: [ADR 0020](0020-caller-authorized-include-traversal.md)

## Context

Include traversal retains each project’s first document directory and the raw optional
`project_directory` declaration, but it must not decide whether a declaration is a local relative
path, absolute path, URI, archive entry, or opaque identifier. Nested includes still need an
inspectable effective parent directory when a caller chooses to apply its own policy.

## Decision

1. `IncludeResolution::plan_project_directories` is a separate, opt-in, I/O-free operation over
   retained include occurrences. It does not alter `ProjectView`, `MergedProject`, traversal, or
   composition.
2. The root uses its first `DocumentInput` directory. A child without an explicit declaration uses
   its own first document directory and never invokes caller policy.
3. Only an explicit declaration invokes `IncludeProjectDirectoryResolver`. Its request retains the
   include edge/request and parent/child occurrence identities and indices, raw located declaration,
   recursively effective parent directory when available, and child first-document directory.
4. Resolver policy returns an authorized `PathBuf`, a non-error deferred outcome, or the typed
   no-message unresolved error. ComposeLens neither joins, expands, interpolates, normalizes,
   canonicalizes, opens, nor existence-checks a value.
5. `IncludeProjectDirectoryPlan` preserves traversal diagnostics unchanged and appends the stable
   `compose.include.project-directory-unresolved` error for unresolved explicit declarations. It
   reports valid-but-incomplete deferred plans separately from invalid unresolved plans. Diagnostics
   use only declaration source labels and spans; raw and effective directories are redacted from
   derived debug output and diagnostics.
6. Planning follows non-cycle edges deterministically. Cycle edges create no new entry and retain
   their existing traversal error.

## Consequences

Consumers can implement workspace, URI, archive, editor-buffer, and platform-specific directory
policy with full source-aware graph context. Descendants receive a resolved/defaulted parent when
available, or `None` after deferred or unresolved policy, so they may independently resolve opaque
or absolute declarations.

The plan does not settle environment precedence, interpolation, resource path resolution, project
naming, provider behavior, or which paths are local files.

## Alternatives considered

### Resolve relative paths in ComposeLens

Rejected because joining requires a caller-owned interpretation of the raw declaration and would
silently make URI, archive, and editor-buffer workflows local-path-specific.

### Use loader-time project-directory policy

Rejected because it would add policy to the authorization/I/O boundary and prevent callers from
inspectably planning already loaded graphs.

### Return arbitrary resolver error messages

Rejected because caller-controlled error text could expose path or credential-like data through
diagnostics and logs.
