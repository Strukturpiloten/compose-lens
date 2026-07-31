# ADR 0007: explicit post-merge processing views

- Status: accepted
- Date: 2026-07-31

## Context

A merged Compose project still does not have one universal semantic interpretation. Active
profiles are caller input. Relative paths need an explicit project origin, while home-relative
paths need caller context. References can exist syntactically but be unavailable because a target
is absent or profile-disabled. Defaults depend on the selected platform and, eventually, a specific
implementation and version.

Applying these rules directly to the merged tree would erase omissions, inactive services, raw
paths, and invalid references that BoxFerry needs for accurate conversion diagnostics. Reading the
process environment, home directory, or file system from the library would make analysis
non-reproducible.

## Decision

1. Profile selection, path resolution, reference validation, and default resolution are explicit,
   independent operations over `MergedProject`.
2. `ProfileRequest` contains only caller-supplied active profiles or an explicit all-profiles flag.
   The library does not infer Docker Compose command-line service targeting.
3. `ProfileSelection` retains every service decision and an exact merged-project snapshot. A
   selection from another project is rejected even if source identifiers were reused.
4. Path resolution retains raw text, source span, purpose, path class, project origin, and an
   optional lexical result. It uses the first loaded document's base for merged-file relative paths.
   Home expansion requires explicit `PathContext`. No canonicalization or file access occurs.
5. Reference validation examines selected services and records every supported edge as found,
   missing, or inactive. It emits structured errors without modifying the target name or source.
6. Default resolution emits `DefaultRequest` values to a caller-owned `DefaultProvider`.
   `NoDefaults` and platform-aware `ComposeDefaults` are supplied policies. Applied defaults form a
   decision overlay and never replace omissions in the merged tree.
7. Sensitive interpolated path, reference, and default data is available only through explicit
   accessors and is redacted from `Debug` output and diagnostic messages.
8. Implementation/version compatibility remains a later validation layer; specification defaults
   are not treated as proof that every runtime implements them.

## Consequences

- BoxFerry can explain inactive, missing, defaulted, and path-origin decisions independently.
- Tests can reproduce processing without ambient environment, home-directory, or file-system state.
- Callers may choose strict defaults, implementation-specific defaults, or no defaults.
- A `ProfileSelection` owns a merged-project snapshot. This increases memory use but prevents a
  semantically unrelated project from accepting the selection because source IDs happen to match.
- Runtime command planning and Compose `include` path scopes require separate future inputs rather
  than hidden behavior in these operations.

## Alternatives considered

### Mutate one fully normalized project

Rejected because it loses omissions, raw paths, inactive services, and unsupported references that
are necessary for diagnostics and preservation-oriented rendering.

### Read process and file-system context automatically

Rejected because results would depend on the machine running ComposeLens and tests would not be
reproducible.

### Identify selections only by source IDs

Rejected because callers control those identifiers and can legally reuse them for a different
loaded project.
