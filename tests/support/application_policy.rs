//! Repository policy for imported real-application contracts and retained evidence.

use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsStr;
use std::fs;
use std::path::{Component, Path, PathBuf};
use toml::{Table, Value};

const NEXTCLOUD_FIXTURE: &str = "boxferry-nextcloud-application";
const NEXTCLOUD_REVISION: &str = "18716257362932e7520d5aed68412ac7c8492e6d";
const NEXTCLOUD_SCENARIO_SHA256: &str = "9ed20c06a3b2b340833e5f660e47725593b84540f3b473009bffe7f69df6536f";
const NEXTCLOUD_ACCEPTANCE_SHA256: &str = "79dc3dc478b710a9545a010a7e0f1338628c2c352e446dcfaef039238ea6447d";
const NEXTCLOUD_INTERPOLATION_SHA256: &str = "b9e1182df0d7aee94a3b3758b27501649d0aa6a1ee2b52683f81baf3ddc95eac";
const NEXTCLOUD_PROVIDER_SHA256: &str = "ca714ddfe9b64620faf47c714db6be2907f7ec6428529c3e041bfd123bdd4761";

const FORGEJO_FIXTURE: &str = "boxferry-forgejo-application";
const FORGEJO_REVISION: &str = "68e08b9a86bcb9ced34e8d831563e732176275a1";
const FORGEJO_SOURCE_HEAD: &str = "1f4ab7ef5d31b6940fc4b296d1e2eac2650cf180";
const FORGEJO_SCENARIO_SHA256: &str = "20f199e9d46d163bd4f3b8b6c3022e816b7f1542b7c59329ef5628fd1eb188f6";
const FORGEJO_INTERPOLATION_SHA256: &str = "5de63b3ad5bafb6535ff49806a1ef737e25d3b32ee81257dcfb236db035cb4ce";
const FORGEJO_PROVIDER_SHA256: &str = "2d7e81eeefc7060812900791db0a3a9fef08b748779f8697fef12f0cced4d5be";
const FORGEJO_PEER_SHA256: &str = "14a22ea3ab8f4941b2611c2a983a349cd69f8a141cce239cc4dbaa8469bca6c8";

const PROVIDER_ID: &str = "docker-compose-5-5-0";
const PROVIDER_VERSION: &str = "5.5.0";
const PROVIDER_ARTIFACT_URL: &str =
    "https://github.com/docker/compose/releases/download/v5.5.0/docker-compose-linux-x86_64";
const PROVIDER_ARTIFACT_SHA256: &str = "c57ab918abd5b05ca7e7d0f275875dd1330a695074f309dc9eab1b49efafcd4b";
const PODMAN_ROOTLESS_IMAGE: &str = "ghcr.io/strukturpiloten/podman-6.1-rootless:v6.1.0@sha256:dd00fadfff6e732728643df565a5db50f6d36dc3ec2d7f23a1fe87e905e08b5e";
const PODMAN_ARCH_ROOTFUL_IMAGE: &str = "ghcr.io/strukturpiloten/podman-arch-rootful:v1.0.0@sha256:2b352f802035f1667f0bdb4f2f25715b97fd65f70d281b025049dc10e7d7d451";
const EVIDENCE_PATH: &str = "conformance/application-evidence.toml";
const EVIDENCE_RECORD_DIRECTORY: &str = "conformance/records/application-2026-09-08";

pub(crate) fn validate_application_fixtures() -> Result<(), String> {
    validate_application_fixture_hashes(
        NEXTCLOUD_FIXTURE,
        NEXTCLOUD_REVISION,
        &[
            (
                "scenario.compose.yaml",
                "scenario-compose-sha256",
                NEXTCLOUD_SCENARIO_SHA256,
            ),
            (
                "scenario.acceptance.yaml",
                "scenario-acceptance-sha256",
                NEXTCLOUD_ACCEPTANCE_SHA256,
            ),
            (
                "interpolation.env",
                "interpolation-sha256",
                NEXTCLOUD_INTERPOLATION_SHA256,
            ),
            (
                "provider.compose.yaml",
                "provider-compose-sha256",
                NEXTCLOUD_PROVIDER_SHA256,
            ),
        ],
    )?;

    validate_application_fixture_hashes(
        FORGEJO_FIXTURE,
        FORGEJO_REVISION,
        &[
            (
                "scenario.compose.yaml",
                "scenario-compose-sha256",
                FORGEJO_SCENARIO_SHA256,
            ),
            (
                "interpolation.env",
                "interpolation-sha256",
                FORGEJO_INTERPOLATION_SHA256,
            ),
            (
                "provider.compose.yaml",
                "provider-compose-sha256",
                FORGEJO_PROVIDER_SHA256,
            ),
            (
                "provider-peer.compose.yaml",
                "provider-peer-compose-sha256",
                FORGEJO_PEER_SHA256,
            ),
        ],
    )?;

    Ok(())
}

