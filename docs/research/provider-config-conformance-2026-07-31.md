# Provider-config conformance — 2026-07-31

## Scope

ComposeLens executed 48 provider-only `config` observations: six exact providers against one combined probe and seven minimal feature probes. No Docker Engine, Podman runtime, registry, image, container, network, or volume was contacted. The reviewed records are retained under [`conformance/records/provider-config-2026-07-31/`](../../conformance/records/provider-config-2026-07-31/).

The Docker Compose binaries were verified against their official release checksum manifests. The two `podman-compose` wheels were verified against PyPI's published SHA-256 metadata and executed with Python 3.13.14, `python-dotenv` 1.2.2, and PyYAML 6.0.3. Every record includes the exact artifact and launcher hashes, fixture hash, arguments, cleared environment, platform, exit state, normalized stdout and stderr, outcome, and review summary.

## Reviewed results

| Provider               | Tag + digest | Short SELinux   | Long SELinux    | `!reset` | `!override`                                | `x-` fields | Unknown field |
| ---------------------- | ------------ | --------------- | --------------- | -------- | ------------------------------------------ | ----------- | ------------- |
| Docker Compose 2.24.3  | retained     | config accepted | config accepted | applied  | accepted but appended instead of replacing | retained    | rejected      |
| Docker Compose 2.24.4  | retained     | config accepted | config accepted | applied  | applied                                    | retained    | rejected      |
| Docker Compose 2.40.3  | retained     | config accepted | config accepted | applied  | applied                                    | retained    | rejected      |
| Docker Compose 5.3.1   | retained     | config accepted | config accepted | applied  | applied                                    | retained    | rejected      |
| `podman-compose` 1.3.0 | retained     | config accepted | config accepted | rejected | rejected                                   | retained    | retained      |
| `podman-compose` 1.5.0 | retained     | config accepted | config accepted | rejected | applied                                    | retained    | retained      |

“Config accepted” for an SELinux form is deliberately weaker than runtime support. It proves only that the provider accepted and rendered the request. Relabel effects require an enforcing SELinux host and one of the separately planned runtime contexts.

## Important boundary

Docker Compose 2.24.3 did not reject the `!override` tag. Its command succeeded, but the output contained both the base and override ports. Docker Compose 2.24.4 retained only the replacement port. The observed behavioral boundary therefore agrees with Docker's documented 2.24.4 minimum while explaining why syntax acceptance alone is an unsafe test.

For `podman-compose`, 1.3.0 rejected both custom merge tags through PyYAML. Version 1.5.0 applied `!override` correctly but still failed while normalizing `!reset`. These results are scoped to the exact recorded Python dependency environment.

## Promotion policy

Built-in compatibility rules use these results only for the exact observed provider versions. An unobserved patch, minor, or major version remains governed by specification/documentation evidence or classified as unknown/implementation-specific. Provider config acceptance is never promoted into a runtime-effect claim.
