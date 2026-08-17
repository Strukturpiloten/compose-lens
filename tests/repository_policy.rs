//! Executable repository and fixture-contract checks.

mod support;

use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

use serde_json::Value;
use sha2::{Digest, Sha256};

const FIXTURE_SUITES: &[&str] = &[
    "syntax",
    "typed-model",
    "processing",
    "roundtrip",
    "conformance",
    "real-world",
];

#[test]
fn github_actions_are_immutable_and_versioned() -> Result<(), String> {
    support::validate_action_pins(&repository_root())
}

#[test]
fn repository_supply_chain_has_single_sources_and_immutable_pins() -> Result<(), String> {
    support::validate_repository_supply_chain(&repository_root())
}

#[test]
fn public_api_compatibility_runs_in_ci_and_release() -> Result<(), String> {
    const ACTION: &str = "obi1kenobi/cargo-semver-checks-action@6b69fcf40e9b5fb17adeb57e4b6ecd020649a239 # v2.9";
    const CONFIGURATION: &str = "package: compose-lens";

    for workflow_name in ["ci.yml", "release.yml"] {
        let workflow_path = repository_root().join(".github/workflows").join(workflow_name);
        let workflow = fs::read_to_string(&workflow_path)
            .map_err(|error| format!("failed to read {}: {error}", workflow_path.display()))?;

        let configured_action = format!("uses: {ACTION}\n        with:\n          {CONFIGURATION}");
        if workflow.matches(ACTION).count() != 1
            || workflow.matches(&configured_action).count() != 1
            || workflow.contains("release-type:")
        {
            return Err(format!(
                "{workflow_name} must contain one version-derived cargo-semver-checks action for compose-lens"
            ));
        }
    }

    Ok(())
}

#[test]
fn coverage_ratchet_runs_in_ci_and_release() -> Result<(), String> {
    const INSTALL: &str = "cargo install --locked --version 0.8.7 cargo-llvm-cov";
    const CLEAN: &str = "cargo llvm-cov clean --locked";
    const COMMAND: &str = "cargo llvm-cov --locked --no-clean --workspace --all-features --all-targets --summary-only\n          --fail-under-regions 88 --fail-under-functions 87 --fail-under-lines 89";

    for workflow_name in ["ci.yml", "release.yml"] {
        let workflow_path = repository_root().join(".github/workflows").join(workflow_name);
        let workflow = fs::read_to_string(&workflow_path)
            .map_err(|error| format!("failed to read {}: {error}", workflow_path.display()))?;

        for required in ["rustup component add llvm-tools-preview", INSTALL, CLEAN, COMMAND] {
            if workflow.matches(required).count() != 1 {
                return Err(format!(
                    "{workflow_name} must contain one pinned ComposeLens coverage guard `{required}`"
                ));
            }
        }
    }

    Ok(())
}

#[test]
fn ci_workflow_enforces_portability_and_an_actionable_pr_gate() -> Result<(), String> {
    let workflow = read_repository_file(".github/workflows/ci.yml")?;

    for required in [
        "  portability:\n    name: Portability (macOS)",
        "runs-on: macos-14",
        "run: cargo ci-check",
        "run: cargo ci-test",
        "  pr-gate:\n    name: PR gate\n    if: always()",
        "needs: [rust, msrv, dependencies, api, documentation, coverage, portability]",
    ] {
        if !workflow.contains(required) {
            return Err(format!("CI workflow is missing contract `{required}`"));
        }
    }

    for (job_name, result_variable, needs_job) in [
        ("Rust quality", "RUST_RESULT", "rust"),
        ("MSRV", "MSRV_RESULT", "msrv"),
        ("Dependency and license policy", "DEPENDENCIES_RESULT", "dependencies"),
        ("Public API compatibility", "API_RESULT", "api"),
        ("Documentation", "DOCUMENTATION_RESULT", "documentation"),
        ("Coverage ratchet", "COVERAGE_RESULT", "coverage"),
        ("macOS portability", "PORTABILITY_RESULT", "portability"),
    ] {
        let required = format!("{result_variable}: ${{{{ needs.{needs_job}.result }}}}");
        if !workflow.contains(&required) {
            return Err(format!("PR gate does not expose a result variable for `{job_name}`"));
        }
    }

    for required in [
        "printf '| Job | Result |\\n'",
        "printf \"| %s | \\`%s\\` |\\n\" \"${name}\" \"${result}\" >> \"${GITHUB_STEP_SUMMARY}\"",
        "::error title=Required PR job did not succeed::${name} concluded ${result}.",
        "Required PR job did not succeed: ${name} concluded ${result}.",
        "if (( failures != 0 )); then",
        "One or more required PR jobs did not succeed; see the result table and annotations above.",
    ] {
        if !workflow.contains(required) {
            return Err(format!("PR gate is missing actionable failure diagnostic `{required}`"));
        }
    }
    if workflow.contains("test \"${{ needs.") {
        return Err("PR gate must not use opaque success test predicates".to_owned());
    }
    if workflow.contains("windows-") {
        return Err("CI must not claim unsupported native Windows portability".to_owned());
    }

    Ok(())
}