pub(crate) fn validate_application_independence_and_gates() -> Result<(), String> {
    let cargo_manifest = read_repository_file("Cargo.toml")?;
    let manifest = parse_table("Cargo.toml", &cargo_manifest)?;
    reject_dependency_named_boxferry(&manifest, "Cargo.toml")?;

    let cargo_lock = read_repository_file("Cargo.lock")?;
    let lock = parse_table("Cargo.lock", &cargo_lock)?;
    let packages = lock
        .get("package")
        .and_then(Value::as_array)
        .ok_or_else(|| "Cargo.lock must contain package records".to_owned())?;
    for package in packages {
        let package = package
            .as_table()
            .ok_or_else(|| "Cargo.lock package records must be tables".to_owned())?;
        if package.get("name").and_then(Value::as_str) == Some("boxferry") {
            return Err("ComposeLens must not resolve a BoxFerry Cargo package".to_owned());
        }
    }

    let cargo_config = parse_table(".cargo/config.toml", &read_repository_file(".cargo/config.toml")?)?;
    let aliases = required_table(&cargo_config, "alias", ".cargo/config.toml")?;
    require_exact_string(
        aliases,
        "ci-application",
        "test --locked --test application_conformance",
        ".cargo/config.toml [alias]",
    )?;

    for (path, required) in [
        ("scripts/check-all.sh", "cargo ci-application"),
        (".github/workflows/ci.yml", "run: cargo ci-application"),
        (".github/workflows/release.yml", "run: cargo ci-application"),
    ] {
        let source = read_repository_file(path)?;
        let count = source.matches(required).count();
        if count != 1 {
            return Err(format!(
                "{path} must contain exactly one explicit application gate `{required}`, found {count}"
            ));
        }
    }

    Ok(())
}

pub(crate) fn validate_application_evidence() -> Result<(), String> {
    let evidence = parse_repository_table(EVIDENCE_PATH)?;
    require_exact_keys(
        &evidence,
        &["schema", "reviewed-at", "scope", "provider", "runtime"],
        EVIDENCE_PATH,
    )?;
    require_exact_integer(&evidence, "schema", 1, EVIDENCE_PATH)?;
    require_date(&evidence, "reviewed-at", EVIDENCE_PATH)?;
    require_nonempty_string(&evidence, "scope", EVIDENCE_PATH)?;

    let provider = required_table(&evidence, "provider", EVIDENCE_PATH)?;
    validate_provider_matrix(provider)?;

    let runtime_rows = required_table_array(&evidence, "runtime", EVIDENCE_PATH)?;
    if runtime_rows.len() != 4 {
        return Err(format!(
            "{EVIDENCE_PATH} must contain exactly four runtime rows, found {}",
            runtime_rows.len()
        ));
    }

    let mut rows = BTreeMap::new();
    let mut referenced_records = BTreeSet::new();
    referenced_records.insert(require_record_path(provider, "provider.record")?.to_owned());
    for row in runtime_rows {
        validate_runtime_matrix_row(row, &mut referenced_records)?;
        let id = required_string(row, "id", "runtime row")?;
        if rows.insert(id.to_owned(), row).is_some() {
            return Err(format!("duplicate application runtime row `{id}`"));
        }
    }
    require_exact_runtime_matrix(&rows)?;

    let provider_record_path = required_string(provider, "record", "provider")?;
    let provider_record = parse_repository_table(provider_record_path)?;
    validate_provider_record(&provider_record, provider)?;

    let nextcloud_record_path =
        required_string(rows["nextcloud-podman-6-1-rootless"], "record", "nextcloud runtime row")?;
    let nextcloud_record = parse_repository_table(nextcloud_record_path)?;
    validate_nextcloud_runtime_record(&nextcloud_record, &[rows["nextcloud-podman-6-1-rootless"]])?;

    let forgejo_record_path = required_string(rows["forgejo-podman-arch-rootful"], "record", "Forgejo runtime row")?;
    if required_string(
        rows["forgejo-podman-6-1-rootless"],
        "record",
        "Forgejo rootless runtime row",
    )? != forgejo_record_path
    {
        return Err("both Forgejo runtime rows must cite the same reviewed job record".to_owned());
    }
    let forgejo_record = parse_repository_table(forgejo_record_path)?;
    validate_forgejo_runtime_record(
        &forgejo_record,
        &[rows["forgejo-podman-arch-rootful"], rows["forgejo-podman-6-1-rootless"]],
    )?;

    let actual_records = list_toml_files(EVIDENCE_RECORD_DIRECTORY)?;
    if actual_records != referenced_records {
        return Err(format!(
            "application record directory must contain exactly the referenced records; referenced {referenced_records:?}, found {actual_records:?}"
        ));
    }

    Ok(())
}

fn validate_provider_matrix(provider: &Table) -> Result<(), String> {
    require_exact_keys(
        provider,
        &[
            "id",
            "version",
            "release-url",
            "artifact-url",
            "artifact-sha256",
            "record",
        ],
        "application provider matrix",
    )?;
    require_exact_string(provider, "id", PROVIDER_ID, "application provider matrix")?;
    require_exact_string(provider, "version", PROVIDER_VERSION, "application provider matrix")?;
    require_exact_string(
        provider,
        "release-url",
        "https://github.com/docker/compose/releases/tag/v5.5.0",
        "application provider matrix",
    )?;
    require_exact_string(
        provider,
        "artifact-url",
        PROVIDER_ARTIFACT_URL,
        "application provider matrix",
    )?;
    require_exact_string(
        provider,
        "artifact-sha256",
        PROVIDER_ARTIFACT_SHA256,
        "application provider matrix",
    )?;
    require_sha256(provider, "artifact-sha256", "application provider matrix")?;
    require_record_path(provider, "provider.record")?;
    Ok(())
}

