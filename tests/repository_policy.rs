//! Executable repository and fixture-contract checks.

mod support;

#[path = "support/application_policy.rs"]
mod application_policy;

#[test]
fn imported_application_fixtures_are_exact_and_independent() -> Result<(), String> {
    application_policy::validate_application_fixtures()
}

#[test]
fn application_evidence_is_reviewed_bounded_and_linked() -> Result<(), String> {
    application_policy::validate_application_evidence()
}

#[test]
fn application_contract_is_independent_and_runs_in_every_complete_gate() -> Result<(), String> {
    application_policy::validate_application_independence_and_gates()
}

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
fn ci_runs_once_per_pull_request_update_and_on_main_pushes() -> Result<(), String> {
    let workflow_path = repository_root().join(".github/workflows/ci.yml");
    let workflow = fs::read_to_string(&workflow_path)
        .map_err(|error| format!("failed to read {}: {error}", workflow_path.display()))?;
    let expected =
        "on:\n  push:\n    branches:\n      - main\n  pull_request:\n  workflow_dispatch:\n  workflow_call:\n";
    if !workflow.contains(expected) {
        return Err(
            "CI must run for main pushes, pull requests, and manual dispatch without duplicate feature-branch push runs"
                .to_owned(),
        );
    }

    Ok(())
}

#[test]
fn release_requires_reusable_complete_and_bounded_native_validation() -> Result<(), String> {
    let release = read_repository_file(".github/workflows/release.yml")?;

    for required in [
        "validation_only:",
        "default: false",
        "uses: ./.github/workflows/ci.yml",
        "uses: ./.github/workflows/provider-conformance.yml",
        "provider-config-conformance:\n    name: Validate observed provider configuration evidence\n    if: github.repository == 'Strukturpiloten/compose-lens'\n    uses: ./.github/workflows/provider-conformance.yml\n    permissions:\n      contents: read",
        "needs: [provider-config-conformance, deterministic-validation, release-metadata-validation]",
        "needs: release-gate",
        "inputs.validation_only != true",
        "needs.release-gate.result == 'success'",
        "artifact-metadata: write",
        "attestations: write",
        "contents: write",
        "id-token: write",
    ] {
        if !release.contains(required) {
            return Err(format!("release workflow is missing fail-closed contract `{required}`"));
        }
    }
    for required in [
        "  release-gate:\n    name: Release validation gate\n    if: always()",
        "PROVIDER_RESULT: ${{ needs.provider-config-conformance.result }}",
        "DETERMINISTIC_RESULT: ${{ needs.deterministic-validation.result }}",
        "METADATA_RESULT: ${{ needs.release-metadata-validation.result }}",
        "if [[ \"${status}\" != success ]]; then",
        "release-metadata-validation:\n    name: Validate release metadata",
        "bash scripts/check-release-metadata.sh",
    ] {
        if !release.contains(required) {
            return Err(format!("release gate is missing fail-closed contract `{required}`"));
        }
    }
    for forbidden in [
        "- name: Install pinned Node.js toolchain",
        "- name: Install locked Node file-quality tools",
        "- name: Install checksum-pinned native file-quality tools",
        "- name: Check non-Rust file formatting and lint",
        "- name: Run Clippy",
        "- name: Run tests",
    ] {
        let publication = release
            .split_once("  release:\n")
            .map_or(release.as_str(), |(_, publication)| publication);
        if publication.contains(forbidden) {
            return Err(format!("credentialed publication job must not repeat `{forbidden}`"));
        }
    }

    validate_provider_conformance_contract()
}

