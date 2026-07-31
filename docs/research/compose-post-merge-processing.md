# Compose post-merge processing evidence

- Evaluated: 2026-07-31
- Scope: Compose Specification `main` and current Docker Compose documentation
- Implementation status: authored library fixtures; runtime matrices remain open

## Sources

- [Compose Specification](https://github.com/compose-spec/compose-spec/blob/main/spec.md)
- [Docker Compose profiles](https://docs.docker.com/compose/how-tos/profiles/)
- [Docker Compose multi-file merge](https://docs.docker.com/compose/how-tos/multiple-compose-files/merge/)
- [Docker Compose services reference](https://docs.docker.com/reference/compose-file/services/)
- [Docker Compose configs reference](https://docs.docker.com/reference/compose-file/configs/)
- [Docker Compose secrets reference](https://docs.docker.com/reference/compose-file/secrets/)

These living sources are recorded with an evaluation date. Compatibility claims require pinned
implementation versions and runtime evidence in addition to this specification-oriented baseline.

## Profile findings

- Services without `profiles` are always enabled. A restricted service is enabled when at least one
  assigned profile is active.
- Profile names follow `[a-zA-Z0-9][a-zA-Z0-9_.-]+`; the `+` makes one-character names invalid.
- Top-level resources are not profile-gated. Profiles select services, not the entire document.
- Docker Compose can auto-start profiles for a service explicitly targeted on its command line.
  ComposeLens does not receive a command invocation, so its selection API models only explicit
  active profiles and the explicit all-profiles case.
- A reference to a profile-disabled service does not itself enable that service. The selected model
  can therefore be invalid and needs a diagnostic rather than hidden activation.

## Path findings

- For an ordered set of merged Compose files, relative paths are evaluated from the first Compose
  file's parent directory, including values authored in later override files.
- `include` establishes a different project/path scope and is not implemented by this milestone.
- Host-path classification and origin attachment do not require file-system access. Canonicalizing
  or checking path existence would add host-specific behavior and is therefore left to callers.
- Home-relative expansion is useful to conversion tools but needs explicit caller context; it must
  not silently read the process home directory.

## Reference findings

- Named volumes, configs, and secrets used by a service require corresponding top-level resources.
- Service network attachments require a declared network, except the implicit `default` network.
- Service edges such as `depends_on` and `service:` namespace modes can name services that profiles
  exclude. Missing and inactive are separate conditions with different remediation.
- External-file `extends` has its own loading scope; the first reference validator covers only local
  `extends.service` edges.

## Defaults in the initial provider

| Omission | Specification-oriented value |
| --- | --- |
| Service networks absent or empty | attach to `default` |
| Top-level `default` network absent when needed | implicit network named `default` |
| Port protocol | `tcp` |
| Port publication mode | `ingress` |
| Volume access | read-write (`read_only = false`) |
| Config short target on Linux | `/<source>` |
| Config short target on Windows | `C:\\<source>` |
| Config or secret mode | `0444` |
| Secret target | source name, mounted by the runtime under its secrets directory |
| Restart policy | `no` |

The table describes requests that `ComposeDefaults` may answer. The merged source continues to
represent each field as omitted. Long-form explicit values always take precedence, and future
compatibility profiles may decline or replace a default based on versioned evidence.

## Authored evidence

- `fixtures/processing/profile-selection` covers no-profile, explicit-profile, all-profile, and
  merge-reset behavior.
- `fixtures/processing/project-resolution` covers selected service paths, explicit home context,
  first-file path origin, found/missing/inactive references, documented defaults, and a no-defaults
  policy.
- `tests/project_processing.rs` rejects a selection belonging to another merged project, including
  another project reusing the same source ID.

## Remaining evidence

- Pin Docker Compose and Podman Compose versions and run the fixtures through their config commands.
- Add Windows-container conformance for config targets.
- Add `include`, external `extends`, and command-target profile inputs only after their loading and
  provenance models are designed.
- Keep implementation differences in compatibility profiles rather than changing the neutral
  Compose processing baseline.