fn validate_runtime_matrix_row(row: &Table, records: &mut BTreeSet<String>) -> Result<(), String> {
    let id = required_string(row, "id", "runtime row")?;
    let state = required_string(row, "state", id)?;
    let mut keys = vec!["id", "application", "state", "provider", "runtime-version", "root-mode"];
    match state {
        "observed" => keys.extend(["record", "record-context"]),
        "planned" => keys.push("reason"),
        other => return Err(format!("{id}: unsupported evidence state `{other}`")),
    }
    require_exact_keys(row, &keys, id)?;
    require_one_of(row, "application", &["nextcloud", "forgejo"], id)?;
    require_exact_string(row, "provider", PROVIDER_ID, id)?;
    require_version(row, "runtime-version", id)?;
    require_one_of(row, "root-mode", &["rootless", "rootful"], id)?;
    if state == "observed" {
        records.insert(require_record_path(row, id)?.to_owned());
        require_nonempty_string(row, "record-context", id)?;
    } else {
        require_nonempty_string(row, "reason", id)?;
        if row.contains_key("record") {
            return Err(format!("{id}: planned rows must not cite an observation record"));
        }
    }
    Ok(())
}

fn require_exact_runtime_matrix(rows: &BTreeMap<String, &Table>) -> Result<(), String> {
    let expected = BTreeSet::from([
        "nextcloud-podman-6-1-rootless",
        "nextcloud-podman-arch-rootful",
        "forgejo-podman-arch-rootful",
        "forgejo-podman-6-1-rootless",
    ]);
    if rows.keys().map(String::as_str).collect::<BTreeSet<_>>() != expected {
        return Err(format!(
            "application runtime matrix IDs differ from the reviewed set: {:?}",
            rows.keys().collect::<Vec<_>>()
        ));
    }
    for (id, application, state, version, mode) in [
        (
            "nextcloud-podman-6-1-rootless",
            "nextcloud",
            "observed",
            "6.1.0",
            "rootless",
        ),
        (
            "nextcloud-podman-arch-rootful",
            "nextcloud",
            "planned",
            "6.1.0-1",
            "rootful",
        ),
        (
            "forgejo-podman-arch-rootful",
            "forgejo",
            "observed",
            "6.1.0-1",
            "rootful",
        ),
        (
            "forgejo-podman-6-1-rootless",
            "forgejo",
            "observed",
            "6.1.0",
            "rootless",
        ),
    ] {
        let row = rows[id];
        require_exact_string(row, "application", application, id)?;
        require_exact_string(row, "state", state, id)?;
        require_exact_string(row, "runtime-version", version, id)?;
        require_exact_string(row, "root-mode", mode, id)?;
    }
    Ok(())
}

fn validate_provider_record(record: &Table, matrix: &Table) -> Result<(), String> {
    const CONTEXT: &str = "application provider record";
    require_exact_keys(
        record,
        &[
            "schema",
            "kind",
            "captured-at",
            "provider-id",
            "provider-version",
            "artifact-url",
            "artifact-sha256",
            "version-command",
            "version-output",
            "runtime-invoked",
            "network-access-after-download",
            "isolated-environment",
            "privacy-reviewed",
            "normalized-output-retained",
            "normalization-token",
            "normalization-rule",
            "project-directory-policy",
            "credential-review",
            "process-environment",
            "result",
            "limitations",
        ],
        CONTEXT,
    )?;
    require_exact_integer(record, "schema", 2, CONTEXT)?;
    require_exact_string(record, "kind", "provider-config", CONTEXT)?;
    require_date(record, "captured-at", CONTEXT)?;
    require_exact_string(record, "provider-id", PROVIDER_ID, CONTEXT)?;
    require_exact_string(record, "provider-version", PROVIDER_VERSION, CONTEXT)?;
    require_exact_string(record, "artifact-url", PROVIDER_ARTIFACT_URL, CONTEXT)?;
    require_exact_string(record, "artifact-sha256", PROVIDER_ARTIFACT_SHA256, CONTEXT)?;
    require_sha256(record, "artifact-sha256", CONTEXT)?;
    require_nonempty_string(record, "version-command", CONTEXT)?;
    require_exact_string(record, "version-output", PROVIDER_VERSION, CONTEXT)?;
    require_exact_bool(record, "runtime-invoked", false, CONTEXT)?;
    require_exact_bool(record, "network-access-after-download", false, CONTEXT)?;
    require_exact_bool(record, "isolated-environment", true, CONTEXT)?;
    require_exact_bool(record, "privacy-reviewed", true, CONTEXT)?;
    require_exact_bool(record, "normalized-output-retained", true, CONTEXT)?;
    require_nonempty_string(record, "normalization-token", CONTEXT)?;
    require_nonempty_string(record, "normalization-rule", CONTEXT)?;
    require_nonempty_string(record, "project-directory-policy", CONTEXT)?;
    require_nonempty_string(record, "credential-review", CONTEXT)?;

    let environment = required_string_array(record, "process-environment", CONTEXT)?;
    for required in [
        "PATH=/usr/bin:/bin",
        "HOME=",
        "XDG_CONFIG_HOME=",
        "XDG_CACHE_HOME=",
        "XDG_RUNTIME_DIR=",
        "DOCKER_CONFIG=",
        "TMPDIR=",
        "COMPOSE_DISABLE_ENV_FILE=1",
        "DOCKER_HOST=",
    ] {
        if !environment.iter().any(|value| value.starts_with(required)) {
            return Err(format!("{CONTEXT}: process environment missing {required}"));
        }
    }

    for (matrix_field, field) in [
        ("artifact-url", "artifact-url"),
        ("artifact-sha256", "artifact-sha256"),
        ("version", "provider-version"),
        ("id", "provider-id"),
    ] {
        if required_string(matrix, matrix_field, "provider matrix")? != required_string(record, field, CONTEXT)? {
            return Err(format!("{CONTEXT}: {field} must match provider matrix {matrix_field}"));
        }
    }

    validate_provider_results(required_table_array(record, "result", CONTEXT)?)?;

    let limitations = required_table(record, "limitations", CONTEXT)?;
    require_exact_keys(limitations, &["statement"], "provider limitations")?;
    let statement = required_string(limitations, "statement", "provider limitations")?;
    for denied in ["No create", "daemon socket", "container", "network", "volume"] {
        if !statement.contains(denied) {
            return Err(format!("provider limitations must explicitly retain boundary {denied}"));
        }
    }
    Ok(())
}