#[test]
fn release_workflow_rechecks_the_msrv() -> Result<(), String> {
    let workflow = read_repository_file(".github/workflows/release.yml")?;
    for required in [
        "- name: Read the workspace MSRV",
        "rustup toolchain install \"${RUST_MSRV}\" --profile minimal",
        "cargo \"+${RUST_MSRV}\" ci-check",
        "cargo \"+${RUST_MSRV}\" ci-policy",
    ] {
        if !workflow.contains(required) {
            return Err(format!("release workflow is missing MSRV guard `{required}`"));
        }
    }
    Ok(())
}

#[test]
fn local_developer_workflow_covers_format_lint_test_and_release_checks() -> Result<(), String> {
    let script = read_repository_file("scripts/check-all.sh")?;

    for required in [
        "list_existing_files",
        "cargo fmt --all",
        "bash scripts/check-files.sh --fix",
        "git --no-pager diff --check",
        "actionlint",
        "zizmor .github/workflows",
        "cargo ci-check",
        "cargo ci-policy",
        "cargo ci-clippy",
        "cargo ci-test",
        "cargo ci-doctest",
        "cargo ci-doc",
        "cargo test --locked --test conformance",
        "cargo test --locked --test runtime_conformance",
        "cargo test --locked --test real_world",
        "cargo test --locked --test public_api",
        "cargo test --locked --test generated_rendering",
        "cargo package --locked --allow-dirty",
        "cargo llvm-cov clean --locked",
        "cargo llvm-cov --locked --no-clean --workspace --all-features",
        "cargo \"+${msrv}\" ci-check",
        "cargo \"+${msrv}\" ci-policy",
        "cargo deny --all-features check",
        "lychee --config lychee.toml --root-dir . --offline",
        "validation_storage_root",
        "coverage_target_dir",
        "semver_cargo_home",
        "semver_target_dir",
        "${CARGO_TARGET_DIR:-${repository_root}/target}/check-all/compose-lens",
        "${validation_storage_root}/coverage",
        "${validation_storage_root}/cargo-home",
        "${validation_storage_root}/cargo-semver-checks-target",
        "env CARGO_TARGET_DIR=\"${coverage_target_dir}\"",
        "env CARGO_HOME=\"${semver_cargo_home}\"",
        "CARGO_TARGET_DIR=\"${semver_target_dir}\"",
        "cargo semver-checks check-release",
        "--package compose-lens",
    ] {
        if !script.contains(required) {
            return Err(format!("local validation runner missing `{required}`"));
        }
    }

    if script.contains("semver_cargo_home=\"${CARGO_HOME:-}\"") {
        return Err("local SemVer checks must not reuse ambient CARGO_HOME".to_owned());
    }

    if script.contains("--release-type") {
        return Err("local SemVer checks must derive the release type from Cargo versions".to_owned());
    }

    for (path, required) in [
        (
            ".vscode/settings.json",
            &["rust-analyzer.check.command", "editor.formatOnSave"][..],
        ),
        (
            ".vscode/extensions.json",
            &[
                "DavidAnson.vscode-markdownlint",
                "esbenp.prettier-vscode",
                "mkhl.shfmt",
                "tombi-toml.tombi",
                "timonwong.shellcheck",
            ][..],
        ),
        (
            ".vscode/tasks.json",
            &[
                "ComposeLens: Format, lint, and test all",
                "scripts/check-all.sh",
                "ComposeLens: Required Rust checks",
                "ComposeLens: Package",
            ][..],
        ),
    ] {
        let contents = read_repository_file(path)?;
        for value in required {
            if !contents.contains(value) {
                return Err(format!("{path} is missing `{value}`"));
            }
        }
    }

    Ok(())
}

