//! Repository-owned, explicitly invoked Compose-provider conformance harness.

use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::error::Error;
use std::ffi::OsString;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

use compose_lens::validation::ImplementationVersion;
use compose_lens::{
    model::{ComposeDocument, SECURITY_OPT_UNMASK_NEAR_MISS, SecurityOptionKind},
    source::SourceId,
    syntax::SyntaxDocument,
};
use sha2::{Digest, Sha256};
use toml::{Table, Value};

const MATRIX_TEXT: &str = include_str!("../conformance/provider-config-matrix.toml");
const SECURITY_OPTIONS_FIXTURE: &str = include_str!("../fixtures/conformance/service-security-options/compose.yaml");

#[derive(Debug)]
struct Matrix {
    targets: BTreeMap<String, Target>,
    probes: BTreeMap<String, Probe>,
    runs: BTreeMap<(String, String), Run>,
}

#[derive(Debug)]
struct Target {
    id: String,
    provider: String,
    version: String,
    release_url: String,
    artifact_url: String,
    artifact_sha256: String,
    execution: Vec<String>,
    version_arguments: Vec<String>,
    file_argument: String,
    config_arguments: Vec<String>,
}

#[derive(Debug)]
struct Probe {
    id: String,
    fixture: String,
    files: Vec<String>,
    fixture_sha256: String,
}

#[derive(Debug)]
struct Run {
    status: String,
    record: Option<String>,
}

#[derive(Debug)]
struct InvocationContext {
    launcher: PathBuf,
    launcher_sha256: String,
    platform: String,
    command_path: String,
    working_directory: PathBuf,
    home: PathBuf,
    config: PathBuf,
    cache: PathBuf,
    runtime: PathBuf,
}

#[test]
fn provider_config_matrix_is_exact_complete_and_reproducible() -> Result<(), String> {
    let root = repository_root();
    let matrix = parse_matrix(MATRIX_TEXT, &root)?;

    let expected_runs = matrix.targets.len() * matrix.probes.len();
    if matrix.runs.len() != expected_runs {
        return Err(format!(
            "matrix contains {} runs but the {} targets and {} probes require {expected_runs}",
            matrix.runs.len(),
            matrix.targets.len(),
            matrix.probes.len()
        ));
    }

    for target in matrix.targets.keys() {
        for probe in matrix.probes.keys() {
            if !matrix.runs.contains_key(&(target.clone(), probe.clone())) {
                return Err(format!("matrix is missing run `{target}` / `{probe}`"));
            }
        }
    }

    assert!(matrix.targets.contains_key("docker-compose-2-24-3"));
    assert!(matrix.targets.contains_key("docker-compose-2-24-4"));
    assert!(matrix.targets.contains_key("docker-compose-2-40-3"));
    assert!(matrix.targets.contains_key("docker-compose-5-3-1"));
    assert!(matrix.targets.contains_key("podman-compose-1-3-0"));
    assert!(matrix.targets.contains_key("podman-compose-1-5-0"));
    assert!(matrix.runs.values().all(|run| match run.status.as_str() {
        "planned" => run.record.is_none(),
        "observed" => run.record.is_some(),
        _ => false,
    }));
    Ok(())
}

#[test]
fn security_options_probe_retains_exact_unmask_candidates_and_near_misses() -> Result<(), Box<dyn Error>> {
    let syntax = SyntaxDocument::parse(SourceId::new(739), SECURITY_OPTIONS_FIXTURE)?;
    let parsed = ComposeDocument::parse(syntax.document());
    let options = parsed
        .document()
        .and_then(|document| document.service("unmask"))
        .and_then(compose_lens::model::Service::security_options)
        .ok_or("unmask conformance service expected")?;

    assert_eq!(options.items().len(), 22);
    for (index, expected) in [
        (0, "ALL"),
        (1, "ALL"),
        (2, "/proc/acpi"),
        (3, "/proc/acpi:/sys/firmware"),
        (4, "/proc/*"),
    ] {
        assert!(matches!(
            options.items()[index].kind(),
            SecurityOptionKind::Unmask { paths } if paths == expected
        ));
    }
    assert!(
        options.items()[5..]
            .iter()
            .all(|item| matches!(item.kind(), SecurityOptionKind::UnmaskNearMiss))
    );
    assert_eq!(
        parsed
            .diagnostics()
            .iter()
            .filter(|diagnostic| diagnostic.code() == SECURITY_OPT_UNMASK_NEAR_MISS)
            .count(),
        17
    );
    Ok(())
}