fn validate_provider_results(results: Vec<&Table>) -> Result<(), String> {
    let expected = BTreeSet::from([
        "forgejo-provider-application",
        "forgejo-provider-peer",
        "nextcloud-provider",
        "nextcloud-scenario-merge",
        "forgejo-processing-all-profiles",
        "forgejo-processing-standard-consistency",
    ]);
    let mut actual = BTreeSet::new();

    for result in results {
        let id = required_string(result, "id", "provider result")?;
        if !actual.insert(id) {
            return Err(format!("duplicate provider result {id}"));
        }
        validate_provider_result(result, id)?;
    }

    if actual != expected {
        return Err(format!(
            "application provider result IDs differ from reviewed set: {actual:?}"
        ));
    }
    Ok(())
}

fn validate_provider_result(result: &Table, id: &str) -> Result<(), String> {
    let (stdout_state, stderr_state) = validate_provider_result_schema(result, id)?;
    let (project, project_directory) = validate_provider_result_identity(result, id)?;
    validate_provider_invocation(result, id, project, project_directory)?;
    validate_provider_streams(result, id, stdout_state, stderr_state)?;
    Ok(())
}

fn validate_provider_result_schema<'a>(result: &'a Table, id: &str) -> Result<(&'a str, &'a str), String> {
    let stdout_state = required_string(result, "stdout-state", id)?;
    let stderr_state = required_string(result, "stderr-state", id)?;
    let mut keys = vec![
        "id",
        "fixture-id",
        "fixture-revision",
        "project-name",
        "project-directory",
        "env-file",
        "env-file-raw-sha256",
        "env-file-record",
        "env-file-sha256",
        "argv",
        "exit-code",
        "raw-stdout-sha256",
        "stdout-state",
        "stdout-sha256",
        "raw-stderr-sha256",
        "stderr-state",
        "stderr-sha256",
        "assertions",
        "input",
    ];
    if stdout_state == "retained" {
        keys.push("stdout-record");
    }
    if stderr_state == "retained" {
        keys.push("stderr-record");
    }
    require_exact_keys(result, &keys, id)?;
    Ok((stdout_state, stderr_state))
}

fn validate_provider_result_identity<'a>(result: &'a Table, id: &str) -> Result<(&'static str, &'a str), String> {
    let (fixture, revision, project, directory, exit_code) = match id {
        "forgejo-provider-application" => (
            FORGEJO_FIXTURE,
            FORGEJO_REVISION,
            "compose-lens-audit-forgejo-provider",
            "/inputs/forgejo",
            0,
        ),
        "forgejo-provider-peer" => (
            FORGEJO_FIXTURE,
            FORGEJO_REVISION,
            "compose-lens-audit-forgejo-peer",
            "/inputs/forgejo",
            0,
        ),
        "nextcloud-provider" => (
            NEXTCLOUD_FIXTURE,
            NEXTCLOUD_REVISION,
            "compose-lens-audit-nextcloud-provider",
            "/inputs/nextcloud",
            0,
        ),
        "nextcloud-scenario-merge" => (
            NEXTCLOUD_FIXTURE,
            NEXTCLOUD_REVISION,
            "compose-lens-audit-nextcloud-scenario",
            "/inputs/nextcloud",
            0,
        ),
        "forgejo-processing-all-profiles" => (
            FORGEJO_FIXTURE,
            FORGEJO_REVISION,
            "compose-lens-audit-forgejo-processing",
            "/inputs/forgejo",
            0,
        ),
        "forgejo-processing-standard-consistency" => (
            FORGEJO_FIXTURE,
            FORGEJO_REVISION,
            "compose-lens-audit-forgejo-processing",
            "/inputs/forgejo",
            1,
        ),
        other => return Err(format!("unknown provider result {other}")),
    };
    require_exact_string(result, "fixture-id", fixture, id)?;
    require_exact_string(result, "fixture-revision", revision, id)?;
    require_exact_string(result, "project-name", project, id)?;
    let project_directory = required_string(result, "project-directory", id)?;
    if !project_directory.contains("CAPTURE_ROOT") || !project_directory.ends_with(directory) {
        return Err(format!("{id}: project directory differs from reviewed policy"));
    }
    require_exact_integer(result, "exit-code", exit_code, id)?;
    for field in [
        "env-file-raw-sha256",
        "env-file-sha256",
        "raw-stdout-sha256",
        "stdout-sha256",
        "raw-stderr-sha256",
        "stderr-sha256",
    ] {
        require_sha256(result, field, id)?;
    }
    Ok((project, project_directory))
}

