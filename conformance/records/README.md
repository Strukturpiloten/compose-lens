# Reviewed conformance records

Each dated target/probe directory is one reviewed observation from an exact matrix run. Generated
output is not evidence until it has been reviewed and the corresponding matrix run links it with
`status = "observed"` and a repository-relative `record` path.

Records retain `record.toml`, version-command stdout and stderr, and probe-command stdout and
stderr. Reviewers must reject credentials, tokens, private registry names, private paths, and any
other sensitive or identifying runtime data before committing a record. Declared deterministic
normalization may replace local repository and acquisition roots with `<repository>` and
`<acquisition>`; semantic provider output must not otherwise be edited.