#[test]
fn provider_version_identity_is_not_a_substring_match() {
    assert!(reports_exact_version(b"5.3.1\n", "5.3.1"));
    assert!(reports_exact_version(b"v5.3.1\r\n", "5.3.1"));
    assert!(!reports_exact_version(b"15.3.1\n", "5.3.1"));
    assert!(!reports_exact_version(b"5.3.1-dev\n", "5.3.1"));
    assert!(!reports_exact_version(&[0xff], "5.3.1"));
}

/// Executes one exact provider/config pair and writes an unreviewed result outside the repository.
///
/// This test is ignored so ordinary tests remain pure. See `conformance/README.md` for the required
/// environment and review workflow.
#[test]
#[ignore = "requires an explicitly acquired provider binary and an isolated result directory"]
fn run_selected_provider_config_probe() -> Result<(), Box<dyn Error>> {
    let root = repository_root();
    let matrix = parse_matrix(MATRIX_TEXT, &root)?;
    let target_id = required_environment("COMPOSE_LENS_CONFORMANCE_TARGET")?;
    let probe_id = required_environment("COMPOSE_LENS_CONFORMANCE_PROBE")?;
    let target = matrix
        .targets
        .get(&target_id)
        .ok_or("selected target is not in the matrix")?;
    let probe = matrix
        .probes
        .get(&probe_id)
        .ok_or("selected probe is not in the matrix")?;
    let run_key = (target_id.clone(), probe_id.clone());
    let _run = matrix
        .runs
        .get(&run_key)
        .ok_or("selected target/probe run is not in the matrix")?;

    let launcher = absolute_environment_path("COMPOSE_LENS_CONFORMANCE_LAUNCHER")?;
    if !launcher.is_file() {
        return Err("the conformance launcher must be an existing regular file".into());
    }
    let result_directory = absolute_environment_path("COMPOSE_LENS_CONFORMANCE_RESULT_DIRECTORY")?;
    if result_directory.exists() {
        return Err("the conformance result directory must not already exist".into());
    }
    let expected_launcher_sha256 = required_environment("COMPOSE_LENS_CONFORMANCE_LAUNCHER_SHA256")?;
    if !is_sha256(&expected_launcher_sha256) {
        return Err("the conformance launcher SHA-256 must be exactly 64 lowercase hexadecimal digits".into());
    }
    let launcher_sha256 = sha256_file(&launcher)?;
    if launcher_sha256 != expected_launcher_sha256 {
        return Err("the conformance launcher does not match its caller-supplied SHA-256".into());
    }
    let fixture_directory = root.join(&probe.fixture);
    if fixture_sha256(&fixture_directory, &probe.files)? != probe.fixture_sha256 {
        return Err("the conformance fixture bytes do not match the matrix SHA-256".into());
    }
    let platform = required_environment("COMPOSE_LENS_CONFORMANCE_PLATFORM")?;
    let command_path = required_environment("COMPOSE_LENS_CONFORMANCE_PATH")?;

    fs::create_dir(&result_directory)?;
    let isolated_home = result_directory.join("home");
    let isolated_config = result_directory.join("config");
    let isolated_cache = result_directory.join("cache");
    let isolated_runtime = result_directory.join("runtime");
    fs::create_dir(&isolated_home)?;
    fs::create_dir(&isolated_config)?;
    fs::create_dir(&isolated_cache)?;
    create_runtime_directory(&isolated_runtime)?;

    let context = InvocationContext {
        launcher,
        launcher_sha256,
        platform,
        command_path,
        working_directory: fixture_directory,
        home: isolated_home,
        config: isolated_config,
        cache: isolated_cache,
        runtime: isolated_runtime,
    };
    let version_output = invoke(&context, &target.version_arguments)?;
    if !version_output.status.success() || !reports_exact_version(&version_output.stdout, &target.version) {
        return Err("provider version command failed or did not report the exact matrix version".into());
    }

    let probe_arguments = probe_arguments(target, probe);
    let probe_output = invoke(&context, &probe_arguments)?;

    fs::write(result_directory.join("version.stdout"), &version_output.stdout)?;
    fs::write(result_directory.join("version.stderr"), &version_output.stderr)?;
    fs::write(result_directory.join("probe.stdout"), &probe_output.stdout)?;
    fs::write(result_directory.join("probe.stderr"), &probe_output.stderr)?;

    let record = result_record(
        target,
        probe,
        &context,
        &probe_arguments,
        &version_output,
        &probe_output,
    )?;
    fs::write(result_directory.join("record.toml"), toml::to_string_pretty(&record)?)?;
    Ok(())
}