fn validate_provider_invocation(
    result: &Table,
    id: &str,
    project: &str,
    project_directory: &str,
) -> Result<(), String> {
    validate_recorded_hash(
        required_string(result, "env-file-record", id)?,
        required_string(result, "env-file-sha256", id)?,
        id,
    )?;

    let argv = required_string_array(result, "argv", id)?;
    if argv.len() < 12
        || !argv[0].ends_with("docker-compose-linux-x86_64")
        || argv[1..3] != ["--project-name", project]
        || argv[3..5] != ["--project-directory", project_directory]
        || argv[5] != "--env-file"
        || argv[6] != required_string(result, "env-file", id)?
        || !argv.contains(&"config")
        || argv[argv.len() - 2..] != ["--format", "yaml"]
    {
        return Err(format!("{id}: full argv is missing or inconsistent"));
    }

    let inputs = required_table_array(result, "input", id)?;
    if inputs.is_empty() {
        return Err(format!("{id}: input list must not be empty"));
    }
    for input in inputs {
        require_exact_keys(input, &["path", "role", "capture-path", "sha256"], id)?;
        require_one_of(input, "role", &["compose-file", "referenced-env-file"], id)?;
        require_nonempty_string(input, "capture-path", id)?;
        require_sha256(input, "sha256", id)?;
        validate_recorded_hash(
            required_string(input, "path", id)?,
            required_string(input, "sha256", id)?,
            id,
        )?;
    }
    Ok(())
}

fn validate_provider_streams(result: &Table, id: &str, stdout_state: &str, stderr_state: &str) -> Result<(), String> {
    require_one_of(result, "stdout-state", &["retained", "empty"], id)?;
    require_one_of(result, "stderr-state", &["retained", "empty"], id)?;
    if stdout_state == "retained" {
        validate_recorded_hash(
            required_string(result, "stdout-record", id)?,
            required_string(result, "stdout-sha256", id)?,
            id,
        )?;
    } else if required_string(result, "stdout-sha256", id)? != EMPTY_SHA256 {
        return Err(format!("{id}: empty stdout must use the empty SHA-256"));
    }
    if stderr_state == "retained" {
        validate_recorded_hash(
            required_string(result, "stderr-record", id)?,
            required_string(result, "stderr-sha256", id)?,
            id,
        )?;
    } else if required_string(result, "stderr-sha256", id)? != EMPTY_SHA256 {
        return Err(format!("{id}: empty stderr must use the empty SHA-256"));
    }
    require_nonempty_string_array(result, "assertions", id)?;
    Ok(())
}

const EMPTY_SHA256: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

fn validate_recorded_hash(path: &str, expected: &str, context: &str) -> Result<(), String> {
    if !is_safe_relative(path) {
        return Err(format!("{context}: unsafe evidence path {path}"));
    }
    let source = read_repository_file(path)?;
    let actual = format!("{:x}", Sha256::digest(source.as_bytes()));
    if actual != expected {
        return Err(format!("{context}: {path} hashes to {actual}, expected {expected}"));
    }
    if path.starts_with("conformance/records/application-2026-09-08/provider-config/")
        && (source.contains("/tmp/compose-lens-provider-audit.")
            || source.contains("/home/")
            || source.contains("/Users/"))
    {
        return Err(format!("{context}: {path} contains an unnormalized private path"));
    }
    Ok(())
}

fn required_string_array<'a>(table: &'a Table, field: &str, context: &str) -> Result<Vec<&'a str>, String> {
    table
        .get(field)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{context}: {field} must be an array"))?
        .iter()
        .map(|value| {
            value
                .as_str()
                .filter(|value| !value.is_empty())
                .ok_or_else(|| format!("{context}: {field} must contain non-empty strings"))
        })
        .collect()
}

fn validate_nextcloud_runtime_record(record: &Table, rows: &[&Table]) -> Result<(), String> {
    const CONTEXT: &str = "Nextcloud runtime record";
    require_exact_keys(
        record,
        &[
            "schema",
            "kind",
            "reviewed-at",
            "state",
            "source-repository",
            "source-head",
            "source-merge",
            "workflow-run",
            "job",
            "job-name",
            "job-started-at",
            "job-completed-at",
            "provider-version",
            "provider-sha256",
            "runtime-version",
            "runtime-image",
            "root-mode",
            "architecture",
            "selinux",
            "cleanup",
            "resource-audit",
            "privacy-reviewed",
            "fixture-scenario-sha256",
            "fixture-acceptance-sha256",
            "fixture-interpolation-sha256",
            "fixture-provider-sha256",
            "assertions",
            "unobserved",
            "context",
        ],
        CONTEXT,
    )?;
    validate_runtime_record_common(record, CONTEXT)?;
    require_exact_string(record, "source-head", FORGEJO_SOURCE_HEAD, CONTEXT)?;
    require_exact_string(record, "source-merge", FORGEJO_REVISION, CONTEXT)?;
    require_exact_string(record, "runtime-version", "6.1.0", CONTEXT)?;
    require_exact_string(record, "runtime-image", PODMAN_ROOTLESS_IMAGE, CONTEXT)?;
    require_exact_string(record, "root-mode", "rootless", CONTEXT)?;
    require_exact_string(record, "architecture", "amd64", CONTEXT)?;
    require_exact_string(record, "selinux", "unobserved", CONTEXT)?;
    require_exact_string(record, "fixture-scenario-sha256", NEXTCLOUD_SCENARIO_SHA256, CONTEXT)?;
    require_exact_string(
        record,
        "fixture-acceptance-sha256",
        NEXTCLOUD_ACCEPTANCE_SHA256,
        CONTEXT,
    )?;
    require_exact_string(
        record,
        "fixture-interpolation-sha256",
        NEXTCLOUD_INTERPOLATION_SHA256,
        CONTEXT,
    )?;
    require_exact_string(record, "fixture-provider-sha256", NEXTCLOUD_PROVIDER_SHA256, CONTEXT)?;
    validate_runtime_context_links(record, rows, CONTEXT)?;
    Ok(())
}

