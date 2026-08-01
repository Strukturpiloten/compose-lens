# ADR 0014: Issue-derived native model expansion

- Status: accepted
- Date: 2026-08-01

## Context

The Podlet and `compose_spec_rs` issue review identified recurring inputs that the first typed
boundary exposed only as unknown fields: `extra_hosts`, user/group values, unlimited `ulimits`,
health-gated dependencies, anonymous container volume targets, Podman user namespaces, and the
large `build` and `deploy` sections.

These inputs combine three different concerns. Compose syntax may have multiple non-interchangeable
forms, container paths may target a different operating system than the ComposeLens process, and
implementation-specific values need provider/runtime evidence rather than stricter parsing.

## Decision

ComposeLens expands its native model under the following rules:

1. `extra_hosts`, `depends_on`, and `build` retain their scalar/list/mapping alternatives. Short
   host entries keep the complete scalar, `=` versus `:`, bracketed versus unbracketed IPv6, and
   implementation tokens.
2. `user` keeps its complete scalar. Optional user/group helpers split only at a colon outside a
   Compose interpolation expression and classify names and numeric IDs without resolving either.
3. `ulimits` retains single and soft/hard forms. `-1` is represented explicitly as unlimited;
   interpolation remains deferred.
4. Container-side mount targets use ComposeLens-owned lexical `ContainerPath` rules. They never use
   the current host's `std::path::Path` behavior. Host bind sources remain in the separate explicit
   host-path resolution stage.
5. Health checks and dependency conditions are typed. Document validation checks local service
   targets. Post-merge reference validation checks the selected project view. A missing Compose
   health check is a warning because image metadata may provide one; an explicitly disabled health
   check used by a required `service_healthy` edge is an error. `required: false` keeps the finding
   but lowers unavailable dependency diagnostics to warnings.
6. Podman-specific `userns_mode` values are typed because the licensed
   `Strukturpiloten/typo3-container` regression is a demonstrated consumer. `host-gateway` is typed
   because migration issues and Podman 5.4 documentation demonstrate it. Compatibility findings
   retain official evidence, but provider pass-through remains unknown until exact conformance
   observations exist.
7. `build` and `deploy` are not assigned one aggregate support decision. Their current
   specification subfields receive stable typed identities and source references so BoxFerry can
   classify or convert each field independently. Nested value semantics remain losslessly
   available through the syntax document until a consumer justifies a deeper type.

## Consequences

- Windows hosts cannot reinterpret `/project/node_modules` as a named host resource.
- Short and long host mappings, unlimited limits, and platform-specific tokens remain available to
  converters without normalization loss.
- Dependency validation is useful without reading image metadata or invoking a runtime.
- Build and deploy coverage can grow field by field without breaking one coarse support flag.
- Compatibility evidence distinguishes runtime capability from unverified Compose-provider
  pass-through.

## Alternatives considered

- Normalizing all alternate forms into one structure was rejected because spelling and provider
  behavior can affect conversion.
- Using host-native path APIs for both mount sides was rejected because host and container
  platforms are independent.
- Rejecting a healthy dependency with no Compose health check was rejected because the image can
  define the health check.
- Deeply typing every nested build and deploy value immediately was rejected because it would add
  a broad, unvalidated API without a current conversion consumer.
