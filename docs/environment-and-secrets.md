# Environment and secret values

Compose environment data has four distinct states. Keeping them separate prevents a parser from
silently reading the host or turning an unavailable value into an authored empty string.

| State               | ComposeLens representation                            | Access performed            |
| ------------------- | ----------------------------------------------------- | --------------------------- |
| Authored syntax     | `Environment`, `EnvironmentFile`, `SecretDefinition`  | None                        |
| Interpolated scalar | `InterpolationResult` with substitution provenance    | Only `EnvironmentProvider`  |
| Effective project   | `ProjectEnvironment` and top-level secret definitions | None                        |
| Authorized value    | `ServiceEnvironmentResolution` or `SecretResolution`  | Only caller-owned providers |

Parsing, merging, project-view construction, canonical rendering, and generated rendering never
read `.env`, `env_file`, process-environment, or secret files. Applications retain the earlier
source-aware representation when they do not request resolution.

## Environment resolution

`resolve_service_environment` requires two explicit inputs:

- an `EnvironmentProvider` for key-only host lookups and interpolation;
- an `EnvironmentFileProvider` that may supply UTF-8 content for each exact request.

The file provider receives the authored path, `required` choice, `format` choice, source span, and
path-sensitivity flag. It may return content, report the file unavailable, or deny the request.
ComposeLens does not normalize or open the path.

Resolution applies environment files in declaration order and then applies service `environment`.
The returned entries are sorted by key for deterministic downstream rendering. These states remain
different:

- `KEY=` and `KEY: ""` are concrete empty strings;
- `KEY` and `KEY: null` request a host value and remain `Unset` when none was supplied;
- single-quoted file values are literal;
- double-quoted and unquoted file values apply explicit interpolation, including default and
  escaped-dollar behavior;
- `format: raw` keeps the right-hand side literal and does not interpolate it.

Environment-file contents and host values are sensitive only when the provider marks them so.
Sensitive input and result debug output is redacted. Diagnostics identify the declaration and error
category without copying file lines or values.

## Secret resolution

`resolve_project_secrets` turns each unambiguous top-level secret definition into a `SecretRequest`.
The request names the native source category: file, host environment, external platform secret, or
opaque driver. Only a caller-owned `SecretProvider` can return a payload.

`SecretValue` is always redacted in `Debug`; `expose()` is the explicit payload boundary. A missing,
ambiguous, unavailable, or denied source produces a source-aware diagnostic. ComposeLens never
reads a secret file, process variable, platform store, or driver itself.

## Rendering and BoxFerry

Canonical rendering retains authored/effective mapping and sequence order. It does not resolve
values. Generated Compose output sorts environment entries lexicographically by key; equal keys use
stable insertion order so list-form last-value semantics remain unchanged.

BoxFerry should normally map the source-aware project view into its neutral model. It should request
environment or secret payload resolution only after its caller explicitly authorizes the matching
provider. Diagnostic/support-bundle code should retain source spans, source categories, empty/unset
state, and sensitivity, but must not serialize protected values by default.

[ADR 0025](decisions/0025-caller-authorized-environment-secret-resolution.md) defines this boundary.
