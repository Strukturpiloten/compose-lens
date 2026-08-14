# ADR 0018: structured final Compose schema keys

- Status: accepted
- Date: 2026-08-14

## Context

The remaining closed-schema Compose keys include nested forms whose meaning is lost by a raw field
reference: include declarations, model definitions and bindings, GPU selectors, and development
watch rules. They must remain source-aware without making parsing load files, interpolate the host
environment, inspect devices, or execute/watch anything.

Current Compose prose and its JSON schema differ in limited places: long `include` prose requires
`path`, while the schema does not; service-model null bindings do not define a default; GPU
selector exclusivity is not defined. ComposeLens needs a stable, non-invented boundary for these
cases.

## Decision

1. Model every supported short and long form as a Compose-native authored type and expose the
   corresponding effective value through `ProjectView` with outer merge provenance.
2. Retain malformed, extension, and unknown nested fields as source references and emit
   field-specific diagnostics. Parsing and project-view construction remain free of file,
   environment, provider, runtime, device, execution, and watch side effects.
3. Enforce the documented required `include.path`, required model `model`, and required
   `develop.watch` action/path members. Keep service-model null bindings and GPU selector
   count/device-ID coexistence as retained evidence; do not synthesize a provider default or make
   a device allocation decision.
4. Validate service model names against top-level model definitions as an ordinary local reference.
   This reports absence but never resolves a model or contacts a provider.
5. Generated Compose output is limited to values whose YAML-safe representation is explicit and
   parse-back validated. It does not imply support for every authored nested form.
6. Treat modern boolean/expression `external` and legacy `external: { name: ... }` as distinct
   source-aware forms. Warn on legacy objects, label both sources when their name conflicts with
   modern `name`, and retain malformed/unknown nested evidence. Resource and develop-exec metadata
   is parsing-only: no driver, template, file, environment, user, path, watcher, or command is
   invoked. Config and secret metadata receives no generated API.

## Consequences

- Consumers receive structured fields and actionable diagnostics instead of opaque syntax-only
  evidence for the closed schema boundary.
- ComposeLens records schema/prose and provider-semantic gaps rather than concealing them through
  normalization.
- Loading included projects, files, environments, model providers, device selection, and develop
  watch execution remain application responsibilities.
- The current schema deprecates legacy external-name objects for networks, volumes, and configs;
  a reviewed Docker Compose 5.4.0 observation warns for secrets too. The model is consistent for
  all four resources but records the divergence instead of asserting provider equivalence.

## Alternatives considered

- Preserve these keys only as unknown syntax. Rejected because it denies consumers nested shape,
  provenance, and local model-reference validation.
- Resolve includes, environment files, models, or devices during parsing. Rejected because it
  breaks explicit processing and side-effect boundaries.
- Infer model environment-variable defaults or GPU allocation semantics. Rejected because the
  current specification does not provide stable evidence for either.
