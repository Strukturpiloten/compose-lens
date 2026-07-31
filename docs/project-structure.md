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
│   ├── source/             # planned: source identifiers, spans, and diagnostics
│   ├── syntax/             # planned: YAML-facing loss-aware representation
│   ├── model/              # planned: native typed Compose document
│   ├── loader/             # planned: files, includes, origins, and project discovery
│   ├── merge/              # planned: multi-file composition
│   ├── interpolation/      # planned: expressions and environment providers
│   ├── profiles/           # planned: profile selection and implementation profiles
│   ├── validation/         # planned: validation rules and capability classification
│   ├── render/             # planned: preservation and canonical rendering
│   └── diagnostic/         # planned: structured errors and warnings
├── tests/                  # planned with the first implemented behavior
│   ├── syntax/
│   ├── conformance/
│   ├── implementations/
│   ├── roundtrip/
│   └── real-world/
├── fixtures/               # planned with the first external fixture
│   └── README.md           # fixture provenance and licensing rules
├── docs/
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
