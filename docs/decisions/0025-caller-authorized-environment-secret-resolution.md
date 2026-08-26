# ADR 0025: caller-authorized environment and secret resolution

- Status: accepted
- Date: 2026-08-26

## Context

Compose can obtain values from authored service environment entries, interpolation inputs,
environment files, host environment variables, secret files, platform secrets, and secret drivers.
Parsing or project-view construction cannot read these sources implicitly without making the same
document host-dependent and risking secret disclosure. The Podman-to-neutral-to-Compose route also
needs deterministic output while preserving unset, empty, source, and sensitivity evidence.

## Decision

1. Authored and effective models remain deferred, source-aware native Compose data. Existing parse,
   interpolation, merge, project-view, validation, and rendering operations perform no new I/O.
2. `EnvironmentProvider` remains the only host-value input for interpolation and key-only
   environment entries. Sensitive provider values redact their own and derived debug output.
3. `EnvironmentFileProvider` is an explicit content-acquisition boundary. Requests carry the exact
   path, source span, `required`, parser mode, and sensitivity; ComposeLens never opens paths.
4. Authorized service-environment resolution applies files in declaration order, then explicit
   service entries. It distinguishes concrete empty strings from unavailable key-only values and
   retains final origin evidence.
5. Compose-mode file values support quote, escape, and interpolation states. Raw mode retains the
   right-hand side literally. Malformed input yields value-free source diagnostics and partial data.
6. `SecretProvider` is the only secret-payload boundary. Requests preserve file, environment,
   external, or driver source identity. `SecretValue` always redacts debug output and requires an
   explicit accessor.
7. Authorized resolution results use deterministic key order. Generated Compose environment output
   also sorts by key; equal-key entries retain insertion order. Canonical rendering continues to
   preserve authored/effective order.
8. BoxFerry may consume source-aware deferred state without payload authorization. Payload resolution
   is a separate caller decision and protected values remain excluded from diagnostics by default.

## Consequences

- Applications can provide filesystem, process-environment, platform, vault, or test adapters
  without granting those capabilities to ComposeLens itself.
- Tests can cover file-backed and secret-backed behavior using synthetic providers.
- Unset, empty, literal, interpolated, raw, missing, and denied states remain observable.
- Deterministic generated output does not reorder duplicate equal-name entries relative to each
  other, preserving list-form last-value semantics.
- Environment-file parsing is intentionally bounded to the documented native contract; future
  provider-specific behavior requires versioned evidence rather than implicit host execution.

## Alternatives considered

### Read `.env`, `env_file`, secret files, or process environment automatically

Rejected because parsing would become host-dependent and secret-bearing I/O would be hidden.

### Store secret payloads in the normal project view

Rejected because project views are routinely inspected, debugged, serialized, and included in
support evidence. Source definitions and payload authorization are separate concerns.

### Sort canonical authored environment collections

Rejected because canonical rendering promises retained effective order and syntax. Sorting belongs
only to authorized semantic results and newly generated output.