fn validate_forgejo_runtime_record(record: &Table, rows: &[&Table]) -> Result<(), String> {
    const CONTEXT: &str = "Forgejo runtime record";
    require_exact_keys(
        record,
        &[
            "schema",
            "kind",
            "reviewed-at",
            "state",
            "source-repository",
            "source-head",
            "source-merge",
            "workflow-run",
            "job",
            "job-name",
            "job-started-at",
            "job-completed-at",
            "provider-version",
            "provider-sha256",
            "cleanup",
            "resource-audit",
            "privacy-reviewed",
            "fixture-scenario-sha256",
            "fixture-interpolation-sha256",
            "fixture-provider-sha256",
            "fixture-peer-sha256",
            "assertions",
            "unobserved",
            "context",
        ],
        CONTEXT,
    )?;
    validate_runtime_record_common(record, CONTEXT)?;
    require_exact_string(record, "source-head", FORGEJO_SOURCE_HEAD, CONTEXT)?;
    require_exact_string(record, "source-merge", FORGEJO_REVISION, CONTEXT)?;
    require_exact_string(record, "fixture-scenario-sha256", FORGEJO_SCENARIO_SHA256, CONTEXT)?;
    require_exact_string(
        record,
        "fixture-interpolation-sha256",
        FORGEJO_INTERPOLATION_SHA256,
        CONTEXT,
    )?;
    require_exact_string(record, "fixture-provider-sha256", FORGEJO_PROVIDER_SHA256, CONTEXT)?;
    require_exact_string(record, "fixture-peer-sha256", FORGEJO_PEER_SHA256, CONTEXT)?;

    let contexts = required_table_array(record, "context", CONTEXT)?;
    if contexts.len() != 2 {
        return Err(format!("{CONTEXT} must contain exactly rootful and rootless contexts"));
    }
    let mut seen = BTreeSet::new();
    for context in contexts {
        let id = required_string(context, "id", "Forgejo runtime context")?;
        require_exact_keys(
            context,
            &["id", "runtime-version", "runtime-image", "root-mode", "architecture"],
            id,
        )?;
        require_exact_string(context, "architecture", "amd64", id)?;
        match id {
            "podman-arch-rootful" => {
                require_exact_string(context, "runtime-version", "6.1.0-1", id)?;
                require_exact_string(context, "root-mode", "rootful", id)?;
                require_exact_string(context, "runtime-image", PODMAN_ARCH_ROOTFUL_IMAGE, id)?;
            }
            "podman-6.1-rootless" => {
                require_exact_string(context, "runtime-version", "6.1.0", id)?;
                require_exact_string(context, "root-mode", "rootless", id)?;
                require_exact_string(context, "runtime-image", PODMAN_ROOTLESS_IMAGE, id)?;
            }
            other => return Err(format!("unknown Forgejo runtime context `{other}`")),
        }
        if !seen.insert(id) {
            return Err(format!("duplicate Forgejo runtime context `{id}`"));
        }
    }
    validate_runtime_context_links(record, rows, CONTEXT)?;
    Ok(())
}

fn validate_runtime_context_links(record: &Table, rows: &[&Table], record_name: &str) -> Result<(), String> {
    let contexts = required_table_array(record, "context", record_name)?;
    if contexts.len() != rows.len() {
        return Err(format!(
            "{record_name}: expected {} referenced contexts, found {}",
            rows.len(),
            contexts.len()
        ));
    }
    let mut by_id = BTreeMap::new();
    for context in contexts {
        let id = required_string(context, "id", record_name)?;
        if by_id.insert(id, context).is_some() {
            return Err(format!("{record_name}: duplicate context `{id}`"));
        }
    }
    for row in rows {
        let row_id = required_string(row, "id", "runtime matrix row")?;
        let context_id = required_string(row, "record-context", row_id)?;
        let context = by_id
            .get(context_id)
            .ok_or_else(|| format!("{row_id}: record does not contain context `{context_id}`"))?;
        require_exact_keys(
            context,
            &["id", "runtime-version", "runtime-image", "root-mode", "architecture"],
            context_id,
        )?;
        require_exact_string(
            context,
            "runtime-version",
            required_string(row, "runtime-version", row_id)?,
            context_id,
        )?;
        require_exact_string(
            context,
            "root-mode",
            required_string(row, "root-mode", row_id)?,
            context_id,
        )?;
        require_exact_string(context, "architecture", "amd64", context_id)?;
        let expected_image = match context_id {
            "podman-arch-rootful" => PODMAN_ARCH_ROOTFUL_IMAGE,
            "podman-6.1-rootless" => PODMAN_ROOTLESS_IMAGE,
            other => return Err(format!("{row_id}: unknown runtime context `{other}`")),
        };
        require_exact_string(context, "runtime-image", expected_image, context_id)?;
    }
    Ok(())
}

fn validate_runtime_record_common(record: &Table, context: &str) -> Result<(), String> {
    require_exact_integer(record, "schema", 1, context)?;
    require_exact_string(record, "kind", "runtime-effect", context)?;
    require_date(record, "reviewed-at", context)?;
    require_exact_string(record, "state", "observed", context)?;
    require_exact_string(
        record,
        "source-repository",
        "https://github.com/Strukturpiloten/boxferry",
        context,
    )?;
    require_commit(record, "source-head", context)?;
    require_commit(record, "source-merge", context)?;
    let workflow = required_string(record, "workflow-run", context)?;
    if !workflow.starts_with("https://github.com/Strukturpiloten/boxferry/actions/runs/") {
        return Err(format!("{context}: workflow-run must be a BoxFerry Actions run URL"));
    }
    let job = required_string(record, "job", context)?;
    if !job.starts_with(&format!("{workflow}/job/")) {
        return Err(format!("{context}: job must belong to the recorded workflow run"));
    }
    require_nonempty_string(record, "job-name", context)?;
    require_timestamp(record, "job-started-at", context)?;
    require_timestamp(record, "job-completed-at", context)?;
    require_exact_string(record, "provider-version", PROVIDER_VERSION, context)?;
    require_exact_string(record, "provider-sha256", PROVIDER_ARTIFACT_SHA256, context)?;
    require_exact_string(record, "cleanup", "passed", context)?;
    require_exact_string(record, "resource-audit", "passed", context)?;
    require_exact_bool(record, "privacy-reviewed", true, context)?;
    require_nonempty_string_array(record, "assertions", context)?;
    require_nonempty_string_array(record, "unobserved", context)?;
    Ok(())
}

