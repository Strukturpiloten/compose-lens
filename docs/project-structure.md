# Target project structure

ComposeLens begins as one public crate with strong internal module boundaries. Separate published crates are not justified until an independent consumer and release need are demonstrated.

```text
compose-lens/
├── Cargo.toml
├── Cargo.lock
├── rust-toolchain.toml
├── AGENTS.md
├── README.md
├── LICENSE
├── src/
│   ├── lib.rs
│   ├── source/             # source identifiers, spans, and diagnostics
│   ├── syntax/             # YAML-facing loss-aware representation
│   ├── model/              # native typed Compose document
│   ├── loader/             # files, includes, origins, and project discovery
│   ├── merge/              # multi-file composition
│   ├── interpolation/      # expressions and environment providers
│   ├── profiles/           # profile selection and implementation profiles
│   ├── validation/         # validation rules and capability classification
│   ├── render/             # preservation and canonical rendering
│   └── diagnostic/         # structured errors and warnings
├── tests/
│   ├── syntax/
│   ├── conformance/
│   ├── implementations/
│   ├── roundtrip/
│   └── real-world/
├── fixtures/
│   └── README.md           # fixture provenance and licensing rules
├── docs/
└── .github/
    ├── workflows/
    └── ISSUE_TEMPLATE/
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
