# Project structure

ComposeLens is one library crate with strong internal module boundaries. The crate foundation exists; entries marked `planned` are created with their first behavior and tests. Separate published crates are not justified until an independent consumer and release need are demonstrated.

```text
compose-lens/
├── Cargo.toml
├── Cargo.lock
├── rust-toolchain.toml
├── rustfmt.toml
├── clippy.toml
├── deny.toml
├── .cargo/
│   └── config.toml        # canonical Cargo aliases
├── AGENTS.md
├── README.md
├── LICENSE
├── src/
│   ├── lib.rs
│   ├── source/             # source identifiers, spans, and line/column lookup
│   ├── syntax/             # loss-aware YAML document and parser diagnostics
│   ├── model/              # planned: native typed Compose document
│   ├── loader/             # planned: files, includes, origins, and project discovery
│   ├── merge/              # planned: multi-file composition
│   ├── interpolation/      # planned: expressions and environment providers
│   ├── profiles/           # planned: profile selection and implementation profiles
│   ├── validation/         # planned: validation rules and capability classification
│   ├── render/             # planned: preservation and canonical rendering
│   └── diagnostic/         # stable codes, severities, labels, and notes
├── tests/
│   ├── README.md           # suite ownership and introduction rules
│   ├── repository_policy.rs # fixture and workflow-pin enforcement
│   ├── syntax.rs           # preservation and malformed-input behavior
│   ├── roundtrip.rs        # parse/render/parse stability
│   └── support/            # private repository-test helpers
├── fixtures/
│   ├── README.md           # fixture location and safety rules
│   ├── syntax/             # authored syntax and recovery cases
│   └── roundtrip/          # authored stability cases
├── docs/
│   ├── fixture-format.md   # versioned fixture manifest contract
│   └── research/           # versioned technical evaluations
└── .github/
    ├── renovate.json
    └── workflows/
        └── ci.yml
```

## Module placement rules

| Concern                                | Module          |
| -------------------------------------- | --------------- |
| Text positions and source identity     | `source`        |
| YAML structure and spelling            | `syntax`        |
| Compose-native types                   | `model`         |
| File access and path origins           | `loader`        |
| Overlay semantics                      | `merge`         |
| Variable expressions and substitution  | `interpolation` |
| Compose service profiles               | `profiles`      |
| Implementation support and correctness | `validation`    |
| YAML output                            | `render`        |
| Stable diagnostic codes                | `diagnostic`    |

Do not place file access in syntax parsing, automatic interpolation in deserialization, or BoxFerry conversion types in the native model.
