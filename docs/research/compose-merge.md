# Compose multi-file merge evidence

- Researched: 2026-07-31
- Scope: the next ComposeLens Phase 3 merge implementation
- Status: specification behavior has authored executable coverage; runtime conformance fixtures remain open

## Authoritative behavior

The Compose Specification and Docker documentation currently describe these rules:

- Files are processed in caller-supplied order. Later files extend, add to, or override earlier
  files.
- Interpolation is applied to each file before merge.
- Relative paths in a multi-file project are evaluated from the first, or base, Compose file. When
  standard input is used, the current working directory supplies that explicit context.
- Mappings merge recursively. A later scalar replaces an earlier scalar.
- Sequences append by default.
- `command`, `entrypoint`, and `healthcheck.test` are shell-command exceptions that replace rather
  than append.
- `ports`, `volumes`, `secrets`, and `configs` use field-specific uniqueness keys. A matching entry
  is merged; a non-matching entry is appended.
- `!reset` clears a value, while `!override` bypasses normal merge behavior and replaces it. Docker
  documents `!override` as requiring Docker Compose 2.24.4 or later.

Sources:

- [Compose Specification: merge](https://github.com/compose-spec/compose-spec/blob/main/13-merge.md)
- [Docker Compose file merge reference](https://docs.docker.com/reference/compose-file/merge/)
- [Docker multi-file merge guide](https://docs.docker.com/compose/how-tos/multiple-compose-files/merge/)
- [Compose Specification: interpolation](https://github.com/compose-spec/compose-spec/blob/main/12-interpolation.md)

## Required ComposeLens representation

The merge result cannot be a plain recursive YAML overlay. It must record enough provenance to
explain whether each value was retained, replaced, recursively merged, appended, reset, or
overridden. Unknown fields must follow generic mapping or sequence behavior without disappearing.

Field-aware behavior must initially cover:

| Field | Required rule |
| --- | --- |
| ordinary mappings | recursively merge by key |
| ordinary sequences | append |
| `command`, `entrypoint`, `healthcheck.test` | replace |
| `environment`, `labels` | merge by variable or label key across mapping/list forms |
| service `volumes`, `devices` | unique by container target |
| service `configs`, `secrets` | unique by container target |
| service `ports` | unique by IP, target, published port, and protocol |
| tagged `!reset` value | clear to the field's default or empty value |
| tagged `!override` value | replace without normal merge |

Short and long forms must remain distinguishable after a uniqueness match. For example, merging a
volume entry by target must not silently discard whether SELinux behavior was authored using short
syntax.

## Test plan

1. Completed: authored fixtures for recursive mapping, scalar replacement, and sequence append.
2. Completed: assertions for every shell-command exception and implemented uniqueness key.
3. Completed: mixed map/list environment and label forms plus mixed short/long unique resources.
4. Completed: reset and override fixtures with the documented Docker Compose 2.24.4 boundary in
   fixture metadata.
5. Completed: source provenance, deterministic ordering, interpolation ordering, YAML merge keys,
   unknown fields, and sensitive debug redaction.
6. Open: capture versioned Docker Compose and Podman Compose oracle output for the same corpus.

## Explicit non-equivalence

Compose `include` does not share all multi-file merge path rules: included projects retain their own
project directories before they are copied into the including model. Include loading therefore
must not be implemented as another ordinary override file.