fn parse_matrix(text: &str, repository_root: &Path) -> Result<Matrix, String> {
    let root = text
        .parse::<Table>()
        .map_err(|error| format!("invalid conformance matrix TOML: {error}"))?;
    exact_fields(&root, &["schema", "reviewed", "targets", "probes", "runs"], "matrix")?;
    if root.get("schema").and_then(Value::as_integer) != Some(1) {
        return Err("matrix `schema` must be integer 1".to_owned());
    }
    let reviewed = required_string(&root, "reviewed", "matrix")?;
    if !is_iso_date(reviewed) {
        return Err("matrix `reviewed` must be an exact YYYY-MM-DD date".to_owned());
    }

    let targets = parse_targets(&root)?;
    let probes = parse_probes(&root, repository_root)?;
    let runs = parse_runs(&root, repository_root, &targets, &probes)?;

    Ok(Matrix { targets, probes, runs })
}

fn parse_targets(root: &Table) -> Result<BTreeMap<String, Target>, String> {
    let mut targets = BTreeMap::new();
    for table in required_table_array(root, "targets", "matrix")? {
        exact_fields(
            table,
            &[
                "id",
                "provider",
                "version",
                "release_url",
                "artifact_url",
                "artifact_sha256",
                "execution",
                "version_arguments",
                "file_argument",
                "config_arguments",
                "purpose",
            ],
            "target",
        )?;
        let target = Target {
            id: required_slug(table, "id", "target")?.to_owned(),
            provider: required_string(table, "provider", "target")?.to_owned(),
            version: required_string(table, "version", "target")?.to_owned(),
            release_url: required_string(table, "release_url", "target")?.to_owned(),
            artifact_url: required_string(table, "artifact_url", "target")?.to_owned(),
            artifact_sha256: required_string(table, "artifact_sha256", "target")?.to_owned(),
            execution: required_strings(table, "execution", "target")?,
            version_arguments: required_strings(table, "version_arguments", "target")?,
            file_argument: required_string(table, "file_argument", "target")?.to_owned(),
            config_arguments: required_strings(table, "config_arguments", "target")?,
        };
        let _purpose = required_string(table, "purpose", "target")?;
        let _: ImplementationVersion = target
            .version
            .parse()
            .map_err(|_| format!("target `{}` must use an exact three-component version", target.id))?;
        let expected_release = match target.provider.as_str() {
            "docker-compose" => format!("https://github.com/docker/compose/releases/tag/v{}", target.version),
            "podman-compose" => format!(
                "https://github.com/containers/podman-compose/releases/tag/v{}",
                target.version
            ),
            _ => {
                return Err(format!(
                    "target `{}` has unknown provider `{}`",
                    target.id, target.provider
                ));
            }
        };
        let expected_id = format!("{}-{}", target.provider, target.version.replace('.', "-"));
        if target.id != expected_id {
            return Err(format!(
                "target `{}` must encode provider and exact version as `{expected_id}`",
                target.id
            ));
        }
        if target.release_url != expected_release {
            return Err(format!("target `{}` must link its exact immutable release", target.id));
        }
        if !is_https_url(&target.artifact_url) || !is_sha256(&target.artifact_sha256) {
            return Err(format!(
                "target `{}` must define an immutable HTTPS artifact and SHA-256",
                target.id
            ));
        }
        if target.execution.is_empty() {
            return Err(format!("target `{}` must define its execution environment", target.id));
        }
        if target.version_arguments.is_empty() || target.config_arguments.is_empty() || target.file_argument.is_empty()
        {
            return Err(format!(
                "target `{}` must define non-empty invocation arguments",
                target.id
            ));
        }
        if target.provider == "podman-compose"
            && (!target.version_arguments.iter().any(|argument| argument == "--dry-run")
                || !target.config_arguments.iter().any(|argument| argument == "--dry-run"))
        {
            return Err(format!(
                "target `{}` must keep provider-only probes independent from the Podman runtime",
                target.id
            ));
        }
        let id = target.id.clone();
        if targets.insert(id.clone(), target).is_some() {
            return Err(format!("duplicate conformance target `{id}`"));
        }
    }
    if targets.is_empty() {
        return Err("matrix must contain at least one target".to_owned());
    }
    Ok(targets)
}