fn validate_provider_conformance_contract() -> Result<(), String> {
    let ci = read_repository_file(".github/workflows/ci.yml")?;
    let native = read_repository_file(".github/workflows/provider-conformance.yml")?;
    let runner = read_repository_file("scripts/run-observed-provider-config.sh")?;
    let bootstrap = read_repository_file("scripts/provider-python-bootstrap.sh")?;
    let complete_gate = read_repository_file("scripts/check-all.sh")?;
    let provider_matrix = read_repository_file("conformance/provider-config-matrix.toml")?;

    for required in [
        "workflow_call:",
        "workflow_dispatch:",
        "cron: \"23 4 * * 1\"",
        "ref: ${{ inputs.candidate_sha || github.sha }}",
        "test \"${actual}\" = \"${CANDIDATE_SHA}\"",
        "name: provider-config-${{ github.run_id }}-${{ matrix.target }}",
        "overwrite: true",
        "if: always()",
        "docker-compose-2-24-3",
        "docker-compose-2-24-4",
        "docker-compose-2-40-3",
        "docker-compose-5-3-1",
        "podman-compose-1-3-0",
        "podman-compose-1-5-0",
        "actions/setup-python@a26af69be951a213d495a4c3e4e4022e16d87065 # v5.6.0",
        "actions/upload-artifact@043fb46d1a93c77aae656e7c1c64a875d1fc6a0a # v7.0.1",
        "python-version: 3.13.14",
        "runs-on: ${{ matrix.runner }}",
        "COMPOSE_LENS_CONFORMANCE_RUNNER_LABEL: ${{ matrix.runner }}",
        "# renovate: datasource=github-runners\n        runner:\n          - ubuntu-24.04",
    ] {
        if !native.contains(required) {
            return Err(format!("native conformance workflow is missing `{required}`"));
        }
    }
    for required in [
        "run[\"status\"] == \"observed\"",
        "len(probes) != 8",
        "and run[\"status\"] == \"planned\"",
        "COMPOSE_LENS_CONFORMANCE_RESULT_DIRECTORY",
        "actual_python_runtime=",
        "does not match matrix",
        "bootstrap requirements must carry one lowercase SHA-256 hash",
        "--require-hashes",
        "requirements.txt",
        "provider_artifact_filename \"${artifact_url}\"",
        "hash_locked_local_wheel_requirement",
        "COMPOSE_LENS_CONFORMANCE_RUNNER_LABEL:?workflow must provide the provider conformance runner label",
        "github-actions-${conformance_runner_label/./-}_provider-config-only_runtime-not-invoked",
    ] {
        if !runner.contains(required) {
            return Err(format!("provider runner is missing bounded-evidence rule `{required}`"));
        }
    }
    for required in [
        "url.scheme != \"https\"",
        "pathlib.Path(sys.argv[1]).resolve(strict=True).as_uri()",
        "*.whl",
        "--hash=sha256:%s",
    ] {
        if !bootstrap.contains(required) {
            return Err(format!("provider Python bootstrap is missing `{required}`"));
        }
    }
    if !complete_gate.contains("bash scripts/test-provider-python-bootstrap.sh") {
        return Err("complete gate must run the provider Python bootstrap regression".to_owned());
    }
    if !ci.contains("run: bash scripts/test-provider-python-bootstrap.sh") {
        return Err("pull-request CI must run the provider Python bootstrap regression".to_owned());
    }
    if native.contains("runtime-effect-matrix") || runner.contains("runtime-effect-matrix") {
        return Err("release native validation must not execute runtime-effect rows".to_owned());
    }
    if provider_matrix.matches("\"--dry-run\"").count() != 4 {
        return Err("both podman-compose targets must keep dry-run version and config boundaries".to_owned());
    }
    Ok(())
}

#[test]
fn repository_supply_chain_has_single_sources_and_immutable_pins() -> Result<(), String> {
    support::validate_repository_supply_chain(&repository_root())
}

#[test]
fn public_api_compatibility_runs_in_ci_and_release() -> Result<(), String> {
    const ACTION: &str = "obi1kenobi/cargo-semver-checks-action@6b69fcf40e9b5fb17adeb57e4b6ecd020649a239 # v2.9";
    const CONFIGURATION: &str = "package: compose-lens";

    let workflow_name = "ci.yml";
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

    Ok(())
}

#[test]
fn coverage_ratchet_runs_in_ci_and_release() -> Result<(), String> {
    const CLEAN: &str = "cargo llvm-cov clean --locked";
    const COMMAND: &str = "cargo llvm-cov --locked --no-clean --workspace --all-features --all-targets --summary-only\n          --fail-under-regions 88 --fail-under-functions 87 --fail-under-lines 89";

    let dockerfile = read_repository_file(".devcontainer/Dockerfile")?;
    let expected_version = pinned_cargo_llvm_cov_version(&dockerfile, ".devcontainer/Dockerfile")?;

    for workflow_name in ["ci.yml"] {
        let workflow_path = repository_root().join(".github/workflows").join(workflow_name);
        let workflow = fs::read_to_string(&workflow_path)
            .map_err(|error| format!("failed to read {}: {error}", workflow_path.display()))?;

        let workflow_version = pinned_cargo_llvm_cov_version(&workflow, workflow_name)?;
        if workflow_version != expected_version {
            return Err(format!(
                "{workflow_name} pins cargo-llvm-cov {workflow_version}, but the Dev Container pins {expected_version}"
            ));
        }

        for required in ["rustup component add llvm-tools-preview", CLEAN, COMMAND] {
            if workflow.matches(required).count() != 1 {
                return Err(format!(
                    "{workflow_name} must contain one pinned ComposeLens coverage guard `{required}`"
                ));
            }
        }
    }

    Ok(())
}

fn pinned_cargo_llvm_cov_version(document: &str, source: &str) -> Result<String, String> {
    const WORKFLOW_PREFIX: &str = "run: cargo install --locked --version ";
    const WORKFLOW_SUFFIX: &str = " cargo-llvm-cov";
    const DEVCONTAINER_PREFIX: &str = "ARG CARGO_LLVM_COV_VERSION=";

    let versions = document
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            line.strip_prefix(WORKFLOW_PREFIX)
                .and_then(|value| value.strip_suffix(WORKFLOW_SUFFIX))
                .or_else(|| line.strip_prefix(DEVCONTAINER_PREFIX))
        })
        .collect::<Vec<_>>();

    if versions.len() != 1 {
        return Err(format!(
            "{source} must contain exactly one cargo-llvm-cov version pin, found {}",
            versions.len()
        ));
    }

    let version = versions[0];
    let components = version.split('.').collect::<Vec<_>>();
    if components.len() != 3
        || components
            .iter()
            .any(|component| component.is_empty() || !component.bytes().all(|byte| byte.is_ascii_digit()))
    {
        return Err(format!(
            "{source} must pin cargo-llvm-cov to an exact major.minor.patch version, found `{version}`"
        ));
    }

    Ok(version.to_owned())
}