fn validate_application_fixture_hashes(id: &str, revision: &str, copied: &[(&str, &str, &str)]) -> Result<(), String> {
    let fixture_root = repository_root().join("fixtures/real-world").join(id);
    let manifest_path = fixture_root.join("fixture.toml");
    let manifest_source = fs::read_to_string(&manifest_path)
        .map_err(|error| format!("failed to read {}: {error}", manifest_path.display()))?;
    let manifest = parse_table(&manifest_path.display().to_string(), &manifest_source)?;
    require_exact_string(&manifest, "id", id, &format!("{id} fixture manifest"))?;
    require_exact_string(&manifest, "suite", "real-world", &format!("{id} fixture manifest"))?;
    let provenance = required_table(&manifest, "provenance", id)?;
    require_exact_string(provenance, "source", "external", id)?;
    require_exact_string(provenance, "revision", revision, id)?;
    let provenance_url = required_string(provenance, "url", id)?;
    if !provenance_url.starts_with("https://github.com/Strukturpiloten/boxferry/tree/")
        || !provenance_url.contains(revision)
    {
        return Err(format!(
            "{id}: provenance URL must pin the reviewed BoxFerry revision {revision}"
        ));
    }
    let application = required_table(required_table(&manifest, "extensions", id)?, "application", id)?;
    require_exact_string(
        application,
        "source-repository",
        "https://github.com/Strukturpiloten/boxferry",
        id,
    )?;
    let listed = manifest
        .get("files")
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{id}: `files` must be an array"))?;
    let listed = listed
        .iter()
        .map(|value| {
            value
                .as_str()
                .ok_or_else(|| format!("{id}: fixture files must be strings"))
        })
        .collect::<Result<BTreeSet<_>, _>>()?;

    for (path, hash_key, expected_sha256) in copied {
        if !listed.contains(path) {
            return Err(format!("{id}: `{path}` is missing from fixture files"));
        }
        require_exact_string(application, hash_key, expected_sha256, id)?;
        require_sha256(application, hash_key, id)?;
        let bytes =
            fs::read(fixture_root.join(path)).map_err(|error| format!("{id}: failed to read {path}: {error}"))?;
        let actual = format!("{:x}", Sha256::digest(bytes));
        if actual != *expected_sha256 {
            return Err(format!(
                "{id}: {path} hash mismatch; expected {expected_sha256}, found {actual}"
            ));
        }
    }
    Ok(())
}

fn reject_dependency_named_boxferry(table: &Table, context: &str) -> Result<(), String> {
    for (key, value) in table {
        let child_context = format!("{context}.{key}");
        if key == "dependencies" || key.ends_with("-dependencies") {
            let dependencies = value
                .as_table()
                .ok_or_else(|| format!("{child_context} must be a dependency table"))?;
            for (alias, specification) in dependencies {
                let package = specification
                    .as_table()
                    .and_then(|table| table.get("package"))
                    .and_then(Value::as_str);
                if alias == "boxferry" || package == Some("boxferry") {
                    return Err(format!(
                        "ComposeLens must not depend on BoxFerry ({child_context}.{alias})"
                    ));
                }
            }
        }
        if let Some(child) = value.as_table() {
            reject_dependency_named_boxferry(child, &child_context)?;
        }
    }
    Ok(())
}

fn require_record_path<'a>(table: &'a Table, context: &str) -> Result<&'a str, String> {
    let path = required_string(table, "record", context)?;
    if !is_safe_relative(path) || !path.starts_with(&format!("{EVIDENCE_RECORD_DIRECTORY}/")) {
        return Err(format!("{context}: unsafe or out-of-scope record path `{path}`"));
    }
    if Path::new(path).extension() != Some(OsStr::new("toml")) {
        return Err(format!("{context}: record path must end in .toml"));
    }
    Ok(path)
}

fn list_toml_files(directory: &str) -> Result<BTreeSet<String>, String> {
    let absolute = repository_root().join(directory);
    let mut records = BTreeSet::new();
    for entry in fs::read_dir(&absolute).map_err(|error| format!("failed to read {}: {error}", absolute.display()))? {
        let entry = entry.map_err(|error| format!("failed to read record entry: {error}"))?;
        let file_type = entry
            .file_type()
            .map_err(|error| format!("failed to inspect {}: {error}", entry.path().display()))?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| "application evidence has non-UTF-8 filename".to_owned())?;
        if file_type.is_dir() && name == "provider-config" {
            continue;
        }
        if !file_type.is_file() {
            return Err(format!(
                "application evidence directory contains unexpected entry {}",
                entry.path().display()
            ));
        }
        if entry.path().extension() != Some(OsStr::new("toml")) {
            return Err(format!("application evidence directory contains non-TOML file {name}"));
        }
        records.insert(format!("{directory}/{name}"));
    }
    Ok(records)
}