fn parse_probes(root: &Table, repository_root: &Path) -> Result<BTreeMap<String, Probe>, String> {
    let mut probes = BTreeMap::new();
    for table in required_table_array(root, "probes", "matrix")? {
        exact_fields(table, &["id", "fixture", "files", "fixture_sha256", "purpose"], "probe")?;
        let probe = Probe {
            id: required_slug(table, "id", "probe")?.to_owned(),
            fixture: required_string(table, "fixture", "probe")?.to_owned(),
            files: required_strings(table, "files", "probe")?,
            fixture_sha256: required_string(table, "fixture_sha256", "probe")?.to_owned(),
        };
        let _purpose = required_string(table, "purpose", "probe")?;
        if !is_safe_relative_path(&probe.fixture) || probe.files.is_empty() {
            return Err(format!("probe `{}` must use safe, non-empty fixture paths", probe.id));
        }
        if !is_sha256(&probe.fixture_sha256) {
            return Err(format!("probe `{}` must define its fixture SHA-256", probe.id));
        }
        let fixture_directory = repository_root.join(&probe.fixture);
        if !fixture_directory.join("fixture.toml").is_file() {
            return Err(format!("probe `{}` fixture manifest does not exist", probe.id));
        }
        let mut unique_files = BTreeSet::new();
        for file in &probe.files {
            if !is_safe_relative_path(file) || !unique_files.insert(file) {
                return Err(format!(
                    "probe `{}` contains an unsafe or duplicate file path",
                    probe.id
                ));
            }
            if !fixture_directory.join(file).is_file() {
                return Err(format!("probe `{}` references missing file `{file}`", probe.id));
            }
        }
        let actual_fixture_sha256 = fixture_sha256(&fixture_directory, &probe.files)
            .map_err(|error| format!("probe `{}` fixture hash failed: {error}", probe.id))?;
        if actual_fixture_sha256 != probe.fixture_sha256 {
            return Err(format!("probe `{}` fixture SHA-256 does not match its files", probe.id));
        }
        let id = probe.id.clone();
        if probes.insert(id.clone(), probe).is_some() {
            return Err(format!("duplicate conformance probe `{id}`"));
        }
    }
    if probes.is_empty() {
        return Err("matrix must contain at least one probe".to_owned());
    }
    Ok(probes)
}

