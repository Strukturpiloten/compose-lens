# ADR 0023: include-aware config and secret file path resolution

- Status: accepted
- Date: 2026-08-14
- Extends: [ADR 0022](0022-caller-owned-include-project-directory-plans.md)

## Context

Include composition selects config and secret definitions from distinct project occurrences. Their
relative `file` paths use that occurrence's project directory, not an including root fallback.
Existing root-project path resolution has one mandatory base and therefore cannot represent a
deferred or unresolved included-project directory honestly.

Include traversal and composition are authored and uninterpolated. Composition retains occurrence
and source evidence but not the merged scalar sensitivity marker, so this slice cannot make an
interpolated-path or sensitivity-classification claim.

## Decision

1. `resolve_included_resource_paths` is a separate, I/O-free operation over an
   `IncludeCompositionResult`, an `IncludeProjectDirectoryPlan`, and explicit `PathContext`.
2. It considers only selected top-level config and secret `file` values from the root composition.
   Definitions rejected by include conflict handling are not resolved.
3. Occurrence index and caller identity must match the directory-plan entry. A mismatch emits
   `compose.include.resource-path-plan-mismatch`; a matching entry without an effective base emits
   `compose.include.resource-path-base-unavailable`. Neither condition falls back to another base.
4. Relative, Unix absolute, Windows drive, UNC, and home-relative spellings reuse the established
   lexical categories. Resolution never canonicalizes, follows symlinks, checks existence, or reads
   a file.
5. Results retain authorized getters for raw values, source spans, purpose, occurrence evidence,
   optional base, and optional lexical result. Debug output always redacts identity and path text.

## Consequences

Consumers can resolve selected included config and secret files with the correct authorized
occurrence base while preserving incomplete/deferred outcomes. The API deliberately remains an
authored, uninterpolated view and exposes no `is_sensitive` claim.

Service binds, build contexts and Dockerfiles, `env_file`, `label_file`, `extends.file`, develop
watch paths, include-declaration loading, URI/non-local policy, interpolation, composed rendering,
provider behavior, and filesystem access remain separate work.

## Alternatives considered

### Reuse the root `ResolvedHostPath`

Rejected because its mandatory origin cannot represent a deferred included-project base.

### Fall back to the including project's directory

Rejected because it would silently resolve against the wrong Compose project.

### Resolve every path-bearing field together

Rejected because Compose fields use different bases and accept different non-local forms.
