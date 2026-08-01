# ADR 0012: repository-side exact-version conformance harness

- Status: accepted
- Date: 2026-07-31

## Context

ComposeLens compatibility rules need evidence from released Docker Compose and `podman-compose`
providers, and later from Docker Engine and Podman runtimes. Invoking those tools inside the
library would introduce ambient state and side effects into otherwise pure processing. Treating a
moving latest release or an unreviewed command result as support would make claims irreproducible.

Provider output can contain resolved environment values, host paths, or other sensitive data.
Runtime probes can additionally create containers, networks, volumes, files, and SELinux label
changes. Conformance therefore needs a stronger acquisition, isolation, retention, and review
boundary than ordinary unit tests.

## Decision

1. ComposeLens remains a pure library and never invokes a provider or runtime.
2. The repository owns a separate, ignored integration-test harness for explicit conformance
   collection. Ordinary Cargo and CI test commands do not execute it.
3. A versioned TOML matrix identifies exact provider releases, immutable release URLs, authored
   fixtures, arguments, and planned or observed state. Moving aliases such as `latest` are not
   valid matrix versions.
4. Every target/probe pair is explicit. A planned entry is not evidence and cannot carry a record.
   An observed entry must link an existing reviewed record.
5. The runner requires an absolute caller-selected launcher, caller-verified artifact URL and
   SHA-256 metadata, a full fixture Git revision, an explicit platform description, an explicit
   executable search path, and a new absolute result directory. It verifies provider-reported
   version text exactly before probing.
6. Provider commands receive a cleared environment with isolated home, config, cache, and runtime
   directories. Authored fixtures contain no interpolation inputs or credentials.
7. Raw stdout, stderr, exit status, command arguments, exact version, release and artifact URLs,
   fixture identity and revision, artifact checksum metadata, platform description, working
   directory, and complete controlled environment are retained. A rejected configuration is an
   observation rather than a harness failure.
8. Generated results are unreviewed. Human review for identity, reproducibility, secrets, and
   interpretation is required before committing a record or citing it from a compatibility rule.
9. Runtime-effect probes require an additional design for isolation, cleanup, privilege, and host
   feature detection before they are added to the harness.

## Consequences

- Provider behavior can be measured without coupling runtime access to the public crate.
- Matrix completeness and metadata errors fail ordinary pure tests even though external commands
  remain opt-in.
- Initial matrix entries remain planned until trusted artifacts are executed and reviewed.
- The runner records caller-supplied artifact checksums but does not itself acquire or authenticate
  binaries; acquisition automation remains open work.
- Runtime semantics such as SELinux relabeling cannot be inferred from provider-config results.

## Alternatives considered

### Invoke installed tools from ordinary tests

Rejected because developer and CI machines have different tools, versions, environment variables,
runtime state, and privileges.

### Put conformance invocation in the library

Rejected because it violates the library's pure side-effect boundary and makes a parser consumer
responsible for Docker or Podman process policy.

### Treat release documentation as conformance

Rejected because documentation may establish intended syntax or version boundaries but does not
prove observable behavior for one provider/runtime/platform combination.

### Record only normalized output

Rejected because diagnosis and evidence review also require exact commands, raw output, exit state,
tool identity, fixture identity, and environment assumptions.