fn parse_runs(
    root: &Table,
    repository_root: &Path,
    targets: &BTreeMap<String, Target>,
    probes: &BTreeMap<String, Probe>,
) -> Result<BTreeMap<(String, String), Run>, String> {
    let mut runs = BTreeMap::new();
    for table in required_table_array(root, "runs", "matrix")? {
        exact_fields(table, &["target", "probe", "status", "record"], "run")?;
        let target = required_string(table, "target", "run")?.to_owned();
        let probe = required_string(table, "probe", "run")?.to_owned();
        if !targets.contains_key(&target) || !probes.contains_key(&probe) {
            return Err(format!("run `{target}` / `{probe}` references an unknown matrix entry"));
        }
        let status = required_string(table, "status", "run")?.to_owned();
        let record = table.get("record").and_then(Value::as_str).map(str::to_owned);
        match (status.as_str(), record.as_deref()) {
            ("planned", None) => {}
            ("observed", Some(path)) if is_safe_relative_path(path) && repository_root.join(path).is_file() => {
                validate_reviewed_record(repository_root, path, &target, &probe, targets, probes)?;
            }
            ("planned", Some(_)) => return Err("planned conformance runs must not cite evidence records".to_owned()),
            ("observed", Some(_)) => {
                return Err("observed conformance runs must cite an existing safe record".to_owned());
            }
            ("observed", None) => return Err("observed conformance runs require a retained record".to_owned()),
            _ => return Err(format!("run `{target}` / `{probe}` has unknown status `{status}`")),
        }
        let key = (target, probe);
        if runs.insert(key.clone(), Run { status, record }).is_some() {
            return Err(format!("duplicate conformance run `{}` / `{}`", key.0, key.1));
        }
    }
    Ok(runs)
}

fn probe_arguments(target: &Target, probe: &Probe) -> Vec<String> {
    let mut arguments = Vec::new();
    for file in &probe.files {
        arguments.push(target.file_argument.clone());
        arguments.push(file.clone());
    }
    arguments.extend(target.config_arguments.iter().cloned());
    arguments
}

fn invoke(context: &InvocationContext, arguments: &[String]) -> Result<Output, std::io::Error> {
    let mut command = Command::new(&context.launcher);
    command
        .args(arguments)
        .current_dir(&context.working_directory)
        .env_clear()
        .env("HOME", &context.home)
        .env("XDG_CONFIG_HOME", &context.config)
        .env("XDG_CACHE_HOME", &context.cache)
        .env("XDG_RUNTIME_DIR", &context.runtime)
        .env("PATH", &context.command_path)
        .env("LANG", "C")
        .env("LC_ALL", "C");
    command.output()
}