#[test]
fn ci_uses_a_trusted_change_plan_and_fail_closed_aggregate_gate() -> Result<(), String> {
    let workflow = read_repository_file(".github/workflows/ci.yml")?;
    let policy = read_repository_file("scripts/validation-policy.json")?;
    let planner = read_repository_file("scripts/validation-plan.py")?;
    for required in [
        "  validation-plan:\n    name: Validation plan",
        "ref: ${{ github.event.pull_request.base.sha }}",
        "python3 .validation-base/scripts/validation-plan.py plan",
        "jobs = \"rust msrv dependencies api documentation coverage lockfile-release-age\".split()",
        "  pr-gate:\n    name: PR gate\n    if: always()",
        "NEEDS_JSON: ${{ toJSON(needs) }}",
        "python3 \"${verifier}\" gate",
        "run: PYTHONDONTWRITEBYTECODE=1 python3 scripts/test-validation-plan.py",
        "  lockfile-release-age:\n    name: Lockfile release age",
        "repository: Strukturpiloten/.github",
        "ref: ${{ github.event.pull_request.head.sha }}",
        "--repository-root \"${GITHUB_WORKSPACE}\"",
        "--base \"${BASE_SHA}\"",
        "--head \"${HEAD_SHA}\"",
        "--minimum-age-hours 72",
    ] {
        if !workflow.contains(required) {
            return Err(format!("CI workflow is missing contract `{required}`"));
        }
    }
    for job in [
        "rust",
        "msrv",
        "dependencies",
        "api",
        "documentation",
        "coverage",
        "lockfile-release-age",
    ] {
        let selection = format!(
            "if: needs.validation-plan.outputs.select_{} == 'true'",
            job.replace('-', "_")
        );
        if !workflow.contains(&selection) {
            return Err(format!("CI job {job} is not controlled by the trusted plan"));
        }
    }
    if workflow.contains("  portability:") || workflow.contains("runs-on: macos-") {
        return Err("Linux-only CI must not claim macOS runner evidence".to_owned());
    }
    let policy_value: Value =
        serde_json::from_str(&policy).map_err(|error| format!("validation policy is invalid JSON: {error}"))?;
    for (pointer, expected) in [
        ("/schema", serde_json::json!(2)),
        ("/repository", serde_json::json!("Strukturpiloten/compose-lens")),
        ("/documentation_examples_manifest", Value::Null),
        (
            "/profile_jobs/prose",
            serde_json::json!(["documentation", "lockfile-release-age"]),
        ),
    ] {
        if policy_value.pointer(pointer) != Some(&expected) {
            return Err(format!("validation policy has unexpected {pointer}"));
        }
    }
    let executable_jobs = policy_value
        .pointer("/profile_jobs/executable-docs")
        .and_then(Value::as_array)
        .ok_or("validation policy is missing executable-docs jobs")?;
    if !executable_jobs.starts_with(&[serde_json::json!("rust"), serde_json::json!("msrv")]) {
        return Err("executable-docs must run Rust and MSRV checks".to_owned());
    }
    if !policy.contains("scripts/check-all.sh") {
        return Err("validation policy is missing the complete local gate".to_owned());
    }
    for required in [
        "def verify_gate(",
        "def require_worktree_local_target(",
        "profile_jobs",
        "local_fingerprint",
    ] {
        if !planner.contains(required) {
            return Err(format!("validation planner is missing `{required}`"));
        }
    }
    Ok(())
}