fn parse_repository_table(path: &str) -> Result<Table, String> {
    if !is_safe_relative(path) {
        return Err(format!("unsafe repository path `{path}`"));
    }
    parse_table(path, &read_repository_file(path)?)
}

fn parse_table(path: &str, source: &str) -> Result<Table, String> {
    source
        .parse::<Table>()
        .map_err(|error| format!("invalid {path}: {error}"))
}

fn required_table<'a>(table: &'a Table, field: &str, context: &str) -> Result<&'a Table, String> {
    table
        .get(field)
        .and_then(Value::as_table)
        .ok_or_else(|| format!("{context}: `{field}` must be a table"))
}

fn required_table_array<'a>(table: &'a Table, field: &str, context: &str) -> Result<Vec<&'a Table>, String> {
    table
        .get(field)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{context}: `{field}` must be an array of tables"))?
        .iter()
        .map(|value| {
            value
                .as_table()
                .ok_or_else(|| format!("{context}: `{field}` entries must be tables"))
        })
        .collect()
}

fn required_string<'a>(table: &'a Table, field: &str, context: &str) -> Result<&'a str, String> {
    table
        .get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("{context}: `{field}` must be a non-empty string"))
}

fn require_nonempty_string(table: &Table, field: &str, context: &str) -> Result<(), String> {
    required_string(table, field, context).map(|_| ())
}

fn require_exact_string(table: &Table, field: &str, expected: &str, context: &str) -> Result<(), String> {
    let actual = required_string(table, field, context)?;
    if actual != expected {
        return Err(format!("{context}: `{field}` must be {expected:?}, found {actual:?}"));
    }
    Ok(())
}

fn require_one_of(table: &Table, field: &str, expected: &[&str], context: &str) -> Result<(), String> {
    let actual = required_string(table, field, context)?;
    if !expected.contains(&actual) {
        return Err(format!(
            "{context}: `{field}` must be one of {expected:?}, found {actual:?}"
        ));
    }
    Ok(())
}

fn require_exact_integer(table: &Table, field: &str, expected: i64, context: &str) -> Result<(), String> {
    let actual = table.get(field).and_then(Value::as_integer);
    if actual != Some(expected) {
        return Err(format!(
            "{context}: `{field}` must be integer {expected}, found {actual:?}"
        ));
    }
    Ok(())
}

fn require_exact_bool(table: &Table, field: &str, expected: bool, context: &str) -> Result<(), String> {
    let actual = table.get(field).and_then(Value::as_bool);
    if actual != Some(expected) {
        return Err(format!("{context}: `{field}` must be {expected}, found {actual:?}"));
    }
    Ok(())
}

fn require_nonempty_string_array(table: &Table, field: &str, context: &str) -> Result<(), String> {
    let values = table
        .get(field)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{context}: `{field}` must be an array"))?;
    if values.is_empty() || values.iter().any(|value| value.as_str().is_none_or(str::is_empty)) {
        return Err(format!("{context}: `{field}` must contain non-empty strings"));
    }
    Ok(())
}

fn require_exact_keys(table: &Table, expected: &[&str], context: &str) -> Result<(), String> {
    let actual = table.keys().map(String::as_str).collect::<BTreeSet<_>>();
    let expected = expected.iter().copied().collect::<BTreeSet<_>>();
    if actual != expected {
        return Err(format!(
            "{context}: fields differ; expected {expected:?}, found {actual:?}"
        ));
    }
    Ok(())
}

fn require_sha256(table: &Table, field: &str, context: &str) -> Result<(), String> {
    let value = required_string(table, field, context)?;
    if !is_lower_hex(value, 64) {
        return Err(format!("{context}: `{field}` must be a 64-character lowercase SHA-256"));
    }
    Ok(())
}

fn require_commit(table: &Table, field: &str, context: &str) -> Result<(), String> {
    let value = required_string(table, field, context)?;
    if !is_lower_hex(value, 40) {
        return Err(format!(
            "{context}: `{field}` must be a 40-character lowercase commit ID"
        ));
    }
    Ok(())
}

fn require_date(table: &Table, field: &str, context: &str) -> Result<(), String> {
    let value = required_string(table, field, context)?;
    let bytes = value.as_bytes();
    if bytes.len() != 10
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes
            .iter()
            .enumerate()
            .any(|(index, byte)| index != 4 && index != 7 && !byte.is_ascii_digit())
    {
        return Err(format!("{context}: `{field}` must be an ISO date"));
    }
    Ok(())
}

fn require_timestamp(table: &Table, field: &str, context: &str) -> Result<(), String> {
    let value = required_string(table, field, context)?;
    if value.len() != 20 || value.as_bytes().get(10) != Some(&b'T') || !value.ends_with('Z') {
        return Err(format!("{context}: `{field}` must be a second-precision UTC timestamp"));
    }
    Ok(())
}

fn require_version(table: &Table, field: &str, context: &str) -> Result<(), String> {
    let value = required_string(table, field, context)?;
    if value.split('.').count() < 3
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || byte == b'.' || byte == b'-')
    {
        return Err(format!("{context}: `{field}` is not an exact runtime version"));
    }
    Ok(())
}

fn is_lower_hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn is_safe_relative(path: &str) -> bool {
    !path.is_empty()
        && !Path::new(path).is_absolute()
        && Path::new(path)
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

fn read_repository_file(path: &str) -> Result<String, String> {
    let absolute = repository_root().join(path);
    fs::read_to_string(&absolute).map_err(|error| format!("failed to read {}: {error}", absolute.display()))
}

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}