fn result_record(
    target: &Target,
    probe: &Probe,
    context: &InvocationContext,
    probe_arguments: &[String],
    version_output: &Output,
    probe_output: &Output,
) -> Result<Table, Box<dyn Error>> {
    let mut record = Table::new();
    record.insert("schema".to_owned(), Value::Integer(1));
    record.insert("review_status".to_owned(), Value::String("unreviewed".to_owned()));
    record.insert(
        "observed_unix_seconds".to_owned(),
        Value::Integer(i64::try_from(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs())?),
    );
    record.insert("target".to_owned(), Value::String(target.id.clone()));
    record.insert("probe".to_owned(), Value::String(probe.id.clone()));
    record.insert("provider".to_owned(), Value::String(target.provider.clone()));
    record.insert("provider_version".to_owned(), Value::String(target.version.clone()));
    record.insert("release_url".to_owned(), Value::String(target.release_url.clone()));
    record.insert(
        "launcher_file".to_owned(),
        Value::String(
            context
                .launcher
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or("launcher must have a UTF-8 file name")?
                .to_owned(),
        ),
    );
    record.insert("artifact_url".to_owned(), Value::String(target.artifact_url.clone()));
    record.insert(
        "artifact_sha256".to_owned(),
        Value::String(target.artifact_sha256.clone()),
    );
    record.insert(
        "launcher_sha256".to_owned(),
        Value::String(context.launcher_sha256.clone()),
    );
    record.insert("fixture_sha256".to_owned(), Value::String(probe.fixture_sha256.clone()));
    record.insert(
        "execution".to_owned(),
        Value::Array(target.execution.iter().cloned().map(Value::String).collect()),
    );
    record.insert("platform".to_owned(), Value::String(context.platform.clone()));
    record.insert("working_directory".to_owned(), Value::String(probe.fixture.clone()));
    record.insert("fixture".to_owned(), Value::String(probe.fixture.clone()));
    record.insert(
        "fixture_files".to_owned(),
        Value::Array(probe.files.iter().cloned().map(Value::String).collect()),
    );
    record.insert(
        "version_arguments".to_owned(),
        Value::Array(target.version_arguments.iter().cloned().map(Value::String).collect()),
    );
    record.insert(
        "probe_arguments".to_owned(),
        Value::Array(probe_arguments.iter().cloned().map(Value::String).collect()),
    );
    record.insert("environment".to_owned(), Value::Table(environment_record(context)));
    insert_status(&mut record, "version", version_output);
    insert_status(&mut record, "probe", probe_output);
    Ok(record)
}

fn environment_record(context: &InvocationContext) -> Table {
    let mut environment = Table::new();
    environment.insert("inherited".to_owned(), Value::Boolean(false));
    environment.insert("HOME".to_owned(), Value::String("<result>/home".to_owned()));
    environment.insert(
        "XDG_CONFIG_HOME".to_owned(),
        Value::String("<result>/config".to_owned()),
    );
    environment.insert("XDG_CACHE_HOME".to_owned(), Value::String("<result>/cache".to_owned()));
    environment.insert(
        "XDG_RUNTIME_DIR".to_owned(),
        Value::String("<result>/runtime".to_owned()),
    );
    environment.insert("PATH".to_owned(), Value::String(context.command_path.clone()));
    environment.insert("LANG".to_owned(), Value::String("C".to_owned()));
    environment.insert("LC_ALL".to_owned(), Value::String("C".to_owned()));
    environment
}

fn create_runtime_directory(path: &Path) -> Result<(), std::io::Error> {
    fs::create_dir(path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}

fn insert_status(record: &mut Table, prefix: &str, output: &Output) {
    record.insert(format!("{prefix}_success"), Value::Boolean(output.status.success()));
    if let Some(code) = output.status.code() {
        record.insert(format!("{prefix}_exit_code"), Value::Integer(i64::from(code)));
    }
}

fn required_environment(name: &str) -> Result<String, Box<dyn Error>> {
    let value = env::var(name).map_err(|_| format!("required environment variable `{name}` is missing"))?;
    if value.is_empty() {
        return Err(format!("required environment variable `{name}` is empty").into());
    }
    Ok(value)
}

fn absolute_environment_path(name: &str) -> Result<PathBuf, Box<dyn Error>> {
    let value: OsString =
        env::var_os(name).ok_or_else(|| format!("required environment variable `{name}` is missing"))?;
    let path = PathBuf::from(value);
    if !path.is_absolute() {
        return Err(format!("environment variable `{name}` must be an absolute path").into());
    }
    Ok(path)
}

fn required_table_array<'a>(table: &'a Table, field: &str, context: &str) -> Result<Vec<&'a Table>, String> {
    let values = table
        .get(field)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{context} `{field}` must be an array of tables"))?;
    values
        .iter()
        .map(|value| {
            value
                .as_table()
                .ok_or_else(|| format!("{context} `{field}` must contain only tables"))
        })
        .collect()
}

fn required_string<'a>(table: &'a Table, field: &str, context: &str) -> Result<&'a str, String> {
    table
        .get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("{context} `{field}` must be a non-empty string"))
}