#[test]
fn non_rust_file_quality_is_locked_and_required() -> Result<(), String> {
    let script = read_repository_file("scripts/check-files.sh")?;
    for required in [
        "git ls-files --cached --others --exclude-standard",
        "list_existing_files",
        ":(exclude)schema/compose-spec.json",
        "markdownlint-cli2 --fix",
        "prettier --write",
        "prettier --check",
        "tombi format --check --offline",
        "tombi lint --error-on-warnings --offline",
        "shfmt -w",
        "shellcheck --",
        "hadolint",
    ] {
        if !script.contains(required) {
            return Err(format!("non-Rust file runner missing `{required}`"));
        }
    }

    let tombi = read_repository_file("tombi.toml")?;
    for required in [
        "dotted-keys-out-of-order = \"error\"",
        "key-empty = \"error\"",
        "tables-out-of-order = \"error\"",
        "docs/schemas/tombi-cargo-offline.schema.json",
        "include = [\"Cargo.toml\", \"**/Cargo.toml\"]",
        "enabled = false",
        "fixtures/**/*.toml",
        "conformance/records/**/*.toml",
    ] {
        if !tombi.contains(required) {
            return Err(format!("tombi.toml is missing `{required}`"));
        }
    }

    let cargo_schema = read_repository_file("docs/schemas/tombi-cargo-offline.schema.json")?;
    for required in [r#""type": "object""#, r#""additionalProperties": true"#] {
        if !cargo_schema.contains(required) {
            return Err(format!("offline Cargo schema must contain `{required}`"));
        }
    }

    let lock = read_repository_file("package-lock.json")?;
    for package in ["markdownlint-cli2", "prettier"] {
        if !lock.contains(&format!("\"{package}\"")) {
            return Err(format!("package-lock.json must lock `{package}`"));
        }
    }

    for workflow_name in ["ci.yml", "release.yml"] {
        let workflow = read_repository_file(&format!(".github/workflows/{workflow_name}"))?;
        for required in [
            "npm ci --ignore-scripts",
            "bash scripts/install-file-tools.sh /usr/local/bin",
            "bash scripts/check-files.sh --check",
        ] {
            if !workflow.contains(required) {
                return Err(format!("{workflow_name} is missing `{required}`"));
            }
        }
    }

    Ok(())
}

#[test]
fn routine_link_checks_are_offline_and_external_checks_are_scheduled() -> Result<(), String> {
    let ci = read_repository_file(".github/workflows/ci.yml")?;
    for required in ["--config lychee.toml", "--offline"] {
        if !ci.contains(required) {
            return Err(format!("CI local-link check is missing `{required}`"));
        }
    }

    let external = read_repository_file(".github/workflows/documentation-links.yml")?;
    for required in ["schedule:", "workflow_dispatch:", "path: .lycheecache", "--cache"] {
        if !external.contains(required) {
            return Err(format!("scheduled external-link workflow is missing `{required}`"));
        }
    }

    Ok(())
}

#[test]
fn specification_drift_check_is_scheduled_manual_and_read_only() -> Result<(), String> {
    let workflow = read_repository_file(".github/workflows/specification-drift.yml")?;
    for required in [
        "schedule:",
        "workflow_dispatch:",
        "contents: read",
        "bash scripts/check-specification-drift.sh",
    ] {
        if !workflow.contains(required) {
            return Err(format!("specification-drift workflow is missing `{required}`"));
        }
    }
    for forbidden in ["pull_request:", "push:", "issues: write", "pull-requests: write"] {
        if workflow.contains(forbidden) {
            return Err(format!("specification-drift workflow must not contain `{forbidden}`"));
        }
    }

    let script = read_repository_file("scripts/check-specification-drift.sh")?;
    for required in [
        "https://raw.githubusercontent.com/compose-spec/compose-spec/main/schema/compose-spec.json",
        "Committed snapshot SHA-256:",
        "added:",
        "removed:",
        "Inventory drift detected:",
        "Content-only drift detected:",
        "no inventory-key changes",
        "shasum -a 256",
    ] {
        if !script.contains(required) {
            return Err(format!("specification-drift script is missing `{required}`"));
        }
    }
    if script.contains("sha256sum") {
        return Err("specification-drift script must not require Linux-only `sha256sum`".to_owned());
    }
    Ok(())
}