#[test]
fn release_workflow_reuses_ci_for_the_msrv() -> Result<(), String> {
    let workflow = read_repository_file(".github/workflows/release.yml")?;
    for required in [
        "needs: [provider-config-conformance, deterministic-validation, release-metadata-validation]",
        "uses: ./.github/workflows/ci.yml",
    ] {
        if !workflow.contains(required) {
            return Err(format!("release workflow is missing reusable CI guard `{required}`"));
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
fn issue_to_pr_workflow_requires_the_complete_local_gate() -> Result<(), String> {
    for (path, required) in [
        (
            "AGENTS.md",
            &[
                "## GitHub issue-to-PR workflow",
                "Run `./scripts/check-all.sh`",
                "failed or incomplete full gate blocks commits, pushes, and pull-request",
                "ready pull request",
                "primary agent owns Git and GitHub writes",
                "Subagents never commit, push, publish, tag, release, or create pull requests",
            ][..],
        ),
        (
            "docs/development-environment.md",
            &[
                "## Complete validation",
                "./scripts/check-all.sh",
                "invalidates the result",
                "issue-to-PR sequence and ownership rules are canonical",
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
        "check_yaml_document_markers",
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

    let prettier_ignore = read_repository_file(".prettierignore")?;
    if prettier_ignore
        .lines()
        .filter(|line| !line.starts_with('#') && !line.is_empty())
        .collect::<Vec<_>>()
        != ["/CHANGELOG.md"]
    {
        return Err("only the release-plz-owned CHANGELOG.md may be excluded from Prettier".to_owned());
    }
    for required in [
        r#"prettier --write --ignore-path .prettierignore --ignore-unknown "${markdown_files[@]}""#,
        r#"markdownlint-cli2 --fix "${markdown_literals[@]}""#,
        r#"markdownlint-cli2 "${markdown_literals[@]}""#,
        r#"prettier --check --ignore-path .prettierignore --ignore-unknown "${markdown_files[@]}""#,
    ] {
        if !script.contains(required) {
            return Err(format!(
                "non-Rust file runner must preserve generated-changelog boundary `{required}`"
            ));
        }
    }

    let lock = read_repository_file("package-lock.json")?;
    for package in ["markdownlint-cli2", "prettier"] {
        if !lock.contains(&format!("\"{package}\"")) {
            return Err(format!("package-lock.json must lock `{package}`"));
        }
    }

    for workflow_name in ["ci.yml"] {
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
fn complete_yaml_documents_use_explicit_start_markers() -> Result<(), String> {
    let root = repository_root();
    let output = Command::new("git")
        .args(["ls-files", "-z", "--", "*.yaml", "*.yml"])
        .current_dir(&root)
        .output()
        .map_err(|error| format!("failed to list YAML documents: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "git ls-files failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    for path in output.stdout.split(|byte| *byte == 0).filter(|path| !path.is_empty()) {
        let path = Path::new(std::str::from_utf8(path).map_err(|error| error.to_string())?);
        let contents = fs::read_to_string(root.join(path))
            .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
        if contents.lines().next() != Some("---") {
            return Err(format!("{} must start with `---`", path.display()));
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
    if workspace.get("release_commits").and_then(toml::Value::as_str)
        != Some(r"^(feat|fix|perf|refactor|revert)(\([^)]+\))?!?:")
    {
        return Err("release-plz must prepare releases only for release-worthy code commits".to_owned());
    }

    validate_release_plz_changelog(&config)?;

    let workflow = read_repository_file(".github/workflows/release-plz.yml")?;
    for required in [
        repository,
        "vars.RELEASE_PLZ_APP_CLIENT_ID",
        "client-id:",
        "secrets.RELEASE_PLZ_APP_PRIVATE_KEY",
        "permission-contents: write",
        "permission-pull-requests: write",
        "continue-on-error: true",
        "steps.app-token.outcome == 'failure'",
        "approve the updated permissions for the App installation",
        "command: release-pr",
        "renovate: datasource=crate depName=release-plz",
        "version: \"0.3.160\"",
        "(.head.ref | startswith(\"release-plz-\"))",
        "actions/workflows/release.yml/dispatches",
        "actions: write",
        "No release was dispatched.",
    ] {
        if !workflow.contains(required) {
            return Err(format!("release-plz workflow is missing `{required}`"));
        }
    }
    for forbidden in [
        "secrets.RELEASE_PLZ_APP_ID",
        "app-id:",
        "command: release\n",
        "cargo publish",
        "git tag",
        "gh release create",
    ] {
        if workflow.contains(forbidden) {
            return Err(format!("release-plz workflow must not contain `{forbidden}`"));
        }
    }

    for action in ["release-plz/action", "actions/create-github-app-token"] {
        if !support::has_exactly_one_immutable_versioned_action(&workflow, action) {
            return Err(format!(
                "release-plz workflow must contain exactly one immutable versioned `{action}` action"
            ));
        }
    }

    if workflow
        .matches("renovate: datasource=crate depName=release-plz")
        .count()
        != 1
    {
        return Err("release-plz action must have one canonical Renovate extraction marker".to_owned());
    }

    let release = read_repository_file(".github/workflows/release.yml")?;
    if release.contains("docs/releases/${version}.md") || !release.contains("bash scripts/extract-release-notes.sh") {
        return Err("protected publication must derive release notes from CHANGELOG.md".to_owned());
    }
    Ok(())
}

fn validate_release_plz_changelog(config: &toml::Value) -> Result<(), String> {
    let changelog = config["changelog"]
        .as_table()
        .ok_or_else(|| "release-plz.toml must contain [changelog]".to_owned())?;
    if changelog.get("protect_breaking_commits").and_then(toml::Value::as_bool) != Some(true) {
        return Err("release-plz must preserve breaking commits in generated changelogs".to_owned());
    }

    let parsers = changelog
        .get("commit_parsers")
        .and_then(toml::Value::as_array)
        .ok_or_else(|| "release-plz must configure changelog commit parsers".to_owned())?;
    let expected = [
        ("^feat", Some("Added"), false),
        ("^fix", Some("Fixed"), false),
        ("^perf", Some("Performance"), false),
        ("^refactor", Some("Changed"), false),
        ("^revert", Some("Reverted"), false),
        ("^.*", None, true),
    ];
    if parsers.len() != expected.len() {
        return Err("release-plz must configure the exact code-only changelog parser set".to_owned());
    }
    for (parser, (message, group, skip)) in parsers.iter().zip(expected) {
        if parser["message"].as_str() != Some(message)
            || parser.get("group").and_then(toml::Value::as_str) != group
            || parser.get("skip").and_then(toml::Value::as_bool).unwrap_or(false) != skip
        {
            return Err(format!("release-plz changelog parser for {message} is invalid"));
        }
    }

    let releasing = read_repository_file("docs/releasing.md")?;
    for required in [
        "## Release classification",
        "`feat`, `fix`, `perf`, `refactor`, or `revert`",
        "`docs`, `test`, `ci`, `build`, `style`, or `chore`",
    ] {
        if !releasing.contains(required) {
            return Err(format!("release documentation is missing `{required}`"));
        }
    }

    Ok(())
}

#[test]
fn tracked_files_are_not_ignored() -> Result<(), String> {
    let output = Command::new("git")
        .args(["ls-files", "-ci", "--exclude-standard"])
        .current_dir(repository_root())
        .output()
        .map_err(|error| format!("failed to inspect tracked ignored files: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "git ls-files failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let tracked_ignored = String::from_utf8_lossy(&output.stdout);
    if !tracked_ignored.trim().is_empty() {
        return Err(format!(
            "tracked files must not also be ignored:\n{}",
            tracked_ignored.trim()
        ));
    }

    Ok(())
}

#[test]
fn renovate_tracks_every_directly_pinned_development_tool() -> Result<(), String> {
    let renovate = read_repository_file(".github/renovate.json")?;
    let renovate_value: Value =
        serde_json::from_str(&renovate).map_err(|error| format!("failed to parse Renovate configuration: {error}"))?;
    validate_renovate_lockfile_policy(&renovate_value)?;
    for required in [
        "Update versioned Dev Container tools",
        "Signal updates for checksum-pinned file-quality tools",
        "Update directly pinned workflow tool versions",
        "Update the documented Dev Container CLI",
        "Update the GitHub CLI installed in the Dev Container",
        "Signal manual review for provider-conformance Python bootstrap pins",
        "sha256=(?<currentDigest>[a-f0-9]{64})",
        "Track the reviewed Python bootstrap runtime across the provider matrix and runner",
        "Track fixed GitHub-hosted runner environments",
        "Require provenance and checksum review for provider-conformance bootstrap pins",
        "Review GitHub-hosted runner environment upgrades manually",
        r#""matchManagers": ["cargo"]"#,
        r#""matchManagers": ["npm"]"#,
        r#""matchManagers": ["github-actions"]"#,
        r#""matchManagers": ["devcontainer"]"#,
        r#""matchManagers": ["rust-toolchain"]"#,
        "Automerge tested non-major dependency updates",
        "Do not delay BoxFerry and Lens releases",
        r#""minimumReleaseAge": "0 days""#,
        r#""platformAutomerge": false"#,
        r#""boxferry-model""#,
        r#""compose-lens""#,
        r#""podman-lens""#,
        r#""quadlet-lens""#,
    ] {
        if !renovate.contains(required) {
            return Err(format!("Renovate configuration is missing `{required}`"));
        }
    }

    if renovate.matches(r#""automerge": false"#).count() != 4 {
        return Err("Renovate must keep Dev Container features, checksum tools, provider bootstrap pins, and hosted runners manual".to_owned());
    }

    for workflow_name in ["ci.yml"] {
        let workflow = read_repository_file(&format!(".github/workflows/{workflow_name}"))?;
        for required in [
            "renovate: datasource=crate depName=cargo-llvm-cov",
            "renovate: datasource=node-version depName=node",
        ] {
            if !workflow.contains(required) {
                return Err(format!("{workflow_name} is missing Renovate marker `{required}`"));
            }
        }
    }

    Ok(())
}

fn validate_renovate_lockfile_policy(renovate: &Value) -> Result<(), String> {
    if renovate["minimumReleaseAge"] != "3 days" {
        return Err("Renovate must retain the three-day minimum release age".to_owned());
    }
    let rules = renovate["packageRules"]
        .as_array()
        .ok_or_else(|| "Renovate packageRules must be an array".to_owned())?;
    let lock_matches = rules
        .iter()
        .enumerate()
        .filter(|(_, rule)| rule["description"] == "Automerge green-gated lock-file maintenance")
        .collect::<Vec<_>>();
    if lock_matches.len() != 1 {
        return Err("Renovate must keep exactly one lock-file maintenance rule".to_owned());
    }
    let (lock_index, lock_rule) = lock_matches[0];
    if lock_rule["matchUpdateTypes"] != serde_json::json!(["lockFileMaintenance"])
        || lock_rule["minimumReleaseAge"] != "0 days"
        || lock_rule["automerge"] != true
        || lock_rule["automergeType"] != "pr"
        || lock_rule["platformAutomerge"] != false
    {
        return Err("Renovate lock-file maintenance must be PR-based and green-gated".to_owned());
    }
    let generic_index = rules
        .iter()
        .position(|rule| rule["description"] == "Automerge tested non-major dependency updates")
        .ok_or_else(|| "Renovate generic non-major automerge rule is missing".to_owned())?;
    let generic_rule = &rules[generic_index];
    if generic_rule["matchUpdateTypes"] != serde_json::json!(["minor", "patch", "pin", "digest", "pinDigest"])
        || generic_rule["automerge"] != true
        || generic_rule["automergeType"] != "pr"
        || generic_rule["platformAutomerge"] != false
        || lock_index <= generic_index
    {
        return Err("Renovate automerge categories and ordering must remain exact and green-gated".to_owned());
    }
    for description in [
        "Keep Dev Container feature versions current; the lock file owns digests",
        "Require checksum review for downloaded file-quality tools",
    ] {
        let (index, rule) = rules
            .iter()
            .enumerate()
            .find(|(_, rule)| rule["description"] == description)
            .ok_or_else(|| format!("Renovate manual rule `{description}` is missing"))?;
        if index <= generic_index || index <= lock_index || rule["automerge"] != false {
            return Err(format!(
                "Renovate manual rule `{description}` must follow both automerge rules and disable it explicitly"
            ));
        }
    }
    validate_shared_policy_manager(renovate)
}

fn validate_shared_policy_manager(renovate: &Value) -> Result<(), String> {
    let managers = renovate["customManagers"]
        .as_array()
        .ok_or_else(|| "Renovate customManagers must be an array".to_owned())?
        .iter()
        .filter(|manager| manager["description"] == "Track the immutable Strukturpiloten shared-policy commit")
        .collect::<Vec<_>>();
    if managers.len() != 1
        || managers[0]["customType"] != "regex"
        || managers[0]["managerFilePatterns"] != serde_json::json!([r"/^\.github/workflows/.*\.ya?ml$/"])
    {
        return Err("Renovate must own exactly one immutable shared workflow revision marker".to_owned());
    }
    if managers[0]["datasourceTemplate"] != "github-digest" {
        return Err("shared-policy Renovate manager must declare the github-digest datasource".to_owned());
    }
    let pattern = managers[0]["matchStrings"][0]
        .as_str()
        .ok_or_else(|| "shared-policy Renovate manager must have one regex".to_owned())?;
    let template = managers[0]["autoReplaceStringTemplate"]
        .as_str()
        .ok_or_else(|| "shared-policy Renovate manager must have a replacement template".to_owned())?;
    if !template.contains('\n') || template.contains(r"\n") {
        return Err("shared-policy Renovate replacement must use a real newline".to_owned());
    }
    let new_digest = "b4a7d2e8f1c903b6a5d4e2f7182930c4b6d5e7f1";
    let rewritten = template
        .replace("{{{indentation}}}", "      ")
        .replace("{{{depName}}}", "Strukturpiloten/.github")
        .replace("{{{newValue}}}", "main")
        .replace("{{{newDigest}}}", new_digest);
    if reextract_shared_policy_marker(pattern, &rewritten) != Some(("Strukturpiloten/.github", "main", new_digest)) {
        return Err("shared-policy Renovate regex must re-extract its replacement".to_owned());
    }
    let workflow = read_repository_file(".github/workflows/ci.yml")?;
    let marker = "# renovate: datasource=github-digest depName=Strukturpiloten/.github currentValue=main";
    let lines = workflow.lines().collect::<Vec<_>>();
    let marker_line = lines
        .iter()
        .position(|line| line.trim() == marker)
        .ok_or_else(|| "CI shared-policy Renovate marker is missing".to_owned())?;
    let shared_ref = lines
        .get(marker_line + 1)
        .and_then(|line| line.trim().strip_prefix("ref: "))
        .ok_or_else(|| "CI shared-policy marker must be adjacent to its ref".to_owned())?;
    if workflow.matches(marker).count() != 1
        || shared_ref.len() != 40
        || !shared_ref
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err("CI must pin one full immutable shared workflow revision".to_owned());
    }
    Ok(())
}

fn reextract_shared_policy_marker<'a>(
    manager_pattern: &str,
    candidate: &'a str,
) -> Option<(&'a str, &'a str, &'a str)> {
    let expected_pattern = "(?<indentation>[ \\t]*)# renovate: datasource=github-digest depName=(?<depName>Strukturpiloten/\\.github) currentValue=(?<currentValue>main)\\n[ \\t]*ref:\\s*(?<currentDigest>[a-f0-9]{40})";
    if manager_pattern != expected_pattern {
        return None;
    }
    let (marker_line, ref_line) = candidate.split_once('\n')?;
    let (indentation, marker) = marker_line.split_once('#')?;
    if !indentation.bytes().all(|byte| matches!(byte, b' ' | b'\t')) {
        return None;
    }
    let marker = marker.strip_prefix(" renovate: datasource=github-digest depName=")?;
    let (dep_name, current_value) = marker.split_once(" currentValue=")?;
    if dep_name != "Strukturpiloten/.github" || current_value != "main" {
        return None;
    }
    let ref_line = ref_line.strip_prefix(indentation)?.strip_prefix("ref: ")?;
    if ref_line.len() != 40
        || !ref_line
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    {
        return None;
    }
    Some((dep_name, current_value, ref_line))
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

#[test]
fn public_documentation_is_source_owned_and_website_published() -> Result<(), String> {
    const PAGES: &[(&str, &[&str])] = &[
        ("docs/public/index.md", &["directly", "Rust API"]),
        ("docs/public/model/index.md", &["Typed document", "side effects"]),
        (
            "docs/public/parsing-rendering/index.md",
            &["interpolation", "render_canonical", "caller"],
        ),
        (
            "docs/public/diagnostics/index.md",
            &["machine-readable code", "source", "partial"],
        ),
        (
            "docs/public/compatibility/index.md",
            &["CompatibilityProfile", "evidence", "unknown"],
        ),
    ];

    let root = repository_root();
    let public_root = root.join("docs/public");
    let actual = walk_markdown_files(&public_root)?;
    let expected = PAGES.iter().map(|(path, _)| root.join(path)).collect::<BTreeSet<_>>();
    if actual != expected {
        return Err(format!(
            "docs/public must contain exactly the source-owned, website-published page inventory; expected {expected:#?}, found {actual:#?}"
        ));
    }

    for (path, topics) in PAGES {
        let document = read_repository_file(path)?;
        if document.lines().filter(|line| line.starts_with("# ")).count() != 1 {
            return Err(format!("{path} must contain exactly one level-one heading"));
        }
        if document.lines().count() > 90 {
            return Err(format!("{path} exceeds the 90-line public-page limit"));
        }
        for paragraph in document.split("\n\n") {
            if paragraph.lines().count() > 14 {
                return Err(format!("{path} contains a paragraph longer than 14 lines"));
            }
        }
        let lowercase = document.to_ascii_lowercase();
        for placeholder in ["todo", "coming soon", "lorem ipsum"] {
            if lowercase.contains(placeholder) {
                return Err(format!("{path} contains placeholder text `{placeholder}`"));
            }
        }
        for topic in *topics {
            if !document.contains(topic) {
                return Err(format!("{path} is missing required public topic `{topic}`"));
            }
        }
    }

    Ok(())
}

#[test]
fn maintained_documentation_is_bounded_current_and_nonduplicative() -> Result<(), String> {
    const ROOT_GUIDES: &[&str] = &[
        "docs/README.md",
        "docs/api-stability.md",
        "docs/architecture.md",
        "docs/coverage.md",
        "docs/dependency-policy.md",
        "docs/development-environment.md",
        "docs/environment-and-secrets.md",
        "docs/processing-model.md",
        "docs/releasing.md",
        "docs/rendering.md",
        "docs/testing.md",
    ];
    const NARRATIVE_DOCUMENTS: &[&str] = &[
        "README.md",
        "AGENTS.md",
        "docs/README.md",
        "docs/api-stability.md",
        "docs/architecture.md",
        "docs/coverage.md",
        "docs/dependency-policy.md",
        "docs/development-environment.md",
        "docs/environment-and-secrets.md",
        "docs/processing-model.md",
        "docs/releasing.md",
        "docs/rendering.md",
        "docs/testing.md",
        "docs/research/README.md",
        "conformance/README.md",
        "fixtures/README.md",
        "tests/README.md",
    ];
    const OBSOLETE_DOCUMENTS: &[&str] = &[
        "docs/conformance.md",
        "docs/fixture-format.md",
        "docs/generated-rendering.md",
        "docs/implementation-plan.md",
        "docs/preservation-editing.md",
        "docs/project-structure.md",
        "docs/quality-plan.md",
        "docs/real-world-corpus.md",
        "docs/render-formatting.md",
        "docs/roadmap.md",
        "docs/typed-model.md",
    ];

    let root = repository_root();
    let actual_root_guides = fs::read_dir(root.join("docs"))
        .map_err(|error| format!("failed to read docs: {error}"))?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.is_file() && path.extension().is_some_and(|extension| extension == "md"))
        .collect::<BTreeSet<_>>();
    let expected_root_guides = ROOT_GUIDES.iter().map(|path| root.join(path)).collect::<BTreeSet<_>>();
    if actual_root_guides != expected_root_guides {
        return Err(format!(
            "docs must contain exactly the maintained narrative guide inventory; expected {expected_root_guides:#?}, found {actual_root_guides:#?}"
        ));
    }

    for path in OBSOLETE_DOCUMENTS {
        if root.join(path).exists() {
            return Err(format!(
                "{path} is an obsolete duplicate ledger; keep its canonical replacement instead"
            ));
        }
    }

    for path in NARRATIVE_DOCUMENTS {
        validate_maintained_narrative(path)?;
    }

    let mut components = env!("CARGO_PKG_VERSION").split('.');
    let major = components
        .next()
        .ok_or_else(|| "package version has no major component".to_owned())?;
    let minor = components
        .next()
        .ok_or_else(|| "package version has no minor component".to_owned())?;
    let release_line = format!("{major}.{minor}.x");
    for path in ["docs/api-stability.md", "tests/README.md"] {
        let document = read_repository_file(path)?;
        if !document.contains(&release_line) {
            return Err(format!("{path} must identify the current {release_line} release line"));
        }
    }

    Ok(())
}

fn validate_maintained_narrative(path: &str) -> Result<(), String> {
    let document = read_repository_file(path)?;
    if document.lines().filter(|line| line.starts_with("# ")).count() != 1 {
        return Err(format!("{path} must contain exactly one level-one heading"));
    }
    if document.lines().count() > 220 {
        return Err(format!("{path} exceeds the 220-line maintained-guide limit"));
    }
    let word_limit = if path == "README.md" { 900 } else { 1_500 };
    let words = document.split_whitespace().count();
    if words > word_limit {
        return Err(format!(
            "{path} contains {words} words and exceeds its {word_limit}-word limit"
        ));
    }
    for paragraph in document.split("\n\n") {
        let paragraph_words = paragraph.split_whitespace().count();
        if paragraph_words > 180 {
            return Err(format!(
                "{path} contains a {paragraph_words}-word paragraph or list block; split or link it"
            ));
        }
    }
    for stale_phrase in ["Phase 2 typed", "supported 0.2.x", "0.2 consumer contract", "post-0.1"] {
        if document.contains(stale_phrase) {
            return Err(format!("{path} contains stale contract wording `{stale_phrase}`"));
        }
    }
    Ok(())
}

fn walk_markdown_files(root: &Path) -> Result<BTreeSet<PathBuf>, String> {
    let mut files = BTreeSet::new();
    let mut directories = vec![root.to_path_buf()];
    while let Some(directory) = directories.pop() {
        for entry in
            fs::read_dir(&directory).map_err(|error| format!("failed to read {}: {error}", directory.display()))?
        {
            let path = entry.map_err(|error| error.to_string())?.path();
            if path.is_dir() {
                directories.push(path);
            } else if path.extension().is_some_and(|extension| extension == "md") {
                files.insert(path);
            }
        }
    }
    Ok(files)
}

#[test]
fn agent_roles_are_explicit() -> Result<(), Box<dyn std::error::Error>> {
    let root = repository_root();
    let config = fs::read_to_string(root.join(".codex/config.toml"))?;
    for required in [
        "model = \"gpt-6-sol\"",
        "model_reasoning_effort = \"xhigh\"",
        "max_concurrent_threads_per_session = 9",
        "default_subagent_model = \"gpt-6-sol\"",
        "default_subagent_reasoning_effort = \"medium\"",
    ] {
        assert!(config.contains(required), "missing agent default: {required}");
    }
    for (role, model, effort, sandbox) in [
        ("implementation-worker", "gpt-6-sol", "high", "workspace-write"),
        ("specification-researcher", "gpt-6-sol", "high", "read-only"),
        ("reviewer", "gpt-6-sol", "high", "read-only"),
        ("verifier", "gpt-6-luna", "high", "workspace-write"),
    ] {
        let text = fs::read_to_string(root.join(format!(".codex/agents/{role}.toml")))?;
        for (key, value) in [
            ("model", model),
            ("model_reasoning_effort", effort),
            ("sandbox_mode", sandbox),
        ] {
            assert!(
                text.contains(&format!("{key} = \"{value}\"")),
                "{role}: incorrect {key}"
            );
        }
    }
    let reviewer = fs::read_to_string(root.join(".codex/agents/reviewer.toml"))?;
    assert!(reviewer.contains("original user requirements"));
    assert!(reviewer.contains("independent expected results"));
    let verifier = fs::read_to_string(root.join(".codex/agents/verifier.toml"))?;
    assert!(verifier.contains("./scripts/check-all.sh --check"));
    assert!(verifier.contains("never run the default formatting gate"));
    assert!(verifier.contains("Escalate complex failure diagnosis to the primary agent"));
    let instructions = fs::read_to_string(root.join("AGENTS.md"))?;
    for required in [
        "`gpt-6-sol` with `xhigh` reasoning",
        "use `gpt-6-sol` with `high` reasoning",
        "`gpt-6-luna` with `high`",
        "Use up to nine concurrent subagents plus the primary manager",
        "Do not create nested agents to evade the limit",
        "Run at most one complete gate or heavy runtime suite at a time",
        "reviewer checks the original requirements and independent expected results",
        "The primary owns integration, the final",
    ] {
        assert!(instructions.contains(required), "missing agent policy: {required}");
    }
    Ok(())
}

#[test]
fn standing_git_authorization_is_scoped_and_safeguarded() -> Result<(), Box<dyn std::error::Error>> {
    let instructions = fs::read_to_string(repository_root().join("AGENTS.md"))?;
    let section = instructions
        .split_once("## Workspace scope and standing GitHub authorization")
        .ok_or("missing standing authorization section")?
        .1
        .split("\n## ")
        .next()
        .ok_or("empty standing authorization section")?;
    let repositories: Vec<_> = section.lines().filter_map(|line| line.strip_prefix("- ")).collect();
    assert_eq!(
        repositories,
        [
            "`Strukturpiloten/boxferry`",
            "`Strukturpiloten/compose-lens`",
            "`Strukturpiloten/podman-lens`",
            "`Strukturpiloten/quadlet-lens`",
            "`Strukturpiloten/boxferry-website`",
            "`Strukturpiloten/docker-lens`",
        ]
    );
    for required in [
        "standing authorization for task-related Git and GitHub work only in these",
        "Do not work on or modify any repository outside this explicit allowlist",
        "An upstream documentation reference is not",
        "A newly discovered checkout is not implicitly",
        "the primary agent may create issues, branches, commits",
        "merge verified task-related pull requests without asking for renewed",
        "does not authorize unrelated backlog work",
        "discussion-only proposals",
        "may narrow or revoke this permission",
        "Immediately before merging, read back the exact head commit",
        "ready, mergeable, independently reviewed, and has every required check successful",
        "exact-head safeguard; never bypass branch protection",
        "override. Read back the merged state and merge commit, synchronize local `main`",
        "and remove the task's recorded worktrees and verified merged local branches",
        "unrelated work",
        "does not authorize releases, publication, deployment operations",
        "merging release/publication/deployment pull requests",
        "those require a separate explicit request",
        "The primary agent owns all Git and GitHub writes",
        "checkout and must not perform those writes",
    ] {
        assert!(
            section.contains(required),
            "missing authorization safeguard: {required}"
        );
    }
    assert!(!instructions.contains("Merge only when the user explicitly authorizes"));
    assert!(
        !instructions
            .contains("Authorization to run the Git workflow or perform GitHub writes does not authorize a merge")
    );
    Ok(())
}

// The full shell gate targets the Linux Dev Container.
// Keep configuration assertions above platform-independent.
#[cfg(target_os = "linux")]
#[test]
fn linux_gate_modes_and_failure_propagation_are_correct() -> Result<(), Box<dyn std::error::Error>> {
    let root = repository_root();
    let result = Command::new("bash")
        .arg("scripts/test-check-all.sh")
        .current_dir(root)
        .output()?;
    assert!(
        result.status.success(),
        "gate mode regression failed:\n{}\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    Ok(())
}