fn required_slug<'a>(table: &'a Table, field: &str, context: &str) -> Result<&'a str, String> {
    let value = required_string(table, field, context)?;
    if value
        .bytes()
        .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    {
        Ok(value)
    } else {
        Err(format!("{context} `{field}` must be a lowercase ASCII slug"))
    }
}

fn required_strings(table: &Table, field: &str, context: &str) -> Result<Vec<String>, String> {
    let values = table
        .get(field)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{context} `{field}` must be an array of strings"))?;
    values
        .iter()
        .map(|value| {
            value
                .as_str()
                .filter(|item| !item.is_empty())
                .map(str::to_owned)
                .ok_or_else(|| format!("{context} `{field}` must contain non-empty strings"))
        })
        .collect()
}

fn exact_fields(table: &Table, fields: &[&str], context: &str) -> Result<(), String> {
    for field in table.keys() {
        if !fields.contains(&field.as_str()) {
            return Err(format!("{context} contains unknown field `{field}`"));
        }
    }
    Ok(())
}

fn is_iso_date(value: &str) -> bool {
    value.len() == 10
        && value.bytes().enumerate().all(|(index, byte)| {
            if index == 4 || index == 7 {
                byte == b'-'
            } else {
                byte.is_ascii_digit()
            }
        })
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn is_https_url(value: &str) -> bool {
    value.starts_with("https://")
        && value.len() > "https://".len()
        && !value.bytes().any(|byte| byte.is_ascii_whitespace())
}

fn sha256_file(path: &Path) -> Result<String, std::io::Error> {
    let bytes = fs::read(path)?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn fixture_sha256(directory: &Path, files: &[String]) -> Result<String, std::io::Error> {
    let mut digest = Sha256::new();
    for file in files {
        digest.update(file.as_bytes());
        digest.update([0]);
        digest.update(fs::read(directory.join(file))?);
        digest.update([0]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

fn validate_reviewed_record(
    repository_root: &Path,
    path: &str,
    target_id: &str,
    probe_id: &str,
    targets: &BTreeMap<String, Target>,
    probes: &BTreeMap<String, Probe>,
) -> Result<(), String> {
    let target = targets.get(target_id).ok_or("record target is missing")?;
    let probe = probes.get(probe_id).ok_or("record probe is missing")?;
    let record_path = repository_root.join(path);
    let text = fs::read_to_string(&record_path)
        .map_err(|error| format!("failed to read reviewed record `{path}`: {error}"))?;
    let record = text
        .parse::<Table>()
        .map_err(|error| format!("reviewed record `{path}` is invalid TOML: {error}"))?;
    for (field, expected) in [
        ("review_status", "reviewed"),
        ("target", target_id),
        ("probe", probe_id),
        ("provider_version", target.version.as_str()),
        ("artifact_sha256", target.artifact_sha256.as_str()),
        ("fixture_sha256", probe.fixture_sha256.as_str()),
    ] {
        if record.get(field).and_then(Value::as_str) != Some(expected) {
            return Err(format!("reviewed record `{path}` has incorrect `{field}`"));
        }
    }
    let directory = record_path.parent().ok_or("reviewed record path has no parent")?;
    for output in ["version.stdout", "version.stderr", "probe.stdout", "probe.stderr"] {
        if !directory.join(output).is_file() {
            return Err(format!("reviewed record `{path}` is missing `{output}`"));
        }
    }
    Ok(())
}

fn reports_exact_version(stdout: &[u8], expected: &str) -> bool {
    let Ok(stdout) = std::str::from_utf8(stdout) else {
        return false;
    };
    let reported = stdout.trim();
    reported == expected || reported.strip_prefix('v') == Some(expected)
}

fn is_safe_relative_path(value: &str) -> bool {
    let path = Path::new(value);
    !path.as_os_str().is_empty()
        && !path.is_absolute()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