#[test]
fn specification_drift_reports_content_only_changes_without_hiding_them() -> Result<(), String> {
    let root = repository_root();
    let upstream_path = std::env::temp_dir().join(format!(
        "compose-lens-specification-drift-{}-content-only.json",
        std::process::id()
    ));
    let mut content = fs::read(root.join("schema/compose-spec.json"))
        .map_err(|error| format!("failed to read committed schema: {error}"))?;
    content.push(b'\n');
    fs::write(&upstream_path, content)
        .map_err(|error| format!("failed to write temporary upstream schema: {error}"))?;

    let command_result = Command::new("bash")
        .arg(root.join("scripts/check-specification-drift.sh"))
        .env(
            "COMPOSE_SPECIFICATION_URL",
            format!("file://{}", upstream_path.display()),
        )
        .output();
    let cleanup_result = fs::remove_file(&upstream_path);
    let output = command_result.map_err(|error| format!("failed to run specification-drift script: {error}"))?;
    cleanup_result.map_err(|error| format!("failed to remove temporary upstream schema: {error}"))?;

    if output.status.success() {
        return Err("content-only schema drift must fail the check".to_owned());
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let transcript = format!("status: {}; stdout: {stdout:?}; stderr: {stderr:?}", output.status);
    if !stdout.contains("no inventory-key changes") {
        return Err(format!(
            "content-only drift must report unchanged inventory sets; {transcript}"
        ));
    }
    if !stderr.contains("Content-only drift detected")
        || !stderr.contains("nested, prose, or other non-inventory schema changes")
    {
        return Err(format!(
            "content-only drift must provide actionable review guidance; {transcript}"
        ));
    }
    Ok(())
}

#[test]
fn compose_schema_snapshot_and_inventory_are_complete() -> Result<(), String> {
    let root = repository_root();
    let schema_path = root.join("schema/compose-spec.json");
    let inventory_path = root.join("schema/compose-key-inventory.json");
    let schema_bytes =
        fs::read(&schema_path).map_err(|error| format!("failed to read {}: {error}", schema_path.display()))?;
    let schema: Value = serde_json::from_slice(&schema_bytes)
        .map_err(|error| format!("failed to parse {}: {error}", schema_path.display()))?;
    let inventory: Value = serde_json::from_str(
        &fs::read_to_string(&inventory_path)
            .map_err(|error| format!("failed to read {}: {error}", inventory_path.display()))?,
    )
    .map_err(|error| format!("failed to parse {}: {error}", inventory_path.display()))?;

    validate_inventory_header(&inventory)?;

    let expected_digest = inventory
        .pointer("/upstream/sha256")
        .and_then(Value::as_str)
        .ok_or("inventory must declare `upstream.sha256`")?;
    let digest = format!("{:x}", Sha256::digest(schema_bytes));
    if digest != expected_digest {
        return Err(format!(
            "schema digest mismatch: expected {expected_digest}, found {digest}"
        ));
    }

    for (field, expected) in [
        ("repository", "https://github.com/compose-spec/compose-spec"),
        ("commit", "11296e387ba76c77db1db768b9153a4304a3c9bd"),
        ("path", "schema/compose-spec.json"),
        ("blob", "fe0e45d68542fee8ba7b1e483760c2f8802f8a4c"),
    ] {
        let actual = inventory
            .pointer(&format!("/upstream/{field}"))
            .and_then(Value::as_str)
            .ok_or_else(|| format!("inventory must declare `upstream.{field}`"))?;
        if actual != expected {
            return Err(format!(
                "inventory `upstream.{field}` must be `{expected}`, found `{actual}`"
            ));
        }
    }

    validate_closed_object(&schema, "", "root")?;
    validate_closed_object(&schema, "/$defs/service", "service")?;
    validate_inventory_set(&schema, &inventory, "root", "/properties")?;
    validate_inventory_set(&schema, &inventory, "service", "/$defs/service/properties")
}

#[test]
fn inventory_accepts_each_allowed_classification_with_required_shape() -> Result<(), String> {
    for entry in [
        serde_json::json!({ "classification": "typed" }),
        serde_json::json!({
            "classification": "preserved-only",
            "rationale": "Nested semantics remain intentionally outside this bounded inventory."
        }),
        serde_json::json!({
            "classification": "intentionally-unsupported",
            "rationale": "The supported public contract deliberately excludes this key."
        }),
    ] {
        validate_classification("service", "example", &entry)?;
    }
    Ok(())
}

#[test]
fn inventory_rejects_missing_rationales_unknown_classes_and_key_drift() -> Result<(), String> {
    let missing_rationale = serde_json::json!({ "classification": "preserved-only" });
    let missing_rationale_error = validate_classification("root", "example", &missing_rationale)
        .err()
        .ok_or_else(|| "non-typed entries must require rationales".to_owned())?;
    assert!(missing_rationale_error.contains("rationale"));

    let unknown_classification = serde_json::json!({ "classification": "eventually" });
    let unknown_classification_error = validate_classification("root", "example", &unknown_classification)
        .err()
        .ok_or_else(|| "unknown classifications must fail".to_owned())?;
    assert!(unknown_classification_error.contains("unsupported classification"));

    let schema = serde_json::json!({ "properties": { "known": {} } });
    let inventory = serde_json::json!({
        "root": { "unexpected": { "classification": "typed" } }
    });
    let key_drift_error = validate_inventory_set(&schema, &inventory, "root", "/properties")
        .err()
        .ok_or_else(|| "schema and inventory keys must match exactly".to_owned())?;
    assert!(key_drift_error.contains("does not match schema"));
    Ok(())
}

fn validate_closed_object(schema: &Value, pointer: &str, name: &str) -> Result<(), String> {
    let object = schema
        .pointer(pointer)
        .and_then(Value::as_object)
        .ok_or_else(|| format!("schema {name} must be an object"))?;
    if object.get("additionalProperties") != Some(&Value::Bool(false)) {
        return Err(format!("schema {name} must set `additionalProperties` to false"));
    }
    let patterns = object
        .get("patternProperties")
        .and_then(Value::as_object)
        .ok_or_else(|| format!("schema {name} must declare `patternProperties`"))?;
    if patterns.len() != 1 || !patterns.contains_key("^x-") {
        return Err(format!("schema {name} must allow only the `^x-` extension namespace"));
    }
    Ok(())
}

fn validate_inventory_header(inventory: &Value) -> Result<(), String> {
    let root = inventory.as_object().ok_or("inventory must be an object")?;
    let expected_top_level = BTreeSet::from([
        "root".to_owned(),
        "schema".to_owned(),
        "service".to_owned(),
        "upstream".to_owned(),
    ]);
    let actual_top_level: BTreeSet<_> = root.keys().cloned().collect();
    if actual_top_level != expected_top_level {
        return Err(format!(
            "inventory must contain only schema, upstream, root, and service; found {actual_top_level:?}"
        ));
    }
    if root.get("schema") != Some(&Value::from(1)) {
        return Err("inventory `schema` must be integer 1".to_owned());
    }

    let upstream = root
        .get("upstream")
        .and_then(Value::as_object)
        .ok_or("inventory `upstream` must be an object")?;
    let expected_upstream = BTreeSet::from([
        "blob".to_owned(),
        "commit".to_owned(),
        "path".to_owned(),
        "repository".to_owned(),
        "sha256".to_owned(),
    ]);
    let actual_upstream: BTreeSet<_> = upstream.keys().cloned().collect();
    if actual_upstream != expected_upstream {
        return Err(format!(
            "inventory upstream metadata must have the pinned source fields only; found {actual_upstream:?}"
        ));
    }
    Ok(())
}

fn validate_inventory_set(schema: &Value, inventory: &Value, scope: &str, schema_pointer: &str) -> Result<(), String> {
    let schema_keys = object_keys(
        schema
            .pointer(schema_pointer)
            .ok_or_else(|| format!("schema missing `{schema_pointer}`"))?,
        &format!("schema {scope} properties"),
    )?;
    let inventory_properties = inventory
        .get(scope)
        .ok_or_else(|| format!("inventory missing `{scope}`"))?;
    let inventory_keys = object_keys(inventory_properties, &format!("inventory {scope}"))?;

    if schema_keys != inventory_keys {
        let missing = schema_keys.difference(&inventory_keys).cloned().collect::<Vec<_>>();
        let unexpected = inventory_keys.difference(&schema_keys).cloned().collect::<Vec<_>>();
        return Err(format!(
            "{scope} inventory does not match schema; missing {missing:?}, unexpected {unexpected:?}"
        ));
    }

    for (key, entry) in inventory_properties
        .as_object()
        .ok_or_else(|| format!("inventory {scope} must be an object"))?
    {
        validate_classification(scope, key, entry)?;
    }
    Ok(())
}

fn object_keys(value: &Value, name: &str) -> Result<BTreeSet<String>, String> {
    value
        .as_object()
        .ok_or_else(|| format!("{name} must be an object"))
        .map(|properties| properties.keys().cloned().collect())
}

fn validate_classification(scope: &str, key: &str, entry: &Value) -> Result<(), String> {
    let entry = entry
        .as_object()
        .ok_or_else(|| format!("inventory {scope}.{key} must be an object"))?;
    let classification = entry
        .get("classification")
        .and_then(Value::as_str)
        .ok_or_else(|| format!("inventory {scope}.{key} must declare `classification`"))?;
    match classification {
        "typed" => {
            if entry.len() != 1 {
                return Err(format!(
                    "typed inventory entry {scope}.{key} may contain only `classification`"
                ));
            }
        }
        "preserved-only" | "intentionally-unsupported" => {
            let rationale = entry
                .get("rationale")
                .and_then(Value::as_str)
                .filter(|rationale| !rationale.trim().is_empty())
                .ok_or_else(|| format!("non-typed inventory entry {scope}.{key} needs a non-empty `rationale`"))?;
            if entry.len() != 2 || rationale.is_empty() {
                return Err(format!(
                    "non-typed inventory entry {scope}.{key} may contain only `classification` and `rationale`"
                ));
            }
        }
        other => {
            return Err(format!(
                "inventory {scope}.{key} has unsupported classification `{other}`"
            ));
        }
    }
    Ok(())
}

#[test]
fn release_workflow_uses_the_create_response_as_its_draft_identity() -> Result<(), String> {
    let workflow_path = repository_root().join(".github/workflows/release.yml");
    let workflow = fs::read_to_string(&workflow_path)
        .map_err(|error| format!("failed to read {}: {error}", workflow_path.display()))?;

    if workflow.contains("/releases/tags/") {
        return Err("release workflow must not use the published-release-by-tag endpoint for drafts".to_owned());
    }
    if workflow.contains("databaseId") {
        return Err("release workflow must use stable REST release fields instead of CLI JSON fields".to_owned());
    }
    if workflow.contains("gh release create") || workflow.contains("gh release list") {
        return Err(
            "release workflow must not rediscover a newly created draft through high-level CLI commands".to_owned(),
        );
    }

    for required in [
        "RELEASE_GITHUB_API_VERSION: \"2026-03-10\"",
        "repos/${GITHUB_REPOSITORY}/releases?per_page=100",
        "gh api --method POST",
        "target_commitish: $target",
        "'.upload_url | sub(",
        "steps.release.outputs.upload_url",
        "--data-binary \"@${asset_path}\"",
        "steps.release.outputs.release_id",
    ] {
        if !workflow.contains(required) {
            return Err(format!(
                "release workflow is missing the draft release-ID guard `{required}`"
            ));
        }
    }

    let release_list_endpoint = "repos/${GITHUB_REPOSITORY}/releases?per_page=100";
    if workflow.matches(release_list_endpoint).count() != 1 {
        return Err(
            "release workflow must list releases only before creation and never rediscover the new draft".to_owned(),
        );
    }

    Ok(())
}

#[test]
fn release_workflow_uses_only_trusted_publishing() -> Result<(), String> {
    let workflow_path = repository_root().join(".github/workflows/release.yml");
    let workflow = fs::read_to_string(&workflow_path)
        .map_err(|error| format!("failed to read {}: {error}", workflow_path.display()))?;

    for forbidden in [
        "CRATES_IO_API_TOKEN",
        "CRATES_IO_BOOTSTRAP_TOKEN",
        "cargo login",
        "--token",
        "secrets.",
    ] {
        if workflow.contains(forbidden) {
            return Err(format!(
                "release workflow contains the forbidden long-lived credential path `{forbidden}`"
            ));
        }
    }

    for required in [
        "id-token: write",
        "rust-lang/crates-io-auth-action@",
        "CARGO_REGISTRY_TOKEN: ${{ steps.crates-auth.outputs.token }}",
        "cargo publish --locked",
    ] {
        if !workflow.contains(required) {
            return Err(format!(
                "release workflow is missing the trusted-publishing guard `{required}`"
            ));
        }
    }

    if workflow.matches("cargo publish --locked").count() != 1 {
        return Err("release workflow must contain exactly one publication command".to_owned());
    }

    Ok(())
}

#[test]
fn release_plz_prepares_only_guarded_releases() -> Result<(), String> {
    validate_release_plz_contract("Strukturpiloten/compose-lens")
}

#[test]
fn release_note_extraction_is_strict_and_bounded() -> Result<(), String> {
    validate_release_note_extraction("compose-lens")
}

fn validate_release_plz_contract(repository: &str) -> Result<(), String> {
    if repository_root().join("docs/releases").exists() {
        return Err("CHANGELOG.md must remain the only release-history source".to_owned());
    }
    let config_text = read_repository_file("release-plz.toml")?;
    let config = toml::from_str::<toml::Value>(&config_text)
        .map_err(|error| format!("failed to parse release-plz.toml: {error}"))?;
    let workspace = config["workspace"]
        .as_table()
        .ok_or_else(|| "release-plz.toml must contain [workspace]".to_owned())?;
    for (name, expected) in [
        ("allow_dirty", false),
        ("changelog_update", true),
        ("dependencies_update", false),
        ("git_release_enable", false),
        ("git_tag_enable", false),
        ("publish", false),
        ("release_always", false),
        ("semver_check", true),
    ] {
        if workspace.get(name).and_then(toml::Value::as_bool) != Some(expected) {
            return Err(format!("release-plz workspace setting {name} must be {expected}"));
        }
    }
    if workspace.get("changelog_path").and_then(toml::Value::as_str) != Some("CHANGELOG.md")
        || workspace.get("pr_branch_prefix").and_then(toml::Value::as_str) != Some("release-plz-")
    {
        return Err("release-plz must use the root changelog and guarded branch prefix".to_owned());
    }

    let workflow = read_repository_file(".github/workflows/release-plz.yml")?;
    for required in [
        repository,
        "secrets.RELEASE_PLZ_APP_ID",
        "secrets.RELEASE_PLZ_APP_PRIVATE_KEY",
        "permission-contents: write",
        "permission-pull-requests: write",
        "command: release-pr",
        "version: \"0.3.160\"",
        "release-plz/action@2eb1d8bcb770b4c48ccfaad919734b38b51958c9 # v0.5.131",
        "actions/create-github-app-token@bcd2ba49218906704ab6c1aa796996da409d3eb1 # v3.2.0",
        "(.head.ref | startswith(\"release-plz-\"))",
        "actions/workflows/release.yml/dispatches",
        "actions: write",
        "No release was dispatched.",
    ] {
        if !workflow.contains(required) {
            return Err(format!("release-plz workflow is missing `{required}`"));
        }
    }
    for forbidden in ["command: release\n", "cargo publish", "git tag", "gh release create"] {
        if workflow.contains(forbidden) {
            return Err(format!("release-plz workflow must not contain `{forbidden}`"));
        }
    }

    let release = read_repository_file(".github/workflows/release.yml")?;
    if release.contains("docs/releases/${version}.md") || !release.contains("bash scripts/extract-release-notes.sh") {
        return Err("protected publication must derive release notes from CHANGELOG.md".to_owned());
    }
    Ok(())
}

fn validate_release_note_extraction(repository: &str) -> Result<(), String> {
    let root = repository_root();
    let directory = std::env::temp_dir().join(format!("{repository}-release-notes-{}", std::process::id()));
    let changelog = directory.join("CHANGELOG.md");
    fs::create_dir_all(&directory).map_err(|error| format!("failed to create {}: {error}", directory.display()))?;
    fs::write(
        &changelog,
        "# Changelog\n\n## [Unreleased]\n\n## [1.2.3](https://example.invalid/v1.2.3) - 2026-08-17\n\n### Added\n\n- Useful change.\n\n## [1.2.2] - 2026-08-16\n\n- Older change.\n",
    )
    .map_err(|error| format!("failed to write {}: {error}", changelog.display()))?;
    let valid = run_release_notes_script(&root, "1.2.3", &changelog)?;
    let valid_stdout = String::from_utf8(valid.stdout).map_err(|error| error.to_string())?;
    if !valid.status.success() || !valid_stdout.contains("Useful change") || valid_stdout.contains("Older change") {
        return Err("valid release notes were not extracted as one bounded section".to_owned());
    }

    let missing = run_release_notes_script(&root, "9.9.9", &changelog)?;
    if missing.status.success() || !String::from_utf8_lossy(&missing.stderr).contains("no release section") {
        return Err("a missing release section must fail with an actionable diagnostic".to_owned());
    }
    let malformed_version = run_release_notes_script(&root, "v1.2.3", &changelog)?;
    if malformed_version.status.success()
        || !String::from_utf8_lossy(&malformed_version.stderr).contains("major.minor.patch")
    {
        return Err("a malformed release version must fail before extraction".to_owned());
    }

    fs::write(
        &changelog,
        "# Changelog\n\n## [1.2.3] - 2026-08-17\n\n## [1.2.2] - 2026-08-16\n",
    )
    .map_err(|error| format!("failed to write {}: {error}", changelog.display()))?;
    let empty = run_release_notes_script(&root, "1.2.3", &changelog)?;
    if empty.status.success() || !String::from_utf8_lossy(&empty.stderr).contains("is empty") {
        return Err("an empty release section must fail".to_owned());
    }

    fs::write(&changelog, "# Changelog\n\n## [1.2.3] - not-a-date\n\n- Change.\n")
        .map_err(|error| format!("failed to write {}: {error}", changelog.display()))?;
    let malformed_heading = run_release_notes_script(&root, "1.2.3", &changelog)?;
    if malformed_heading.status.success() || !String::from_utf8_lossy(&malformed_heading.stderr).contains("YYYY-MM-DD")
    {
        return Err("a malformed release heading must fail".to_owned());
    }

    fs::remove_dir_all(&directory).map_err(|error| format!("failed to remove {}: {error}", directory.display()))?;
    Ok(())
}

fn run_release_notes_script(root: &Path, version: &str, changelog: &Path) -> Result<Output, String> {
    Command::new("bash")
        .arg(root.join("scripts/extract-release-notes.sh"))
        .arg(version)
        .arg(changelog)
        .current_dir(root)
        .output()
        .map_err(|error| format!("failed to run release-note extractor: {error}"))
}

#[test]
fn fixture_manifests_follow_the_common_contract() -> Result<(), String> {
    support::validate_fixture_tree(&repository_root(), FIXTURE_SUITES)
}

#[test]
fn fixture_contract_accepts_authored_metadata() {
    let errors = support::validate_fixture_manifest_text(
        "valid fixture",
        r#"
schema = 1
id = "minimal-service"
suite = "syntax"
description = "Protects a minimal service."
secrets_reviewed = true
files = ["compose.yaml"]

[provenance]
source = "authored"
license = "MPL-2.0"
redistribution = "allowed"
modifications = "none"

[environment]
description = "No environment is provided."

[expectations]
summary = "The service remains present."
"#,
        FIXTURE_SUITES,
    );

    assert!(errors.is_empty(), "{errors:#?}");
}

#[test]
fn fixture_contract_rejects_unsafe_external_metadata() {
    let errors = support::validate_fixture_manifest_text(
        "invalid fixture",
        r#"
schema = 1
id = "external-project"
suite = "real-world"
description = "An incomplete external fixture."
secrets_reviewed = false
files = ["../secret.env"]

[provenance]
source = "external"
license = "unknown"
redistribution = "allowed"
modifications = "none"

[environment]
description = "Unknown."

[expectations]
summary = "Must not be accepted."
"#,
        FIXTURE_SUITES,
    );

    assert!(
        errors.iter().any(|error| error.contains("secrets_reviewed")),
        "{errors:#?}"
    );
    assert!(
        errors.iter().any(|error| error.contains("unsafe fixture path")),
        "{errors:#?}"
    );
    assert!(errors.iter().any(|error| error.contains("`url`")), "{errors:#?}");
    assert!(errors.iter().any(|error| error.contains("`revision`")), "{errors:#?}");
}

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read_repository_file(path: &str) -> Result<String, String> {
    let path = repository_root().join(path);
    fs::read_to_string(&path).map_err(|error| format!("failed to read {}: {error}", path.display()))
}
